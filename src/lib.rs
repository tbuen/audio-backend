mod com;
mod common;
mod json;

use std::cell::Cell;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{Builder, JoinHandle};
use std::time::Duration;

use log::{debug, error, info};

use crate::common::access_point::Connector;
use crate::json::{Handler, Message, Response};

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
    Connected(Con, Version),
    Disconnected,
    ScanResult(Result<Vec<Network>, Error>),
    NetworkList(Result<Vec<String>, Error>),
    //Reload(Reload),
}

#[derive(Debug)]
pub struct Error {
    _code: i16,
    _message: String,
}

#[derive(Debug, Clone)]
pub struct Con {
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct Version {
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
    GetWifiScanResult,
    GetWifiNetworkList,
    SetWifiNetwork { ssid: String, key: String },
    DeleteWifiNetwork { ssid: String },
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

    pub fn receiver(&self) -> Option<Receiver<Event>> {
        self.receiver.take()
    }

    pub fn get_access_point_mode(&self) -> bool {
        let (mutex, cvar) = &*self.direct;
        let mut data = mutex.lock().unwrap();
        self.sender.send(Command::GetAccessPointMode).unwrap();
        data = cvar.wait(data).unwrap();
        *data
    }

    pub fn set_access_point_mode(&self, auto: bool) {
        self.sender.send(Command::SetAccessPointMode(auto)).unwrap();
    }

    pub fn get_wifi_scan_result(&self) {
        self.sender.send(Command::GetWifiScanResult).unwrap();
    }

    pub fn get_wifi_network_list(&self) {
        self.sender.send(Command::GetWifiNetworkList).unwrap();
    }

    pub fn set_wifi_network(&self, ssid: String, key: String) {
        self.sender
            .send(Command::SetWifiNetwork { ssid, key })
            .unwrap();
    }

    pub fn delete_wifi_network(&self, ssid: String) {
        self.sender
            .send(Command::DeleteWifiNetwork { ssid })
            .unwrap();
    }

    //pub fn database(&self) -> Database {
    //    self.database.clone()
    //}

    //fn thread(tx: Sender<Event>, rx: Receiver<Command>, database: Database) {
    fn thread(tx: Sender<Event>, rx: Receiver<Command>, direct: Arc<(Mutex<bool>, Condvar)>) {
        let com = com::Com::new();
        let json = Handler::default();
        let (mutex, cvar) = &*direct;
        let mut ap = None;
        let mut mode = None;

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
                            ap = Some(Connector::new(
                                "esp32-audio".to_owned(),
                                "secret-wifi-key".to_owned(),
                            ));
                        } else if !auto && ap.is_some() {
                            ap.take();
                        }
                    }
                    Command::GetWifiScanResult => {
                        com.send(json.get_wifi_scan_result());
                    }
                    Command::GetWifiNetworkList => {
                        com.send(json.get_wifi_network_list());
                    }
                    Command::SetWifiNetwork { ssid, key } => {
                        com.send(json.set_wifi_network(&ssid, &key));
                    }
                    Command::DeleteWifiNetwork { ssid } => {
                        com.send(json.delete_wifi_network(&ssid));
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
                    com::Event::Connected => {
                        info!("Connected!");
                        com.send(json.get_info_con());
                    }
                    com::Event::Disconnected => {
                        info!("Disconnected!");
                        mode.take();
                        tx.send(Event::Disconnected).unwrap();
                    }
                    com::Event::Message(msg) => {
                        debug!("Message: {msg}");
                        if let Some(m) = json.parse(&msg) {
                            debug!("Backend received valid message :-)");
                            //Self::handle_message(m, &com, &mut rpc, &tx, &database);
                            Self::handle_message(m, &com, &json, &tx, &mut mode);
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
        com: &com::Com,
        json: &Handler,
        tx: &Sender<Event>,
        //database: &Database,
        mode: &mut Option<String>,
    ) {
        match msg {
            Message::Response(resp) => match resp {
                Response::InfoCon(res) => match res {
                    Ok(con) => {
                        mode.replace(con.mode);
                        com.send(json.get_info_about());
                    }
                    Err(e) => error!("Could not get InfoCon: {e}"),
                },
                Response::InfoAbout(res) => match res {
                    Ok(about) => {
                        let evt = Event::Connected(
                            Con {
                                mode: mode.as_ref().unwrap().to_owned(),
                            },
                            Version {
                                project: about.project,
                                version: about.version,
                                esp_idf: about.esp_idf,
                            },
                        );
                        tx.send(evt).unwrap();
                    }
                    Err(e) => error!("Could not get InfoAbout: {e}"),
                },
                Response::ScanResult(res) => match res {
                    Ok(list) => {
                        let mut networks = Vec::new();
                        for e in list {
                            networks.push(Network {
                                ssid: e.ssid,
                                rssi: e.rssi,
                            });
                        }
                        let evt = Event::ScanResult(Ok(networks));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => {
                        let evt = Event::ScanResult(Err(Error {
                            _code: e.code,
                            _message: e.message,
                        }));
                        tx.send(evt).unwrap();
                    }
                },
                Response::NetworkList(res) => match res {
                    Ok(list) => {
                        let evt = Event::NetworkList(Ok(list));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => {
                        let evt = Event::NetworkList(Err(Error {
                            _code: e.code,
                            _message: e.message,
                        }));
                        tx.send(evt).unwrap();
                    }
                },
            },
            /*RpcResult::FileList(lst) => {
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
