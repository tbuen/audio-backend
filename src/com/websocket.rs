use std::io::ErrorKind::WouldBlock;
use std::net::SocketAddrV4;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};

use tungstenite::Bytes;
use tungstenite::client::connect;
use tungstenite::error::Error::{ConnectionClosed, Io, Protocol};
use tungstenite::error::ProtocolError::ResetWithoutClosingHandshake;
use tungstenite::protocol::Message::{Ping, Pong, Text};
use tungstenite::stream::MaybeTlsStream;

use log::{debug, error};

use super::Event;

pub(crate) struct WebSocket {
    handle: Option<JoinHandle<()>>,
    sender: Sender<Command>,
    receiver: Receiver<Event>,
}

enum Command {
    Quit,
    Message(String),
}

impl WebSocket {
    pub(crate) fn new(sock: SocketAddrV4) -> Self {
        let (sender, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        Self {
            handle: Some(
                Builder::new()
                    .name("audio:websocket".into())
                    .spawn(move || Self::thread(sock, tx, rx))
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

    fn thread(sock: SocketAddrV4, tx: Sender<Event>, rx: Receiver<Command>) {
        #![expect(clippy::similar_names)]

        let url = format!("ws://{}:{}/websocket", sock.ip(), sock.port());
        let mut websocket = None;

        debug!("try to connect");
        match connect(&url) {
            Ok((ws, _)) => {
                debug!("connected :)");
                if let MaybeTlsStream::Plain(s) = ws.get_ref() {
                    s.set_nonblocking(true).unwrap();
                }
                websocket = Some(ws);
                tx.send(Event::Connected).unwrap();
            }
            Err(e) => {
                error!("Error connecting ws: {e:?}");
            }
        }

        if let Some(mut ws) = websocket {
            const PING_INTERVAL: u64 = 2;
            const PONG_TIMEOUT: u64 = 5;
            const CLOSE_TIMEOUT: u64 = 5;

            let mut ping_time = Instant::now();
            let mut pong_time = Instant::now();
            let mut close_time = None;

            loop {
                if let Ok(cmd) = rx.recv_timeout(Duration::from_millis(10)) {
                    match cmd {
                        Command::Quit => {
                            debug!("ws thread received Quit");
                            ws.close(None).unwrap();
                            close_time = Some(Instant::now());
                        }
                        Command::Message(msg) => {
                            debug!("try to send message {msg}");
                            match ws.send(Text(msg.into())) {
                                Ok(_) => (),
                                Err(e) => {
                                    error!("ws send error: {e:?}");
                                }
                            }
                        }
                    }
                }

                if ping_time.elapsed() >= Duration::from_secs(PING_INTERVAL) {
                    if ws.can_write() {
                        match ws.send(Ping(Bytes::new())) {
                            Ok(()) => {
                                debug!("ping...");
                            }
                            Err(e) => {
                                error!("ws send error: {e:?}");
                            }
                        }
                    } else {
                        debug!("too late to write...");
                    }
                    ping_time = Instant::now();
                }

                if let Some(t) = close_time {
                    if t.elapsed() >= Duration::from_secs(CLOSE_TIMEOUT) {
                        debug!("close not successful after {CLOSE_TIMEOUT}s => hard close");
                        break;
                    }
                } else if pong_time.elapsed() >= Duration::from_secs(PONG_TIMEOUT) {
                    debug!("no pong received within {PONG_TIMEOUT}s");
                    ws.close(None).unwrap();
                    close_time = Some(Instant::now());
                }

                match ws.read() {
                    Ok(Pong(_)) => {
                        debug!("...pong");
                        pong_time = Instant::now();
                    }
                    Ok(Text(s)) => {
                        debug!("ws received message: {s}");
                        tx.send(Event::Message(s.as_str().into())).unwrap();
                    }
                    Ok(msg) => {
                        debug!("ws received {msg:?}");
                    }
                    Err(ConnectionClosed) => {
                        debug!("ws connection closed with handshake");
                        break;
                    }
                    Err(Protocol(ResetWithoutClosingHandshake)) => {
                        debug!("ws connection closed without handshake");
                        break;
                    }
                    Err(Io(e)) => {
                        if e.kind() == WouldBlock {
                            //debug!("would block...");
                        } else {
                            error!("ws received IO error: {e:?}");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("ws received error: {e:?}");
                    }
                }
            }
        }

        tx.send(Event::Disconnected).unwrap();

        if let Ok(Command::Quit) = rx.recv_timeout(Duration::from_secs(1)) {
            debug!("ws thread received Quit after close");
        }

        debug!("ws thread stopped");
    }
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        self.sender.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
