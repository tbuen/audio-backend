use crate::com::mdns::Mdns;
use std::net::SocketAddrV4;

mod mdns;

pub struct Client {
    socket_addr: Option<SocketAddrV4>,
    mdns: Mdns,
}

impl Client {
    pub fn new() -> Self {
        Self {
            socket_addr: None,
            mdns: Mdns::new(),
        }
    }

    pub fn shutdown(&self) {
        self.mdns.shutdown();
    }

    pub async fn recv(&mut self) -> Option<String> {
        println!("com recv");
        loop {
            match self.socket_addr {
                Some(_) => {}
                None => match self.mdns.scan().await {
                    Some(s) => self.socket_addr = Some(s),
                    None => break,
                },
            }
        }
        None
    }
}
