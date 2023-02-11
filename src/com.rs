use crate::com::mdns::Mdns;
pub use crate::com::websocket::Event;
use crate::com::websocket::WebSocket;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod mdns;
mod websocket;

enum Command {
    Quit,
    Message(String),
}

pub struct Com {
    handle: Option<thread::JoinHandle<()>>,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Event>,
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

    pub fn recv_timeout(&self, dur: Duration) -> Result<Event, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(dur)
    }

    pub fn send(&self, msg: String) {
        self.tx.send(Command::Message(msg)).unwrap();
    }

    fn thread(tx: mpsc::Sender<Event>, rx: mpsc::Receiver<Command>) {
        let mut mdns = Some(Mdns::new());
        let mut websocket: Option<WebSocket> = None;

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Quit) => {
                    println!("com thread received Quit");
                    break;
                }
                Ok(Command::Message(msg)) => {
                    if let Some(ws) = &websocket {
                        ws.send(msg);
                    }
                }
                Err(_) => {}
            }

            if let Some(m) = &mdns {
                match m.recv_timeout(Duration::from_millis(10)) {
                    Ok(sock) => {
                        println!("mDNS result: {}", sock);
                        mdns.take().unwrap();
                        websocket = Some(WebSocket::new(sock));
                    }
                    Err(_) => {}
                }
            }

            if let Some(ws) = &websocket {
                match ws.recv_timeout(Duration::from_millis(10)) {
                    Ok(evt) => {
                        if let Event::Disconnected = &evt {
                            websocket.take().unwrap();
                            mdns = Some(Mdns::new());
                        }
                        tx.send(evt).unwrap();
                    }
                    Err(_) => {}
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
