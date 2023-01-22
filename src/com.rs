use crate::com::mdns::Mdns;
use crate::com::ws::Ws;
use std::net::SocketAddrV4;

mod mdns;
mod ws;

pub struct Client {
    mdns: Mdns,
    socket: Option<SocketAddrV4>,
    ws: Option<Ws>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            mdns: Mdns::new(),
            socket: None,
            ws: None,
        }
    }

    pub async fn shutdown(&mut self) {
        self.mdns.shutdown();
        if let Some(ws) = &mut self.ws {
            ws.shutdown().await;
        }
    }

    pub async fn recv(&mut self) -> Option<String> {
        println!("com recv");
        let mut data = None;
        loop {
            if let Some(ws) = &mut self.ws {
                data = ws.recv().await;
                break;
            } else if let Some(s) = self.socket {
                self.ws = Ws::connect(s).await;
            } else {
                match self.mdns.scan().await {
                    Some(s) => self.socket = Some(s),
                    None => break,
                }
            }
        }
        data
    }
}
