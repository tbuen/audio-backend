use crate::com::mdns::Mdns;
use crate::com::websocket::{Event, WebSocket};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod mdns;
mod websocket;

enum Command {
    Quit,
}

pub struct Com {
    handle: Option<thread::JoinHandle<()>>,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<String>,
}

impl Com {
    pub fn new() -> Self {
        let (thread_tx, rx) = mpsc::channel();
        let (tx, thread_rx) = mpsc::channel();
        Self {
            handle: Some(
                thread::Builder::new()
                    .name(String::from("audio:com"))
                    .spawn(move || Self::thread(thread_tx, thread_rx))
                    .unwrap(),
            ),
            tx,
            rx,
        }
    }

    pub fn recv_timeout(&self, dur: Duration) -> Result<String, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(dur)
    }

    fn thread(_tx: mpsc::Sender<String>, rx: mpsc::Receiver<Command>) {
        let mut mdns = Some(Mdns::new());
        let mut websocket = None;

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Quit) => {
                    println!("com thread received Quit");
                    break;
                }
                _ => {}
            }

            if let Some(m) = &mdns {
                match m.recv_timeout(Duration::from_millis(10)) {
                    Ok(sock) => {
                        println!("mDNS result: {}", sock);
                        mdns.take().unwrap();
                        websocket = Some(WebSocket::new(sock));
                    }
                    _ => {}
                }
            }

            if let Some(ws) = &websocket {
                match ws.recv_timeout(Duration::from_millis(10)) {
                    Ok(Event::Connected) => {
                        println!("WebSocket connected!");
                    }
                    Ok(Event::Disconnected) => {
                        println!("WebSocket disconnected!");
                        websocket.take().unwrap();
                        mdns = Some(Mdns::new());
                    }
                    Ok(Event::Message(msg)) => {
                        println!("WebSocket message: {}", msg);
                    }
                    _ => {}
                }
            }
        }

        println!("exit com thread");
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        self.tx.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
