//pub use database::{Database, DirEntry};
//pub use event::{Event, Version};
//use json::{ErrReq, Message, Rpc, RpcResult};
//use std::sync::mpsc::{self, Receiver, Sender};
use log::{debug, info};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
//use crossbeam_channel::{Receiver, Sender};
use com::{Com, Event as ComEvent};
use std::thread::{Builder, JoinHandle};
use std::time::Duration;

mod access_point;
mod com;
//mod database;
//mod event;
//mod json;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("VERSION");

enum Command {
    AccessPoint(Option<bool>),
    //Resync,
    Quit,
}

#[derive(Debug)]
pub struct Version {
    pub project: String,
    pub version: String,
    pub esp_idf: String,
}

//pub enum Reload {
//    Start,
//    Step(Option<(usize, usize)>),
//    Stop,
//}

pub enum Event {
    Connected,
    Disconnected,
    Version(Version),
    //Reload(Reload),
    //Error(String),
}

pub struct Backend {
    handle: Option<JoinHandle<()>>,
    sender: Sender<Command>,
    receiver: Receiver<Event>,
    direct: Arc<(Mutex<bool>, Condvar)>,
    //database: Database,
}

impl Backend {
    pub fn new() -> Self {
        let (sender, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        let direct = Arc::new((Mutex::new(false), Condvar::new()));
        let direct_thread = Arc::clone(&direct);
        //let database = Database::new(sender.clone());
        let handle = {
            //let database = database.clone();
            Builder::new()
                .name("audio:backend".into())
                .spawn(move || Self::thread(tx, rx, direct_thread))
                .unwrap()
        };
        Self {
            handle: Some(handle),
            sender,
            receiver,
            direct,
            //database,
        }
    }

    pub fn get_access_point_mode(&self) -> bool {
        let (mutex, cvar) = &*self.direct;
        let mut data = mutex.lock().unwrap();
        self.sender.send(Command::AccessPoint(None)).unwrap();
        data = cvar.wait(data).unwrap();
        *data
    }

    /// Enables or disables automatic connection to an audio-esp access point.
    pub fn set_access_point_mode(&self, automatic: bool) {
        self.sender
            .send(Command::AccessPoint(Some(automatic)))
            .unwrap();
    }

    pub fn receiver(&self) -> &Receiver<Event> {
        &self.receiver
    }

    //pub fn database(&self) -> Database {
    //    self.database.clone()
    //}

    //fn thread(tx: Sender<Event>, rx: Receiver<Command>, database: Database) {
    fn thread(tx: Sender<Event>, rx: Receiver<Command>, direct: Arc<(Mutex<bool>, Condvar)>) {
        let com = Com::new();
        //let mut rpc = Rpc::new();
        let (mutex, cvar) = &*direct;
        let mut ap = None;

        loop {
            match rx.try_recv() {
                Ok(Command::AccessPoint(auto)) => {
                    if let Some(a) = auto {
                        if a && ap.is_none() {
                            ap = Some(access_point::Connector::new());
                        } else if !a && ap.is_some() {
                            ap.take();
                        }
                    } else {
                        let mut data = mutex.lock().unwrap();
                        *data = ap.is_some();
                        cvar.notify_one();
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

            match com.recv_timeout(Duration::from_millis(10)) {
                Ok(ComEvent::Connected) => {
                    info!("Connected!");
                    tx.send(Event::Connected).unwrap();
                    //com.send(rpc.get_version());
                }
                Ok(ComEvent::Disconnected) => {
                    info!("Disconnected!");
                    tx.send(Event::Disconnected).unwrap();
                }
                Ok(ComEvent::Message(msg)) => {
                    debug!("Message: {msg}");
                    /*if let Some(m) = rpc.parse(&msg) {
                        println!("backend received message :-)");
                        Self::handle_message(m, &com, &mut rpc, &tx, &database);
                    }*/
                }
                Err(_) => {}
            }
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

impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.sender.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
