mod access_point;
mod com;
mod json;

use std::cell::Cell;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{Builder, JoinHandle};
use std::time::Duration;

use log::{debug, info};

use self::com::{Com, Event as ComEvent};
use self::json::{Message, Rpc, RpcResult};

//pub use database::{Database, DirEntry};
//pub use event::{Event, Version};
//use std::sync::mpsc::{self, Receiver, Sender};
//use crossbeam_channel::{Receiver, Sender};

//mod database;
//mod event;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("VERSION");

pub struct Backend {
    handle: Option<JoinHandle<()>>,
    sender: Sender<Command>,
    receiver: Cell<Option<Receiver<Event>>>,
    direct: Arc<(Mutex<bool>, Condvar)>,
    //database: Database,
}

pub enum Event {
    Connected(Info),
    Disconnected,
    ScanResult(Vec<Network>),
    NetworkList(Vec<String>),
    //Reload(Reload),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Info {
    pub con_type: String,
    pub project: String,
    pub version: String,
    pub esp_idf: String,
}

#[derive(Debug, Clone)]
pub struct Network {
    pub ssid: String,
    pub rssi: i8,
}

enum Command {
    GetAccessPointMode,
    SetAccessPointMode(bool),
    GetScanResult,
    GetNetworkList,
    //Resync,
    Quit,
}

//pub enum Reload {
//    Start,
//    Step(Option<(usize, usize)>),
//    Stop,
//}

impl Backend {
    pub fn new() -> Self {
        let (sender, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        let receiver = Cell::new(Some(receiver));
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
        self.sender.send(Command::GetAccessPointMode).unwrap();
        data = cvar.wait(data).unwrap();
        *data
    }

    pub fn set_access_point_mode(&self, automatic: bool) {
        self.sender
            .send(Command::SetAccessPointMode(automatic))
            .unwrap();
    }

    pub fn get_scan_result(&self) {
        self.sender.send(Command::GetScanResult).unwrap();
    }

    pub fn get_network_list(&self) {
        self.sender.send(Command::GetNetworkList).unwrap();
    }

    pub fn receiver(&self) -> Option<Receiver<Event>> {
        self.receiver.take()
    }

    //pub fn database(&self) -> Database {
    //    self.database.clone()
    //}

    //fn thread(tx: Sender<Event>, rx: Receiver<Command>, database: Database) {
    fn thread(tx: Sender<Event>, rx: Receiver<Command>, direct: Arc<(Mutex<bool>, Condvar)>) {
        let com = Com::new();
        let rpc = Rpc::new();
        let (mutex, cvar) = &*direct;
        let mut ap = None;

        loop {
            if let Ok(cmd) = rx.try_recv() {
                match cmd {
                    Command::GetAccessPointMode => {
                        let mut data = mutex.lock().unwrap();
                        *data = ap.is_some();
                        cvar.notify_one();
                    }
                    Command::SetAccessPointMode(auto) => {
                        if auto && ap.is_none() {
                            ap = Some(access_point::Connector::new());
                        } else if !auto && ap.is_some() {
                            ap.take();
                        }
                    }
                    Command::GetScanResult => {
                        com.send(rpc.get_scan_result());
                    }
                    Command::GetNetworkList => {
                        com.send(rpc.get_network_list());
                    }
                    /*Ok(Command::Resync) => {
                        tx.send(Event::Reload(Reload::Start)).unwrap();
                        com.send(rpc.get_file_list(true));
                    }*/
                    Command::Quit => {
                        debug!("quit received");
                        break;
                    }
                }
            }

            if let Ok(event) = com.recv_timeout(Duration::from_millis(10)) {
                match event {
                    ComEvent::Connected => {
                        info!("Connected!");
                        com.send(rpc.get_info_con());
                    }
                    ComEvent::Disconnected => {
                        info!("Disconnected!");
                        tx.send(Event::Disconnected).unwrap();
                    }
                    ComEvent::Message(msg) => {
                        debug!("Message: {msg}");
                        if let Some(m) = rpc.parse(&msg) {
                            println!("backend received message :-)");
                            //Self::handle_message(m, &com, &mut rpc, &tx, &database);
                            Self::handle_message(m, &com, &rpc, &tx);
                        }
                        //tx.send(Event::Connected).unwrap();
                    }
                }
            }
        }

        debug!("quit");
    }

    fn handle_message(
        msg: Message,
        _com: &Com,
        _rpc: &Rpc,
        tx: &Sender<Event>,
        //database: &Database,
    ) {
        match msg {
            Message::Response(r) => match r {
                Ok(s) => match s {
                    RpcResult::InfoCon(info) => {
                        let evt = Event::Connected(Info {
                            con_type: info.mode,
                            project: info.about.project,
                            version: info.about.version,
                            esp_idf: info.about.esp_idf,
                        });
                        tx.send(evt).unwrap();
                    }
                    RpcResult::ScanResult(list) => {
                        let mut networks = Vec::new();
                        for e in list {
                            networks.push(Network {
                                ssid: e.ssid,
                                rssi: e.rssi,
                            });
                        }
                        let evt = Event::ScanResult(networks);
                        tx.send(evt).unwrap();
                    }
                    RpcResult::NetworkList(list) => {
                        let evt = Event::NetworkList(list);
                        tx.send(evt).unwrap();
                    } /*RpcResult::FileList(lst) => {
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
                      }*/
                },
                Err(e) => {
                    tx.send(Event::Error(format!(
                        "{}: {} ({})",
                        e.method, e.message, e.code
                    )))
                    .unwrap();
                    /*match e.request {
                        ErrReq::Version => {}
                        ErrReq::FileList => {
                            tx.send(Event::Reload(Reload::Stop)).unwrap();
                        }
                        ErrReq::FileInfo => {
                            tx.send(Event::Reload(Reload::Stop)).unwrap();
                        }
                        _ => {}
                    }*/
                }
            },
            //Message::Notification => {}
        }
    }
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
