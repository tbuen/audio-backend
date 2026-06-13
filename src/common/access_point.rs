use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};

use dbus::arg::{self, PropMap, Variant};
use dbus::blocking::Connection;
use dbus::{Message, Path};
use log::error;

use crate::common::dbus_codegen::networkmanager::{
    OrgFreedesktopDBusPropertiesPropertiesChanged, OrgFreedesktopNetworkManager,
};
use crate::common::dbus_codegen::networkmanager_accesspoint::OrgFreedesktopNetworkManagerAccessPoint;
use crate::common::dbus_codegen::networkmanager_connection_active::{
    OrgFreedesktopNetworkManagerConnectionActive,
    OrgFreedesktopNetworkManagerConnectionActiveStateChanged,
};
use crate::common::dbus_codegen::networkmanager_device::{
    OrgFreedesktopNetworkManagerDevice, OrgFreedesktopNetworkManagerDeviceWireless,
};
use crate::common::dbus_codegen::networkmanager_settings::OrgFreedesktopNetworkManagerSettings;
use crate::common::dbus_codegen::networkmanager_settings_connection::OrgFreedesktopNetworkManagerSettingsConnection;

const NM_DEVICE_TYPE_WIFI: u32 = 2;
const SCAN_INTERVAL_SEC: u64 = 10;
const PROXY_TIMEOUT_SEC: u64 = 5;

pub(crate) struct Connector {
    handle: Option<JoinHandle<()>>,
    sender: Sender<Cmd>,
}

enum Cmd {
    ActiveConnectionsChanged(Vec<Path<'static>>),
    ConnectionActive(bool),
    ScanFinished,
    Quit,
}

impl Connector {
    pub(crate) fn new(ssid: String, key: String) -> Self {
        let (sender, receiver) = mpsc::channel();
        let tx = sender.clone();
        Self {
            handle: Some(
                Builder::new()
                    .name("access_point".into())
                    .spawn(move || Self::thread(tx, receiver, &ssid, &key))
                    .unwrap(),
            ),
            sender,
        }
    }

    fn thread(tx: Sender<Cmd>, rx: Receiver<Cmd>, ssid: &str, key: &str) {
        let conn = Connection::new_system().unwrap();

        let proxy = conn.with_proxy(
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            Duration::from_secs(PROXY_TIMEOUT_SEC),
        );

        let txc = tx.clone();
        proxy
            .match_signal(
                move |p: OrgFreedesktopDBusPropertiesPropertiesChanged,
                      _: &Connection,
                      _: &Message| {
                    if let Some(acs) =
                        arg::prop_cast::<Vec<Path>>(&p.changed_properties, "ActiveConnections")
                    {
                        txc.send(Cmd::ActiveConnectionsChanged(acs.clone()))
                            .unwrap();
                    }
                    true
                },
            )
            .unwrap();

        let connection = {
            let proxy_settings = conn.with_proxy(
                "org.freedesktop.NetworkManager",
                "/org/freedesktop/NetworkManager/Settings",
                Duration::from_secs(PROXY_TIMEOUT_SEC),
            );

            let mut connection = PropMap::new();
            connection.insert("id".into(), Variant(Box::new(String::from("ap-connector"))));
            connection.insert(
                "type".into(),
                Variant(Box::new(String::from("802-11-wireless"))),
            );

            let mut wifi = PropMap::new();
            wifi.insert("ssid".into(), Variant(Box::new(Vec::<u8>::from(ssid))));

            let mut security = PropMap::new();
            security.insert(
                "key-mgmt".into(),
                Variant(Box::new(String::from("wpa-psk"))),
            );
            security.insert("psk".into(), Variant(Box::new(String::from(key))));

            let mut settings = HashMap::new();
            settings.insert("connection", connection);
            settings.insert("802-11-wireless", wifi);
            settings.insert("802-11-wireless-security", security);

            match proxy_settings.add_connection_unsaved(settings) {
                Ok(path_connection) => {
                    let proxy_connection = conn.with_proxy(
                        "org.freedesktop.NetworkManager",
                        path_connection.clone(),
                        Duration::from_secs(PROXY_TIMEOUT_SEC),
                    );
                    Some((path_connection, proxy_connection))
                }
                Err(e) => {
                    error!("Could not create connection: {e}");
                    None
                }
            }
        };

        let device = {
            let mut device = None;
            if let Ok(devices) = proxy.get_devices() {
                for path_device in devices {
                    let proxy_device = conn.with_proxy(
                        "org.freedesktop.NetworkManager",
                        path_device.clone(),
                        Duration::from_secs(PROXY_TIMEOUT_SEC),
                    );

                    if proxy_device.device_type().unwrap() == NM_DEVICE_TYPE_WIFI {
                        let txc = tx.clone();
                        proxy_device
                            .match_signal(
                                move |p: OrgFreedesktopDBusPropertiesPropertiesChanged,
                                      _: &Connection,
                                      _: &Message| {
                                    if p.changed_properties.contains_key("LastScan") {
                                        txc.send(Cmd::ScanFinished).unwrap();
                                    }
                                    true
                                },
                            )
                            .unwrap();
                        device = Some((path_device, proxy_device));
                        break;
                    }
                }
            }
            device
        };

        if let (Some((path_connection, _)), Some((path_device, proxy_device))) =
            (&connection, &device)
        {
            let mut last_scan_time = Instant::now()
                .checked_sub(Duration::from_secs(SCAN_INTERVAL_SEC))
                .unwrap_or(Instant::now());
            let mut path_active_connection = None;
            let mut active = false;

            loop {
                if !active && last_scan_time.elapsed() > Duration::from_secs(SCAN_INTERVAL_SEC) {
                    proxy_device.request_scan(HashMap::new()).unwrap();
                    last_scan_time = Instant::now();
                }

                if let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        Cmd::Quit => break,
                        Cmd::ActiveConnectionsChanged(acs) => {
                            for ac in acs {
                                let proxy_ac = conn.with_proxy(
                                    "org.freedesktop.NetworkManager",
                                    ac.clone(),
                                    Duration::from_secs(PROXY_TIMEOUT_SEC),
                                );
                                if let Ok(c) = &proxy_ac.connection()
                                    && c == path_connection
                                {
                                    if let Some(pac) = &path_active_connection
                                        && pac == &ac
                                    {
                                    } else {
                                        path_active_connection = Some(ac);
                                        let txc = tx.clone();
                                        proxy_ac
                                            .match_signal(
                                                move |s: OrgFreedesktopNetworkManagerConnectionActiveStateChanged,
                                                      _: &Connection,
                                                      _: &Message| {
                                                          txc.send(Cmd::ConnectionActive(s.state <= 2)).unwrap();
                                                    true
                                                },
                                            )
                                            .unwrap();
                                    }
                                }
                            }
                        }
                        Cmd::ConnectionActive(a) => {
                            active = a;
                            if !active {
                                last_scan_time = Instant::now();
                            }
                        }
                        Cmd::ScanFinished => {
                            if !active {
                                last_scan_time = Instant::now();
                                if let Ok(accesspoints) = proxy_device.get_all_access_points() {
                                    for ap in accesspoints {
                                        let proxy_ap = conn.with_proxy(
                                            "org.freedesktop.NetworkManager",
                                            &ap,
                                            Duration::from_secs(PROXY_TIMEOUT_SEC),
                                        );
                                        if proxy_ap.ssid().unwrap() == ssid.as_bytes()
                                            && let Err(e) = proxy.activate_connection(
                                                path_connection.to_owned(),
                                                path_device.to_owned(),
                                                ap,
                                            )
                                        {
                                            error!("Could not activate AP connection: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                conn.process(Duration::from_millis(10)).unwrap();
            }
        }

        if let Some((_, proxy_device)) = &connection
            && let Err(e) = OrgFreedesktopNetworkManagerSettingsConnection::delete(proxy_device)
        {
            error!("Could not delete AP connection: {e}");
        }
    }
}

impl Drop for Connector {
    fn drop(&mut self) {
        self.sender.send(Cmd::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
