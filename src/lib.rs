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
    shared: Arc<(Mutex<SharedData>, Condvar)>,
    //database: Database,
}

pub enum Event {
    Connected,
    Disconnected,
    InfoConnection(Result<Connection, RemoteError>),
    InfoAbout(Result<About, RemoteError>),
    InfoMemory(Result<Memory, RemoteError>),
    InfoSPIFlash(Result<SPIFlash, RemoteError>),
    ScanResult(Result<Vec<Network>, RemoteError>),
    NetworkList(Result<Vec<String>, RemoteError>),
    SetNetwork(Result<(), RemoteError>),
    DeleteNetwork(Result<(), RemoteError>),
    //Reload(Reload),
}

#[derive(Debug)]
pub struct NotConnectedError;

#[derive(Debug)]
pub struct RemoteError {
    pub code: i16,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct About {
    pub project: String,
    pub version: String,
    pub esp_idf: String,
}

#[derive(Debug, Clone)]
pub struct Memory {
    pub heap: Heap,
}

#[derive(Debug, Clone)]
pub struct Heap {
    pub allocated: u32,
    pub free: u32,
    pub minimum_free: u32,
}

#[derive(Debug, Clone)]
pub struct SPIFlash {
    pub total: u32,
    pub free: u32,
    pub files: Vec<File>,
}

#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub content_type: String,
    pub size: u32,
    pub md5: String,
}

#[derive(Debug, Clone)]
pub struct Network {
    pub ssid: String,
    pub rssi: i8,
}

enum Command {
    GetAccessPointMode,
    SetAccessPointMode(bool),
    GetInfoConnection,
    GetInfoAbout,
    GetInfoMemory,
    GetInfoSPIFlash,
    GetWifiScanResult,
    GetWifiNetworkList,
    SetWifiNetwork { ssid: String, key: String },
    DeleteWifiNetwork { ssid: String },
    //Resync,
    Quit,
}

#[derive(Default)]
struct SharedData {
    connected: bool,
    ap_mode: bool,
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
        let shared = Arc::new((Mutex::new(SharedData::default()), Condvar::new()));
        let shared_thread = shared.clone();
        //let database = Database::new(sender.clone());
        let handle = {
            //let database = database.clone();
            Builder::new()
                .name("audio:backend".into())
                .spawn(move || Self::thread(tx, rx, shared_thread))
                .unwrap()
        };
        Self {
            handle: Some(handle),
            sender,
            receiver,
            shared,
            //database,
        }
    }

    pub fn receiver(&self) -> Option<Receiver<Event>> {
        self.receiver.take()
    }

    pub fn get_access_point_mode(&self) -> bool {
        let (mutex, cvar) = &*self.shared;
        let mut data = mutex.lock().unwrap();
        self.sender.send(Command::GetAccessPointMode).unwrap();
        data = cvar.wait(data).unwrap();
        data.ap_mode
    }

    pub fn set_access_point_mode(&self, auto: bool) {
        self.sender.send(Command::SetAccessPointMode(auto)).unwrap();
    }

    pub fn get_info_connection(&self) -> Result<(), NotConnectedError> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(NotConnectedError);
        }
        self.sender.send(Command::GetInfoConnection).unwrap();
        Ok(())
    }

    pub fn get_info_about(&self) -> Result<(), NotConnectedError> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(NotConnectedError);
        }
        self.sender.send(Command::GetInfoAbout).unwrap();
        Ok(())
    }

    pub fn get_info_memory(&self) -> Result<(), NotConnectedError> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(NotConnectedError);
        }
        self.sender.send(Command::GetInfoMemory).unwrap();
        Ok(())
    }

    pub fn get_info_spiflash(&self) -> Result<(), NotConnectedError> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(NotConnectedError);
        }
        self.sender.send(Command::GetInfoSPIFlash).unwrap();
        Ok(())
    }

    pub fn get_wifi_scan_result(&self) -> Result<(), NotConnectedError> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(NotConnectedError);
        }
        self.sender.send(Command::GetWifiScanResult).unwrap();
        Ok(())
    }

    pub fn get_wifi_network_list(&self) -> Result<(), NotConnectedError> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(NotConnectedError);
        }
        self.sender.send(Command::GetWifiNetworkList).unwrap();
        Ok(())
    }

    pub fn set_wifi_network(&self, ssid: String, key: String) -> Result<(), NotConnectedError> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(NotConnectedError);
        }
        self.sender
            .send(Command::SetWifiNetwork { ssid, key })
            .unwrap();
        Ok(())
    }

    pub fn delete_wifi_network(&self, ssid: String) -> Result<(), NotConnectedError> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(NotConnectedError);
        }
        self.sender
            .send(Command::DeleteWifiNetwork { ssid })
            .unwrap();
        Ok(())
    }

    //pub fn database(&self) -> Database {
    //    self.database.clone()
    //}

    //fn thread(tx: Sender<Event>, rx: Receiver<Command>, database: Database) {
    fn thread(tx: Sender<Event>, rx: Receiver<Command>, shared: Arc<(Mutex<SharedData>, Condvar)>) {
        let com = com::Com::new();
        let json = Handler::default();
        let (mutex, cvar) = &*shared;
        let mut ap = None;

        loop {
            if let Ok(cmd) = rx.try_recv() {
                match cmd {
                    Command::GetAccessPointMode => {
                        let mut data = mutex.lock().unwrap();
                        data.ap_mode = ap.is_some();
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
                    Command::GetInfoConnection => {
                        com.send(json.get_info_connection());
                    }
                    Command::GetInfoAbout => {
                        com.send(json.get_info_about());
                    }
                    Command::GetInfoMemory => {
                        com.send(json.get_info_memory());
                    }
                    Command::GetInfoSPIFlash => {
                        com.send(json.get_info_spiflash());
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
                        let mut data = mutex.lock().unwrap();
                        data.connected = true;
                        tx.send(Event::Connected).unwrap();
                    }
                    com::Event::Disconnected => {
                        info!("Disconnected!");
                        let mut data = mutex.lock().unwrap();
                        data.connected = false;
                        tx.send(Event::Disconnected).unwrap();
                    }
                    com::Event::Message(msg) => {
                        debug!("Message: {msg}");
                        if let Some(m) = json.parse(&msg) {
                            debug!("Backend received valid message :-)");
                            //Self::handle_message(m, &com, &mut rpc, &tx, &database);
                            Self::handle_message(m, &com, &json, &tx);
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
        _com: &com::Com,
        _json: &Handler,
        tx: &Sender<Event>,
        //database: &Database,
    ) {
        match msg {
            Message::Response(resp) => match resp {
                Response::InfoConnection(res) => match res {
                    Ok(connection) => {
                        let evt = Event::InfoConnection(Ok(Connection {
                            mode: connection.mode,
                        }));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => error!("Could not get InfoConnection: {e}"),
                },
                Response::InfoAbout(res) => match res {
                    Ok(about) => {
                        let evt = Event::InfoAbout(Ok(About {
                            project: about.project,
                            version: about.version,
                            esp_idf: about.esp_idf,
                        }));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => error!("Could not get InfoAbout: {e}"),
                },
                Response::InfoMemory(res) => match res {
                    Ok(info) => {
                        let evt = Event::InfoMemory(Ok(Memory {
                            heap: Heap {
                                allocated: info.heap.allocated,
                                free: info.heap.free,
                                minimum_free: info.heap.minimum_free,
                            },
                        }));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => error!("Could not get InfoMemory: {e}"),
                },
                Response::InfoSPIFlash(res) => match res {
                    Ok(info) => {
                        let mut files = Vec::new();
                        for f in info.files {
                            files.push(File {
                                name: f.name,
                                content_type: f.content_type,
                                size: f.size,
                                md5: f.md5,
                            });
                        }
                        let evt = Event::InfoSPIFlash(Ok(SPIFlash {
                            total: info.total,
                            free: info.free,
                            files,
                        }));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => error!("Could not get InfoMemory: {e}"),
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
                        let evt = Event::ScanResult(Err(RemoteError {
                            code: e.code,
                            message: e.message,
                        }));
                        tx.send(evt).unwrap();
                    }
                },
                Response::NetworkList(res) => match res {
                    Ok(list) => {
                        let mut networks = Vec::new();
                        for e in list {
                            networks.push(e.ssid);
                        }
                        let evt = Event::NetworkList(Ok(networks));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => {
                        let evt = Event::NetworkList(Err(RemoteError {
                            code: e.code,
                            message: e.message,
                        }));
                        tx.send(evt).unwrap();
                    }
                },
                Response::SetNetwork(res) => match res {
                    Ok(_empty) => {
                        let evt = Event::SetNetwork(Ok(()));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => {
                        let evt = Event::SetNetwork(Err(RemoteError {
                            code: e.code,
                            message: e.message,
                        }));
                        tx.send(evt).unwrap();
                    }
                },
                Response::DeleteNetwork(res) => match res {
                    Ok(_empty) => {
                        let evt = Event::DeleteNetwork(Ok(()));
                        tx.send(evt).unwrap();
                    }
                    Err(e) => {
                        let evt = Event::DeleteNetwork(Err(RemoteError {
                            code: e.code,
                            message: e.message,
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
