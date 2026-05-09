use dbus::arg::{PropMap, Variant};
use dbus::blocking::Connection;
use dbus::{Message, Path};
use dbus_codegen::networkmanager::OrgFreedesktopNetworkManager as _;
use dbus_codegen::networkmanager_accesspoint::OrgFreedesktopNetworkManagerAccessPoint as _;
use dbus_codegen::networkmanager_device::{
    OrgFreedesktopNetworkManagerDevice as _, OrgFreedesktopNetworkManagerDeviceWireless as _,
    OrgFreedesktopNetworkManagerDeviceWirelessAccessPointAdded,
};
use dbus_codegen::networkmanager_settings::OrgFreedesktopNetworkManagerSettings as _;
use dbus_codegen::networkmanager_settings_connection::OrgFreedesktopNetworkManagerSettingsConnection;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

mod dbus_codegen;

const NM_DEVICE_TYPE_WIFI: u32 = 2;

enum Cmd {
    AccessPointAdded(Path<'static>),
    Quit,
}

pub(crate) struct Connector {
    handle: Option<thread::JoinHandle<()>>,
    sender: Sender<Cmd>,
}

impl Connector {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let sender_thread = sender.clone();
        let handle = thread::Builder::new()
            .name("audio:access_point".into())
            .spawn(|| Self::thread(sender_thread, receiver))
            .unwrap();
        Self {
            handle: Some(handle),
            sender,
        }
    }

    fn thread(tx: Sender<Cmd>, rx: Receiver<Cmd>) {
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
            connection.insert("id".into(), Variant(Box::new(String::from("esp32-audio"))));
            connection.insert(
                "type".into(),
                Variant(Box::new(String::from("802-11-wireless"))),
            );

            let mut wifi = PropMap::new();
            wifi.insert(
                "ssid".into(),
                Variant(Box::new(Vec::<u8>::from("esp32-audio"))),
            );

            let mut security = PropMap::new();
            security.insert(
                "key-mgmt".into(),
                Variant(Box::new(String::from("wpa-psk"))),
            );
            security.insert(
                "psk".into(),
                Variant(Box::new(String::from("iWFt55J9mzuPslBNbqTVfraR"))),
            );

            let mut settings = HashMap::new();
            settings.insert("connection", connection);
            settings.insert("802-11-wireless", wifi);
            settings.insert("802-11-wireless-security", security);
            proxy_settings.add_connection_unsaved(settings).ok()
        };

        let mut device = None;
        if let Ok(devices) = proxy.get_devices() {
            for d in devices {
                let proxy_device =
                    conn.with_proxy("org.freedesktop.NetworkManager", &d, Duration::from_secs(5));

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
        }

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
                        if proxy_ap.ssid().unwrap() == "esp32-audio".as_bytes()
                            && let (Some(c), Some(d)) = (&connection, &device)
                        {
                            proxy
                                .activate_connection(c.to_owned(), d.to_owned(), ap)
                                .unwrap();
                        }
                    }
                }
            }
            conn.process(Duration::from_millis(10)).unwrap();
        }

        if let Some(c) = connection {
            let proxy_connection =
                conn.with_proxy("org.freedesktop.NetworkManager", c, Duration::from_secs(5));
            OrgFreedesktopNetworkManagerSettingsConnection::delete(&proxy_connection).unwrap();
        }
    }
}

impl Drop for Connector {
    fn drop(&mut self) {
        self.sender.send(Cmd::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
