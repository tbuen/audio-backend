mod mdns;
mod websocket;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{Builder, JoinHandle};
use std::time::Duration;

use log::{debug, error, info};

use self::mdns::Mdns;
use self::websocket::WebSocket;

pub(crate) struct Com {
    handle: Option<JoinHandle<()>>,
    sender: Sender<Command>,
    receiver: Receiver<Event>,
}

pub(crate) enum Event {
    Connected,
    Disconnected,
    Message(String),
}

enum Command {
    Quit,
    Message(String),
}

impl Com {
    pub(crate) fn new() -> Self {
        let (sender, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        Self {
            handle: Some(
                Builder::new()
                    .name("audio:com".into())
                    .spawn(move || Self::thread(tx, rx))
                    .unwrap(),
            ),
            sender,
            receiver,
        }
    }

    pub(crate) fn recv_timeout(&self, dur: Duration) -> Result<Event, RecvTimeoutError> {
        self.receiver.recv_timeout(dur)
    }

    pub(crate) fn send(&self, msg: String) {
        self.sender.send(Command::Message(msg)).unwrap();
    }

    fn thread(tx: Sender<Event>, rx: Receiver<Command>) {
        let mut mdns = Some(Mdns::new());
        let mut websocket: Option<WebSocket> = None;

        loop {
            // TODO: try to give the receiver to mdns and websocket so that they can directly send their messages and com doesn't need to poll
            if let Ok(cmd) = rx.recv_timeout(Duration::from_millis(10)) {
                match cmd {
                    Command::Quit => {
                        debug!("com thread received Quit");
                        break;
                    }
                    Command::Message(msg) => {
                        if let Some(ws) = &websocket {
                            ws.send(msg);
                        } else {
                            error!("not connected!");
                        }
                    }
                }
            }

            if let Some(m) = &mdns
                && let Ok(sock) = m.recv_timeout(Duration::from_millis(10))
            {
                info!("mDNS result: {sock}");
                mdns.take().unwrap();
                websocket = Some(WebSocket::new(sock));
            }

            if let Some(ws) = &websocket
                && let Ok(evt) = ws.recv_timeout(Duration::from_millis(10))
            {
                if let Event::Disconnected = &evt {
                    websocket.take().unwrap();
                    mdns = Some(Mdns::new());
                }
                tx.send(evt).unwrap();
            }
        }

        debug!("exit com thread");
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        self.sender.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
