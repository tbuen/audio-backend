use crate::json::{Message, Rpc, RpcResult};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod com;
mod json;

pub const VERSION: &str = env!("VERSION");

// TODO move to other file, types.rs ???
#[derive(Debug)]
pub struct Version {
    pub project: String,
    pub version: String,
    pub esp_idf: String,
}

enum Command {
    Quit,
}

pub enum Event {
    Connected,
    Version(Version),
    Synchronized,
    Disconnected,
}

pub struct Backend {
    handle: Option<thread::JoinHandle<()>>,
    tx: mpsc::Sender<Command>,
}

impl Backend {
    pub fn new() -> (Self, mpsc::Receiver<Event>) {
        let (tx, rx_thread) = mpsc::channel();
        let (tx_thread, rx) = mpsc::channel();
        (
            Self {
                handle: Some(
                    thread::Builder::new()
                        .name(String::from("audio:backend"))
                        .spawn(move || Self::thread(tx_thread, rx_thread))
                        .unwrap(),
                ),
                tx,
            },
            rx,
        )
    }

    fn thread(tx: mpsc::Sender<Event>, rx: mpsc::Receiver<Command>) {
        let com = com::Com::new();
        let mut rpc = Rpc::new();

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Quit) => {
                    println!("backend thread quit");
                    break;
                }
                Err(_) => {}
            }

            match com.recv_timeout(Duration::from_millis(10)) {
                Ok(com::Event::Connected) => {
                    println!("Connected!");
                    tx.send(Event::Connected).unwrap();
                    com.send(rpc.get_version());
                    com.send(rpc.get_file_list());
                }
                Ok(com::Event::Disconnected) => {
                    println!("Disconnected!");
                    tx.send(Event::Disconnected).unwrap();
                }
                Ok(com::Event::Message(msg)) => {
                    println!("Message: {}", msg);
                    if let Some(m) = rpc.parse(&msg) {
                        println!("backend received message :-)");
                        // TODO move to separate function or module??
                        match m {
                            Message::Response(r) => match r {
                                RpcResult::Version(ver) => {
                                    let evt = Event::Version(Version {
                                        project: String::from(ver.project),
                                        version: String::from(ver.version),
                                        esp_idf: String::from(ver.esp_idf),
                                    });
                                    tx.send(evt).unwrap();
                                }
                                RpcResult::FileList(_lst) => {
                                    // TODO
                                    tx.send(Event::Synchronized).unwrap();
                                }
                            },
                            //Message::Notification => {}
                        }
                    }
                }
                Err(_) => {}
            }
        }

        println!("exit backend thread");
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.tx.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
