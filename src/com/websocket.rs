use std::io::ErrorKind::WouldBlock;
use std::net::SocketAddrV4;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::client::connect;
use tungstenite::error::Error::{ConnectionClosed, Io, Protocol};
use tungstenite::error::ProtocolError::ResetWithoutClosingHandshake;
use tungstenite::protocol::Message::{Ping, Pong, Text};
use tungstenite::stream::MaybeTlsStream;

enum Command {
    Quit,
    Message(String),
}

pub enum Event {
    Connected,
    Disconnected,
    Message(String),
}

pub struct WebSocket {
    handle: Option<thread::JoinHandle<()>>,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Event>,
}

impl WebSocket {
    pub fn new(sock: SocketAddrV4) -> Self {
        let (thread_tx, rx) = mpsc::channel();
        let (tx, thread_rx) = mpsc::channel();
        Self {
            handle: Some(
                thread::Builder::new()
                    .name(String::from("audio:websocket"))
                    .spawn(move || Self::thread(sock, thread_tx, thread_rx))
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

    fn thread(sock: SocketAddrV4, tx: mpsc::Sender<Event>, rx: mpsc::Receiver<Command>) {
        let url = format!("ws://{}:{}/websocket", sock.ip(), sock.port());
        let mut websocket = None;

        println!("try to connect");
        match connect(&url) {
            Ok((ws, _)) => {
                println!("connected :)");
                match ws.get_ref() {
                    MaybeTlsStream::Plain(s) => s.set_nonblocking(true).unwrap(),
                    _ => {}
                }
                websocket = Some(ws);
                tx.send(Event::Connected).unwrap();
            }
            Err(e) => {
                println!("Error connecting ws: {}", e);
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
                match rx.recv_timeout(Duration::from_millis(10)) {
                    Ok(Command::Quit) => {
                        println!("ws thread received Quit");
                        ws.close(None).unwrap();
                        close_time = Some(Instant::now());
                    }
                    Ok(Command::Message(msg)) => {
                        println!("try to send message {}", msg);
                        match ws.write_message(Text(msg)) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("ws send error: {:?}", e);
                            }
                        }
                    }
                    Err(_) => {}
                }

                if ping_time.elapsed() >= Duration::from_secs(PING_INTERVAL) {
                    if ws.can_write() {
                        match ws.write_message(Ping(Vec::new())) {
                            Ok(_) => {
                                println!("ping...");
                            }
                            Err(e) => {
                                println!("ws send error: {:?}", e);
                            }
                        }
                    } else {
                        println!("too late to write...");
                    }
                    ping_time = Instant::now();
                }

                if let Some(t) = close_time {
                    if t.elapsed() >= Duration::from_secs(CLOSE_TIMEOUT) {
                        println!(
                            "close not successful after {}s => hard close",
                            CLOSE_TIMEOUT
                        );
                        break;
                    }
                } else {
                    if pong_time.elapsed() >= Duration::from_secs(PONG_TIMEOUT) {
                        println!("no pong received within {}s", PONG_TIMEOUT);
                        ws.close(None).unwrap();
                        close_time = Some(Instant::now());
                    }
                }

                match ws.read_message() {
                    Ok(Pong(_)) => {
                        println!("...pong");
                        pong_time = Instant::now();
                    }
                    Ok(Text(s)) => {
                        println!("ws received message: {}", s);
                        tx.send(Event::Message(s)).unwrap();
                    }
                    Ok(msg) => {
                        println!("ws received {:?}", msg);
                    }
                    Err(ConnectionClosed) => {
                        println!("ws connection closed with handshake");
                        break;
                    }
                    Err(Protocol(ResetWithoutClosingHandshake)) => {
                        println!("ws connection closed without handshake");
                        break;
                    }
                    Err(Io(e)) => {
                        if e.kind() == WouldBlock {
                            //println!("would block...");
                        } else {
                            println!("ws received IO error: {:?}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        println!("ws received error: {:?}", e);
                    }
                }
            }
        }

        tx.send(Event::Disconnected).unwrap();

        println!("ws thread stopped");
    }
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        match self.tx.send(Command::Quit) {
            _ => {}
        }
        self.handle.take().unwrap().join().unwrap();
    }
}
