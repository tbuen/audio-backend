mod com;
mod common;
mod database;
mod json;

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::sync::{MutexGuard, mpsc};
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};

use log::{debug, error, info};

pub use crate::database::filesystem::{Dir, FileSystem, FsError};

use crate::common::access_point::Connector;
use crate::json::{Handler, Message, Response};

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("VERSION");

const PARALLEL_REQUESTS: usize = 5;
const SYNC_TIMEOUT_S: u64 = 2;

pub struct Backend {
    handle: Option<JoinHandle<()>>,
    cmd_sender: Sender<Command>,
    evt_sender: Sender<Event>,
    receiver: Cell<Option<Receiver<Event>>>,
    shared: Arc<(Mutex<SharedData>, Condvar)>,
    filesystem: Arc<Mutex<FileSystem>>,
}

pub enum Event {
    Connected,
    Disconnected,
    InfoConnection(Result<Connection, RemoteError>), // TODO RemoteError gar nicht zurückgeben, NotAllowedInMode vorher checken. Bei RemoteError dann nur log-Ausgabe. Hier wirklich nur Erfolge eventieren
    InfoAbout(Result<About, RemoteError>),
    InfoMemory(Result<Memory, RemoteError>),
    InfoSPIFlash(Result<SPIFlash, RemoteError>),
    ScanResult(Result<Vec<Network>, RemoteError>),
    NetworkList(Result<Vec<String>, RemoteError>),
    SetNetwork(Result<(), RemoteError>),
    DeleteNetwork(Result<(), RemoteError>),
    FileSync(SyncStatus),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NotConnected,
    AlreadyRunning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Default, Copy, Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    #[default]
    Idle,
    Running,
    Completed,
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
    SyncFiles,
    Quit,
}

#[derive(Default)]
struct SharedData {
    connected: bool,
    ap_mode: bool,
    filesync: FileSync,
}

#[derive(Default)]
struct FileSync {
    running: bool,
    start: Option<Instant>,
    timestamp: Option<Instant>,
    prefix: String,
    map: HashMap<String, FileSyncEntry>,
}

struct FileSyncEntry {
    step: FileSyncStep,
    dirs: Vec<String>,
    files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileSyncStep {
    New,
    Requested,
    Received,
}

impl Backend {
    pub fn new() -> Self {
        let (cmd_sender, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        let evt_sender = tx.clone();
        let receiver = Cell::new(Some(receiver));
        let shared = Arc::new((Mutex::new(SharedData::default()), Condvar::new()));
        let filesystem = Arc::new(Mutex::new(FileSystem::new()));
        let handle = {
            let shared_thread = shared.clone();
            let filesystem_thread = filesystem.clone();
            Builder::new()
                .name("audio:backend".into())
                .spawn(move || Self::thread(tx, rx, shared_thread, filesystem_thread))
                .unwrap()
        };
        Self {
            handle: Some(handle),
            cmd_sender,
            evt_sender,
            receiver,
            shared,
            filesystem,
        }
    }

    pub fn receiver(&self) -> Option<Receiver<Event>> {
        self.receiver.take()
    }

    pub fn get_access_point_mode(&self) -> bool {
        let (mutex, cvar) = &*self.shared;
        let mut data = mutex.lock().unwrap();
        self.cmd_sender.send(Command::GetAccessPointMode).unwrap();
        data = cvar.wait(data).unwrap();
        data.ap_mode
    }

    pub fn set_access_point_mode(&self, auto: bool) {
        self.cmd_sender
            .send(Command::SetAccessPointMode(auto))
            .unwrap();
    }

    pub fn get_info_connection(&self) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        self.cmd_sender.send(Command::GetInfoConnection).unwrap();
        Ok(())
    }

    pub fn get_info_about(&self) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        self.cmd_sender.send(Command::GetInfoAbout).unwrap();
        Ok(())
    }

    pub fn get_info_memory(&self) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        self.cmd_sender.send(Command::GetInfoMemory).unwrap();
        Ok(())
    }

    pub fn get_info_spiflash(&self) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        self.cmd_sender.send(Command::GetInfoSPIFlash).unwrap();
        Ok(())
    }

    pub fn get_wifi_scan_result(&self) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        self.cmd_sender.send(Command::GetWifiScanResult).unwrap();
        Ok(())
    }

    pub fn get_wifi_network_list(&self) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        self.cmd_sender.send(Command::GetWifiNetworkList).unwrap();
        Ok(())
    }

    pub fn set_wifi_network(&self, ssid: String, key: String) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        self.cmd_sender
            .send(Command::SetWifiNetwork { ssid, key })
            .unwrap();
        Ok(())
    }

    pub fn delete_wifi_network(&self, ssid: String) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        self.cmd_sender
            .send(Command::DeleteWifiNetwork { ssid })
            .unwrap();
        Ok(())
    }

    pub fn sync_files(&self) -> Result<(), Error> {
        let (mutex, _) = &*self.shared;
        let mut data = mutex.lock().unwrap();
        if !data.connected {
            return Err(Error::NotConnected);
        }
        if data.filesync.running {
            return Err(Error::AlreadyRunning);
        }
        data.filesync.running = true;
        data.filesync.map.clear();
        data.filesync.start = Some(Instant::now());
        data.filesync.timestamp = Some(Instant::now());
        self.evt_sender
            .send(Event::FileSync(SyncStatus::Running))
            .unwrap();
        self.cmd_sender.send(Command::SyncFiles).unwrap();
        Ok(())
    }

    pub fn filesystem(&self) -> Arc<Mutex<FileSystem>> {
        self.filesystem.clone()
    }

    fn thread(
        tx: Sender<Event>,
        rx: Receiver<Command>,
        shared: Arc<(Mutex<SharedData>, Condvar)>,
        filesystem: Arc<Mutex<FileSystem>>,
    ) {
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
                    Command::SyncFiles => {
                        com.send(json.get_file_list(None));
                    }
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
                        if data.filesync.running {
                            data.filesync.running = false;
                            tx.send(Event::FileSync(SyncStatus::Idle)).unwrap();
                        }
                    }
                    com::Event::Message(msg) => {
                        debug!("Message: {msg}");
                        if let Some(m) = json.parse(&msg) {
                            debug!("Backend received valid message :-)");
                            let data = mutex.lock().unwrap();
                            Self::handle_message(m, &com, &json, &tx, data);
                        }
                    }
                }
            }

            let mut data = mutex.lock().unwrap();
            if data.filesync.running {
                let n_total = data.filesync.map.len();
                let mut n_new = data
                    .filesync
                    .map
                    .values()
                    .filter(|e| e.step == FileSyncStep::New)
                    .count();
                let mut n_req = data
                    .filesync
                    .map
                    .values()
                    .filter(|e| e.step == FileSyncStep::Requested)
                    .count();
                if n_total > 0 && n_new == 0 && n_req == 0 {
                    info!(
                        "file sync finished in {}ms",
                        data.filesync.start.unwrap().elapsed().as_millis()
                    );
                    data.filesync.running = false;
                    let mut fs = filesystem.lock().unwrap();
                    fs.rebuild(&mut data.filesync);
                    tx.send(Event::FileSync(SyncStatus::Completed)).unwrap();
                } else if Instant::now()
                    > data.filesync.timestamp.unwrap() + Duration::from_secs(SYNC_TIMEOUT_S)
                {
                    error!("file sync timeout");
                    data.filesync.running = false;
                    tx.send(Event::FileSync(SyncStatus::Idle)).unwrap();
                } else if n_new > 0 && n_req < PARALLEL_REQUESTS {
                    for (k, v) in &mut data.filesync.map {
                        if v.step == FileSyncStep::New {
                            com.send(json.get_file_list(Some(k)));
                            v.step = FileSyncStep::Requested;
                            n_new -= 1;
                            n_req += 1;
                            if n_new == 0 || n_req == PARALLEL_REQUESTS {
                                break;
                            }
                        }
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
        mut data: MutexGuard<'_, SharedData>,
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
                Response::FileList(res) => match res {
                    Ok(list) => {
                        let dl = if let Some(d) = &list.dirs { d.len() } else { 0 };
                        let fl = if let Some(f) = &list.files {
                            f.len()
                        } else {
                            0
                        };
                        debug!("Received {dl} dirs and {fl} files");
                        if data.filesync.map.is_empty() {
                            data.filesync.prefix.clone_from(&list.path);
                            data.filesync.map.insert(
                                list.path.clone(),
                                FileSyncEntry {
                                    step: FileSyncStep::Requested,
                                    dirs: Vec::new(),
                                    files: Vec::new(),
                                },
                            );
                        }
                        let entry = data.filesync.map.get_mut(&list.path).unwrap();
                        entry.step = FileSyncStep::Received;
                        if let Some(files) = list.files {
                            for f in files {
                                entry.files.push(f);
                            }
                        }
                        if let Some(dirs) = &list.dirs {
                            for d in dirs {
                                entry.dirs.push(d.clone());
                            }
                        }
                        if let Some(dirs) = list.dirs {
                            for d in dirs {
                                let p = format!("{}/{}", list.path, d);
                                data.filesync.map.insert(
                                    p,
                                    FileSyncEntry {
                                        step: FileSyncStep::New,
                                        dirs: Vec::new(),
                                        files: Vec::new(),
                                    },
                                );
                            }
                        }
                        data.filesync.timestamp = Some(Instant::now());
                    }
                    Err(e) => {
                        if data.filesync.running {
                            data.filesync.running = false;
                            error!("error during file sync: {} [{}]", e.message, e.code);
                            tx.send(Event::FileSync(SyncStatus::Idle)).unwrap();
                        }
                    }
                },
            }, /*RpcResult::FileList(lst) => {
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
        self.cmd_sender.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
