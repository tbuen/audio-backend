//pub use database::{Database, DirEntry};
pub use event::Event;
//use json::{ErrReq, Message, Rpc, RpcResult};
//use std::sync::mpsc::{self, Receiver, Sender};
use log::debug;
//use std::sync::mpsc;
//use std::sync::mpsc::{Receiver, Sender};
use crossbeam_channel::{Receiver, Sender};
use std::thread;
use std::time::Duration;

mod access_point;
//mod com;
//mod database;
mod event;
//mod json;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("VERSION");

enum Command {
    AccessPoint(bool),
    //Resync,
    Quit,
}

pub struct Backend {
    handle: Option<thread::JoinHandle<()>>,
    sender: Sender<Command>,
    receiver: Receiver<Event>,
    //database: Database,
}

impl Backend {
    pub fn new() -> Self {
        let (sender, rx) = crossbeam_channel::unbounded();
        let (tx, receiver) = crossbeam_channel::unbounded();
        //let database = Database::new(sender.clone());
        let handle = {
            //let database = database.clone();
            thread::Builder::new()
                .name(String::from("audio:backend"))
                .spawn(move || Self::thread(tx, rx))
                .unwrap()
        };
        Self {
            handle: Some(handle),
            sender,
            receiver,
            //database,
        }
    }

    /// Enables or disables automatic connection to an audio-esp access point.
    pub fn set_access_point_mode(&self, automatic: bool) {
        self.sender.send(Command::AccessPoint(automatic)).unwrap();
    }

    pub fn receiver(&self) -> &Receiver<Event> {
        &self.receiver
    }

    //pub fn database(&self) -> Database {
    //    self.database.clone()
    //}

    //fn thread(tx: Sender<Event>, rx: Receiver<Command>, database: Database) {
    fn thread(_: Sender<Event>, rx: Receiver<Command>) {
        //let com = com::Com::new();
        //let mut rpc = Rpc::new();
        let mut ap = None;

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::AccessPoint(auto)) => {
                    if auto && ap.is_none() {
                        ap = Some(access_point::Connector::new());
                    } else if !auto && ap.is_some() {
                        ap.take();
                    }
                }
                /*Ok(Command::Resync) => {
                    tx.send(Event::Reload(Reload::Start)).unwrap();
                    com.send(rpc.get_file_list(true));
                }*/
                Ok(Command::Quit) => {
                    debug!("quit received");
                    break;
                }
                Err(_) => {}
            }

            /*match com.recv_timeout(Duration::from_millis(10)) {
                Ok(com::Event::Connected) => {
                    println!("Connected!");
                    tx.send(Event::Connected).unwrap();
                    //com.send(rpc.get_version());
                }
                Ok(com::Event::Disconnected) => {
                    println!("Disconnected!");
                    tx.send(Event::Disconnected).unwrap();
                }
                Ok(com::Event::Message(msg)) => {
                    println!("Message: {}", msg);
                    /*if let Some(m) = rpc.parse(&msg) {
                        println!("backend received message :-)");
                        Self::handle_message(m, &com, &mut rpc, &tx, &database);
                    }*/
                }
                Err(_) => {}
            }*/
        }

        debug!("quit");
    }

    /*fn handle_message(msg: Message, com: &com::Com, rpc: &mut Rpc, tx: &Sender<Event>, database: &Database) {
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
                        database.update_file_list(lst.files, lst.last);
                        if lst.last {
                            match database.get_unsynced_file() {
                                Some(f) => {
                                    let p = database.sync_stats();
                                    tx.send(Event::Reload(Reload::Step(Some(p)))).unwrap();
                                    com.send(rpc.get_file_info(f));
                                }
                                None => {
                                    tx.send(Event::Reload(Reload::Stop)).unwrap();
                                }
                            }
                        } else {
                            tx.send(Event::Reload(Reload::Step(None))).unwrap();
                            com.send(rpc.get_file_list(false));
                        }
                    }
                    RpcResult::FileInfo(info) => {
                        database.set_file_info(info);
                        match database.get_unsynced_file() {
                            Some(f) => {
                                let p = database.sync_stats();
                                tx.send(Event::Reload(Reload::Step(Some(p)))).unwrap();
                                com.send(rpc.get_file_info(f));
                            }
                            None => {
                                tx.send(Event::Reload(Reload::Stop)).unwrap();
                                database.save();
                            }
                        }
                    }
                },
                Err(e) => {
                    tx.send(Event::Error(format!("{}: {} ({})", e.method, e.message, e.code))).unwrap();
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
    }*/
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.sender.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
