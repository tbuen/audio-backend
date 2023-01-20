use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent};
use std::net::SocketAddrV4;

const SERVICE_TYPE: &str = "_audio-jsonrpc-ws._tcp.local.";

pub struct Mdns {
    daemon: ServiceDaemon,
    receiver: Option<Receiver<ServiceEvent>>,
}

impl Mdns {
    pub fn new() -> Self {
        Self {
            daemon: ServiceDaemon::new().unwrap(),
            receiver: None,
        }
    }

    pub fn shutdown(&self) {
        if self.receiver.is_some() {
            self.daemon.stop_browse(SERVICE_TYPE).unwrap();
        }
    }

    pub async fn scan(&mut self) -> Option<SocketAddrV4> {
        let mut socket = None;

        println!("mDNS scan...");

        if self.receiver.is_none() {
            self.receiver = Some(self.daemon.browse(SERVICE_TYPE).unwrap());
        }

        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.recv_async().await {
                    Ok(event) => match event {
                        ServiceEvent::SearchStarted(s) => {
                            println!("mDNS search started: {s}");
                        }
                        ServiceEvent::SearchStopped(s) => {
                            println!("mDNS search stopped: {s}");
                            self.receiver.take();
                            break;
                        }
                        ServiceEvent::ServiceResolved(info) => {
                            if let Some(addr) = info.get_addresses().iter().next() {
                                socket = Some(SocketAddrV4::new(*addr, info.get_port()));
                                println!(
                                    "mDNS service resolved: {} {} {:?}",
                                    info.get_fullname(),
                                    info.get_hostname(),
                                    socket.unwrap()
                                );
                                self.daemon.stop_browse(SERVICE_TYPE).unwrap();
                            }
                        }
                        _ => {}
                    },
                    Err(e) => println!("mDNS receiver error {}", e),
                }
            }
        }

        socket
    }
}

impl Drop for Mdns {
    fn drop(&mut self) {
        self.daemon.shutdown().unwrap();
    }
}
