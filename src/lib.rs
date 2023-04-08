pub use database::{Database, DirEntry};
pub use event::{Event, Reload};
use json::{ErrReq, Message, Rpc, RpcResult};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

mod com;
mod database;
mod event;
mod json;

pub const VERSION: &str = env!("VERSION");

enum Command {
    Resync,
    Quit,
}

pub struct Backend {
    handle: Option<thread::JoinHandle<()>>,
    sender: Sender<Command>,
    database: Database,
}

impl Backend {
    pub fn new() -> (Self, Receiver<Event>) {
        let (sender, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        let database = Database::new(sender.clone());
        let handle = {
            let database = database.clone();
            thread::Builder::new()
                .name(String::from("audio:backend"))
                .spawn(move || Self::thread(tx, rx, database))
                .unwrap()
        };
        let backend = Self {
            handle: Some(handle),
            sender,
            database,
        };
        (backend, receiver)
    }

    pub fn database(&self) -> Database {
        self.database.clone()
    }

    fn thread(tx: Sender<Event>, rx: Receiver<Command>, database: Database) {
        let com = com::Com::new();
        let mut rpc = Rpc::new();

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Resync) => {
                    database.clear_file_list();
                    tx.send(Event::Reload(Reload::Start)).unwrap();
                    com.send(rpc.get_file_list(true));
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
                    com.send(rpc.get_version());
                }
                Ok(com::Event::Disconnected) => {
                    println!("Disconnected!");
                    tx.send(Event::Disconnected).unwrap();
                }
                Ok(com::Event::Message(msg)) => {
                    println!("Message: {}", msg);
                    if let Some(m) = rpc.parse(&msg) {
                        println!("backend received message :-)");
                        Self::handle_message(m, &com, &mut rpc, &tx, &database);
                    }
                }
                Err(_) => {}
            }
        }

        println!("exit backend thread");
    }

    fn handle_message(
        msg: Message,
        com: &com::Com,
        rpc: &mut Rpc,
        tx: &Sender<Event>,
        database: &Database,
    ) {
        match msg {
            Message::Response(r) => match r {
                Ok(r) => match r {
                    RpcResult::Version(ver) => {
                        let evt = Event::Version(event::Version {
                            project: String::from(&ver.project),
                            version: String::from(&ver.version),
                            esp_idf: String::from(&ver.esp_idf),
                        });
                        tx.send(evt).unwrap();
                    }
                    RpcResult::FileList(lst) => {
                        if lst.first {
                            database.clear_file_list();
                        }
                        database.append_file_list(lst.files);
                        if lst.last {
                            match database.get_unsynced_file() {
                                Some(f) => {
                                    tx.send(Event::Reload(Reload::Step)).unwrap();
                                    com.send(rpc.get_file_info(f));
                                }
                                None => {
                                    tx.send(Event::Reload(Reload::Stop)).unwrap();
                                }
                            }
                        } else {
                            tx.send(Event::Reload(Reload::Step)).unwrap();
                            com.send(rpc.get_file_list(false));
                        }
                    }
                    RpcResult::FileInfo(info) => {
                        database.set_file_info(info);
                        match database.get_unsynced_file() {
                            Some(f) => {
                                tx.send(Event::Reload(Reload::Step)).unwrap();
                                com.send(rpc.get_file_info(f));
                            }
                            None => {
                                tx.send(Event::Reload(Reload::Stop)).unwrap();
                            }
                        }
                    }
                },
                Err(e) => {
                    tx.send(Event::Error(format!(
                        "{}: {} ({})",
                        e.method, e.message, e.code
                    )))
                    .unwrap();
                    match e.request {
                        ErrReq::Version => {}
                        ErrReq::FileList => {
                            tx.send(Event::Reload(Reload::Stop)).unwrap();
                        }
                        ErrReq::FileInfo => {
                            tx.send(Event::Reload(Reload::Stop)).unwrap();
                        }
                        _ => {}
                    }
                }
            },
            //Message::Notification => {}
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.sender.send(Command::Quit).unwrap();
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
