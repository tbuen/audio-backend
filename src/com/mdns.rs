use local_ip_address::local_ip;
use log::{debug, info};
use mdns_sd::{Receiver as MdnsReceiver, ServiceDaemon, ServiceEvent};
use std::net::SocketAddrV4;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};

const SERVICE_TYPE: &str = "_audio-jsonrpc-websocket._tcp.local.";

enum Command {
    Quit,
}

pub(crate) struct Mdns {
    handle: Option<JoinHandle<()>>,
    sender: Sender<Command>,
    receiver: Receiver<SocketAddrV4>,
}

impl Mdns {
    pub(crate) fn new() -> Self {
        let (sender, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        Self {
            handle: Some(
                Builder::new()
                    .name("audio:mdns".into())
                    .spawn(move || Self::thread(tx, rx))
                    .unwrap(),
            ),
            sender,
            receiver,
        }
    }

    pub(crate) fn recv_timeout(&self, dur: Duration) -> Result<SocketAddrV4, RecvTimeoutError> {
        self.receiver.recv_timeout(dur)
    }

    fn thread(tx: Sender<SocketAddrV4>, rx: Receiver<Command>) {
        let mut daemon = None;
        let mut receiver = None;
        let mut ip_addr = None;
        let mut ip_time = Instant::now();

        debug!("mDNS thread started");

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Quit) => {
                    debug!("mDNS thread received Quit");
                    Self::stop(&mut daemon, &mut receiver);
                    break;
                }
                Err(_) => {}
            }

            if ip_time.elapsed() >= Duration::from_secs(1) {
                match local_ip() {
                    Ok(ip) => {
                        if let Some(old_ip) = ip_addr {
                            if ip != old_ip {
                                info!("New IP: {ip:?}");
                                ip_addr = Some(ip);
                                Self::stop(&mut daemon, &mut receiver);
                                Self::start(&mut daemon, &mut receiver);
                            }
                        } else {
                            info!("New IP: {ip:?}");
                            ip_addr = Some(ip);
                            Self::start(&mut daemon, &mut receiver);
                        }
                    }
                    Err(_) => {
                        if ip_addr.take().is_some() {
                            info!("Lost IP!");
                            Self::stop(&mut daemon, &mut receiver);
                        }
                    }
                }
                ip_time = Instant::now();
            }

            if let Some(r) = &receiver {
                match r.recv_timeout(Duration::from_millis(10)) {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        if let Some(addr) = info.get_addresses_v4().iter().next() {
                            debug!(
                                "mDNS service resolved: {} {} {}:{}",
                                info.get_fullname(),
                                info.get_hostname(),
                                addr,
                                info.get_port(),
                            );
                            tx.send(SocketAddrV4::new(*addr, info.get_port())).unwrap();
                        }
                    }
                    Ok(event) => debug!("mDNS received event: {event:?}"),
                    Err(_) => {}
                }
            }
        }

        debug!("mDNS thread stopped");
    }

    fn start(
        daemon: &mut Option<ServiceDaemon>,
        receiver: &mut Option<MdnsReceiver<ServiceEvent>>,
    ) {
        if let (None, None) = (&daemon, &receiver) {
            let d = ServiceDaemon::new().unwrap();
            let r = d.browse(SERVICE_TYPE).unwrap();
            *daemon = Some(d);
            *receiver = Some(r);
            debug!("mDNS daemon started");
        }
    }

    fn stop(daemon: &mut Option<ServiceDaemon>, receiver: &mut Option<MdnsReceiver<ServiceEvent>>) {
        if let (Some(d), Some(r)) = (&daemon, &receiver) {
            d.stop_browse(SERVICE_TYPE).unwrap();
            loop {
                if let Ok(ServiceEvent::SearchStopped(_)) = r.recv() {
                    break;
                }
            }
            d.shutdown().unwrap();
            *receiver = None;
            *daemon = None;
            debug!("mDNS daemon stopped");
        }
    }
}

impl Drop for Mdns {
    fn drop(&mut self) {
        self.sender.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
