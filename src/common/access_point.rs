use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{Builder, JoinHandle};
use std::time::Duration;

use super::dbus_codegen::networkmanager::OrgFreedesktopNetworkManager as _;
use super::dbus_codegen::networkmanager_accesspoint::OrgFreedesktopNetworkManagerAccessPoint as _;
use super::dbus_codegen::networkmanager_device::{
    OrgFreedesktopNetworkManagerDevice as _, OrgFreedesktopNetworkManagerDeviceWireless as _,
    OrgFreedesktopNetworkManagerDeviceWirelessAccessPointAdded,
};
use super::dbus_codegen::networkmanager_settings::OrgFreedesktopNetworkManagerSettings as _;
use super::dbus_codegen::networkmanager_settings_connection::OrgFreedesktopNetworkManagerSettingsConnection;
use dbus::arg::{PropMap, Variant};
use dbus::blocking::Connection;
use dbus::{Message, Path};
use log::warn;

const NM_DEVICE_TYPE_WIFI: u32 = 2;

pub(crate) struct Connector {
    handle: Option<JoinHandle<()>>,
    sender: Sender<Cmd>,
}

enum Cmd {
    AccessPointAdded(Path<'static>),
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
            Duration::from_secs(5),
        );

        let connection = {
            let proxy_settings = conn.with_proxy(
                "org.freedesktop.NetworkManager",
                "/org/freedesktop/NetworkManager/Settings",
                Duration::from_secs(5),
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
            proxy_settings.add_connection_unsaved(settings).ok()
        };

        let device = {
            if let Ok(devices) = proxy.get_devices() {
                let mut device = None;
                for d in devices {
                    let proxy_device = conn.with_proxy(
                        "org.freedesktop.NetworkManager",
                        &d,
                        Duration::from_secs(5),
                    );

                    let device_type = proxy_device.device_type().unwrap();
                    if device_type == NM_DEVICE_TYPE_WIFI {
                        let txc = tx.clone();
                        proxy_device
                        .match_signal(move |ap: OrgFreedesktopNetworkManagerDeviceWirelessAccessPointAdded, _: &Connection, _: &Message| {
                            txc.send(Cmd::AccessPointAdded(ap.access_point)).unwrap();
                            true
                        })
                        .unwrap();
                        if let Ok(access_points) = proxy_device.get_all_access_points() {
                            for access_point in access_points {
                                tx.send(Cmd::AccessPointAdded(access_point)).unwrap();
                            }
                        }
                        device = Some(d);
                        break;
                    }
                }
                device
            } else {
                None
            }
        };

        loop {
            if let Ok(cmd) = rx.try_recv() {
                match cmd {
                    Cmd::Quit => break,
                    Cmd::AccessPointAdded(ap) => {
                        let proxy_ap = conn.with_proxy(
                            "org.freedesktop.NetworkManager",
                            &ap,
                            Duration::from_secs(5),
                        );
                        if proxy_ap.ssid().unwrap() == ssid.as_bytes()
                            && let (Some(c), Some(d)) = (&connection, &device)
                            && let Err(e) =
                                proxy.activate_connection(c.to_owned(), d.to_owned(), ap)
                        {
                            warn!("Could not connect to AP: {e}");
                        }
                    }
                }
            }
            conn.process(Duration::from_millis(10)).unwrap();
        }

        if let Some(c) = connection {
            let proxy_connection =
                conn.with_proxy("org.freedesktop.NetworkManager", c, Duration::from_secs(5));
            if let Err(e) =
                OrgFreedesktopNetworkManagerSettingsConnection::delete(&proxy_connection)
            {
                warn!("Could not delete AP connection: {e}");
            }
        }
    }
}

impl Drop for Connector {
    fn drop(&mut self) {
        self.sender.send(Cmd::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
