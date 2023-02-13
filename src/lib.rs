use crate::json::{Message, Result, Rpc};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod com;
mod json;

pub const VERSION: &str = env!("VERSION");

// TODO move to other file, types.rs ???
#[derive(Debug)]
pub struct Info {
    pub project: String,
    pub version: String,
    pub esp_idf: String,
}

enum Command {
    Idle,
    Quit,
}

pub enum Event {
    Connected,
    Disconnected,
    Info(Info),
}

pub struct Backend {
    handle: Option<thread::JoinHandle<()>>,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Event>,
}

impl Backend {
    pub fn new() -> Self {
        let (thread_tx, rx) = mpsc::channel();
        let (tx, thread_rx) = mpsc::channel();
        Self {
            handle: Some(
                thread::Builder::new()
                    .name(String::from("audio:backend"))
                    .spawn(move || Self::thread(thread_tx, thread_rx))
                    .unwrap(),
            ),
            tx,
            rx,
        }
    }

    pub fn receive(&self) -> Option<Event> {
        self.tx.send(Command::Idle).unwrap();
        self.rx.try_recv().ok()
    }

    fn thread(tx: mpsc::Sender<Event>, rx: mpsc::Receiver<Command>) {
        let com = com::Com::new();
        let mut rpc = Rpc::new();

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Idle) => {
                    println!("backend thread idle");
                }
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
                    com.send(rpc.get_info());
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
                                Result::Info(i) => {
                                    let evt = Event::Info(Info {
                                        project: String::from(i.project),
                                        version: String::from(i.version),
                                        esp_idf: String::from(i.esp_idf),
                                    });
                                    tx.send(evt).unwrap();
                                }
                                Result::FileList(_lst) => {
                                    // TODO
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
