use local_ip_address::local_ip;
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent};
use std::net::SocketAddrV4;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const SERVICE_TYPE: &str = "_audio-jsonrpc-ws._tcp.local.";

enum Command {
    Quit,
}

pub struct Mdns {
    handle: Option<thread::JoinHandle<()>>,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<SocketAddrV4>,
}

impl Mdns {
    pub fn new() -> Self {
        let (thread_tx, rx) = mpsc::channel();
        let (tx, thread_rx) = mpsc::channel();
        Self {
            handle: Some(
                thread::Builder::new()
                    .name(String::from("audio:mdns"))
                    .spawn(move || Self::thread(thread_tx, thread_rx))
                    .unwrap(),
            ),
            tx,
            rx,
        }
    }

    pub fn recv_timeout(&self, dur: Duration) -> Result<SocketAddrV4, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(dur)
    }

    fn thread(tx: mpsc::Sender<SocketAddrV4>, rx: mpsc::Receiver<Command>) {
        let mut daemon = None;
        let mut receiver = None;
        let mut ip_addr = None;
        let mut ip_time = Instant::now();

        println!("mDNS thread started");

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Quit) => {
                    println!("mDNS thread received Quit");
                    Self::stop(&mut daemon, &mut receiver);
                    break;
                }
                _ => {}
            }

            if ip_time.elapsed() >= Duration::from_secs(1) {
                match local_ip() {
                    Ok(ip) => match ip_addr {
                        Some(old_ip) => {
                            if ip != old_ip {
                                println!("New IP: {:?}", ip);
                                ip_addr = Some(ip);
                                Self::stop(&mut daemon, &mut receiver);
                                Self::start(&mut daemon, &mut receiver);
                            }
                        }
                        None => {
                            println!("New IP: {:?}", ip);
                            ip_addr = Some(ip);
                            Self::start(&mut daemon, &mut receiver);
                        }
                    },
                    Err(_) => {
                        if ip_addr.take().is_some() {
                            println!("Lost IP!");
                            Self::stop(&mut daemon, &mut receiver);
                        }
                    }
                }
                ip_time = Instant::now();
            }

            if let Some(r) = &receiver {
                match r.recv_timeout(Duration::from_millis(10)) {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        if let Some(addr) = info.get_addresses().iter().next() {
                            println!(
                                "mDNS service resolved: {} {} {}:{}",
                                info.get_fullname(),
                                info.get_hostname(),
                                addr,
                                info.get_port(),
                            );
                            tx.send(SocketAddrV4::new(*addr, info.get_port())).unwrap();
                        }
                    }
                    Ok(event) => println!("mDNS received event: {:?}", event),
                    Err(_) => {}
                }
            }
        }

        println!("mDNS thread stopped");
    }

    fn start(daemon: &mut Option<ServiceDaemon>, receiver: &mut Option<Receiver<ServiceEvent>>) {
        if let (None, None) = (&daemon, &receiver) {
            let d = ServiceDaemon::new().unwrap();
            let r = d.browse(SERVICE_TYPE).unwrap();
            *daemon = Some(d);
            *receiver = Some(r);
            println!("mDNS daemon started");
        }
    }

    fn stop(daemon: &mut Option<ServiceDaemon>, receiver: &mut Option<Receiver<ServiceEvent>>) {
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
            println!("mDNS daemon stopped");
        }
    }
}

impl Drop for Mdns {
    fn drop(&mut self) {
        self.tx.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
