use std::cell::Cell;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{Builder, JoinHandle};
use std::time::Duration;

use log::{debug, info};

use crate::com::{Com, Event as ComEvent};
use crate::common::access_point::Connector;
use crate::common::jsonrpc;
use crate::event::{
    About, Connection, Event, File, FileSync, Heap, Memory, Network, SPIFlash, TagSync,
};
use crate::filesystem::{FileSystem, PathContent};
use crate::json::{self, Handler, Message, Response};
use crate::sync::{FileSyncState, SyncFiles, SyncTags, TagSyncState};
use crate::{Error, Result};

pub struct Backend {
    handle: Option<JoinHandle<()>>,
    cmd: Sender<Command>,
    evt: Sender<Event>,
    receiver: Cell<Option<Receiver<Event>>>,
    shared: Arc<(Mutex<SharedData>, Condvar)>,
    filesystem: Arc<Mutex<FileSystem>>,
}

#[derive(Debug)]
pub enum ChangeDirectory<'a> {
    ToRoot,
    ToParent,
    ToChild(&'a str),
}

#[derive(Debug)]
pub struct DirectoryContent {
    pub cover: Option<String>,
    pub dirs: Vec<String>,
    pub tracks: Vec<String>,
}

enum Command {
    GetAccessPointMode,
    SetAccessPointMode(bool),
    GetInfoConnection,
    GetInfoAbout,
    GetInfoMemory,
    GetInfoSPIFlash,
    GetWiFiScanResult,
    GetWiFiNetworkList,
    SetWiFiNetwork { ssid: String, key: String },
    DeleteWiFiNetwork { ssid: String },
    Quit,
}

#[derive(Default)]
struct SharedData {
    connected: bool,
    ap_mode: bool,
    filesync: bool,
    tagsync: Option<String>,
}

impl Backend {
    pub fn new() -> Self {
        let (cmd, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        let evt = tx.clone();
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
            cmd,
            evt,
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
        self.cmd.send(Command::GetAccessPointMode).unwrap();
        data = cvar.wait(data).unwrap();
        data.ap_mode
    }

    pub fn set_access_point_mode(&self, auto: bool) {
        self.cmd.send(Command::SetAccessPointMode(auto)).unwrap();
    }

    pub fn get_info_connection(&self) {
        // TODO macro(GetInfoConnection, InfoConnection)
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if data.connected {
            self.cmd.send(Command::GetInfoConnection).unwrap();
        } else {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        }
    }

    pub fn get_info_about(&self) {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if data.connected {
            self.cmd.send(Command::GetInfoAbout).unwrap();
        } else {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        }
    }

    pub fn get_info_memory(&self) {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if data.connected {
            self.cmd.send(Command::GetInfoMemory).unwrap();
        } else {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        }
    }

    pub fn get_info_spiflash(&self) {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if data.connected {
            self.cmd.send(Command::GetInfoSPIFlash).unwrap();
        } else {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        }
    }

    pub fn get_wifi_scan_result(&self) {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if data.connected {
            self.cmd.send(Command::GetWiFiScanResult).unwrap();
        } else {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        }
    }

    pub fn get_wifi_network_list(&self) {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if data.connected {
            self.cmd.send(Command::GetWiFiNetworkList).unwrap();
        } else {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        }
    }

    pub fn set_wifi_network(&self, ssid: String, key: String) {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if data.connected {
            self.cmd
                .send(Command::SetWiFiNetwork { ssid, key })
                .unwrap();
        } else {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        }
    }

    pub fn delete_wifi_network(&self, ssid: String) {
        let (mutex, _) = &*self.shared;
        let data = mutex.lock().unwrap();
        if data.connected {
            self.cmd.send(Command::DeleteWiFiNetwork { ssid }).unwrap();
        } else {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        }
    }

    pub fn sync_files(&self) {
        let (mutex, _) = &*self.shared;
        let mut data = mutex.lock().unwrap();
        if !data.connected {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        } else if data.filesync || data.tagsync.is_some() {
            self.evt.send(Event::Error(Error::Busy)).unwrap();
        } else {
            data.filesync = true;
            self.evt.send(Event::FileSync(FileSync::Started)).unwrap();
        }
    }

    pub fn sync_tags(&self) {
        let (mutex, _) = &*self.shared;
        let mut data = mutex.lock().unwrap();
        if !data.connected {
            self.evt.send(Event::Error(Error::NotConnected)).unwrap();
        } else if data.filesync || data.tagsync.is_some() {
            self.evt.send(Event::Error(Error::Busy)).unwrap();
        } else {
            let fs = self.filesystem.lock().unwrap();
            match fs.current_directory_with_prefix() {
                Ok(dir) => {
                    data.tagsync = Some(dir);
                    self.evt.send(Event::TagSync(TagSync::Started)).unwrap();
                }
                Err(e) => self.evt.send(Event::Error(e)).unwrap(),
            }
        }
    }

    pub fn current_directory(&self) -> Result<Vec<String>> {
        let fs = self.filesystem.lock().unwrap();
        fs.current_directory()
            .map(|path| path.split('/').map(ToOwned::to_owned).collect())
    }

    pub fn change_directory(&self, to: ChangeDirectory) -> Result<()> {
        let mut fs = self.filesystem.lock().unwrap();
        fs.change_directory(to)
    }

    pub fn directory_content(&self) -> Result<DirectoryContent> {
        let fs = self.filesystem.lock().unwrap();
        fs.directory_content().map(Into::into)
    }

    fn thread(
        tx: Sender<Event>,
        rx: Receiver<Command>,
        shared: Arc<(Mutex<SharedData>, Condvar)>,
        filesystem: Arc<Mutex<FileSystem>>,
    ) {
        let com = Com::new();
        let json = Handler::default();
        let (mutex, cvar) = &*shared;
        let mut ap = None;
        let mut filesync = None;
        let mut tagsync = None;

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
                    Command::GetWiFiScanResult => {
                        com.send(json.get_wifi_scan_result());
                    }
                    Command::GetWiFiNetworkList => {
                        com.send(json.get_wifi_network_list());
                    }
                    Command::SetWiFiNetwork { ssid, key } => {
                        com.send(json.set_wifi_network(&ssid, &key));
                    }
                    Command::DeleteWiFiNetwork { ssid } => {
                        com.send(json.delete_wifi_network(&ssid));
                    }
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
                        let mut data = mutex.lock().unwrap();
                        data.connected = true;
                        tx.send(Event::Connected).unwrap();
                    }
                    ComEvent::Disconnected => {
                        info!("Disconnected!");
                        let mut data = mutex.lock().unwrap();
                        data.connected = false;
                        tx.send(Event::Disconnected).unwrap();
                        if data.filesync {
                            filesync.take();
                            tx.send(Event::Error(Error::Disconnected)).unwrap();
                            tx.send(Event::FileSync(FileSync::Aborted)).unwrap();
                            data.filesync = false;
                        }
                        if data.tagsync.is_some() {
                            tagsync.take();
                            tx.send(Event::Error(Error::Disconnected)).unwrap();
                            tx.send(Event::TagSync(TagSync::Aborted)).unwrap();
                            data.tagsync.take();
                        }
                    }
                    ComEvent::Message(msg) => {
                        debug!("Message: {msg}");
                        match json.parse(&msg) {
                            Ok(m) => {
                                Self::handle_message(
                                    m,
                                    &com,
                                    &json,
                                    &tx,
                                    shared.clone(),
                                    filesync.as_mut(),
                                    tagsync.as_mut(),
                                );
                            }
                            Err(e) => tx.send(Event::Error(e.into())).unwrap(),
                        }
                    }
                }
            }

            let mut data = mutex.lock().unwrap();
            if data.filesync {
                let fs = filesync.get_or_insert_with(SyncFiles::start);
                match fs.state() {
                    FileSyncState::Finished => {
                        let mut fs = filesystem.lock().unwrap();
                        fs.rebuild(filesync.take().unwrap());
                        tx.send(Event::FileSync(FileSync::Completed)).unwrap();
                        data.filesync = false;
                    }
                    FileSyncState::Timeout => {
                        filesync.take();
                        tx.send(Event::Error(Error::Timeout)).unwrap();
                        tx.send(Event::FileSync(FileSync::Aborted)).unwrap();
                        data.filesync = false;
                    }
                    FileSyncState::Error(e) => {
                        filesync.take();
                        tx.send(Event::Error(e.into())).unwrap();
                        tx.send(Event::FileSync(FileSync::Aborted)).unwrap();
                        data.filesync = false;
                    }
                    FileSyncState::NextToSync(list) => {
                        if list.is_empty() {
                            com.send(json.get_file_list(None));
                        } else {
                            for path in list {
                                com.send(json.get_file_list(Some(path)));
                            }
                        }
                    }
                    FileSyncState::Waiting => (),
                }
            }
            if let Some(dir) = &data.tagsync {
                let ts = tagsync.get_or_insert_with(|| {
                    let fs = filesystem.lock().unwrap();
                    SyncTags::start(dir, fs)
                });
                match ts.state() {
                    TagSyncState::Finished => {
                        //let mut fs = filesystem.lock().unwrap();
                        //fs.rebuild(tagsync.take().unwrap());
                        tagsync.take();
                        tx.send(Event::TagSync(TagSync::Completed)).unwrap();
                        data.tagsync.take();
                    }
                    TagSyncState::Timeout => {
                        tagsync.take();
                        tx.send(Event::Error(Error::Timeout)).unwrap();
                        tx.send(Event::TagSync(TagSync::Aborted)).unwrap();
                        data.tagsync.take();
                    }
                    TagSyncState::Error(e) => {
                        tagsync.take();
                        tx.send(Event::Error(e.into())).unwrap();
                        tx.send(Event::TagSync(TagSync::Aborted)).unwrap();
                        data.tagsync.take();
                    }
                    TagSyncState::NextToSync(list, n_synced, n_total) => {
                        tx.send(Event::TagSync(TagSync::Step(n_synced, n_total)))
                            .unwrap();
                        for file in list {
                            com.send(json.get_track_info(file));
                        }
                    }
                    TagSyncState::Waiting => (),
                }
            }
        }
        debug!("quit");
    }

    fn handle_message(
        msg: Message,
        _com: &Com,
        _json: &Handler,
        tx: &Sender<Event>,
        //mut _data: MutexGuard<'_, SharedData>,
        _shared: Arc<(Mutex<SharedData>, Condvar)>,
        //database: &Database,
        fs: Option<&mut SyncFiles>,
        ts: Option<&mut SyncTags>,
    ) {
        match msg {
            Message::Response(resp) => match resp {
                Response::InfoConnection(res) => match res {
                    Ok(connection) => {
                        let evt = Event::InfoConnection(Connection {
                            mode: connection.mode,
                        });
                        tx.send(evt).unwrap();
                    }
                    Err(e) => tx.send(Event::Error(e.into())).unwrap(),
                },
                Response::InfoAbout(res) => match res {
                    Ok(about) => {
                        let evt = Event::InfoAbout(About {
                            project: about.project,
                            version: about.version,
                            esp_idf: about.esp_idf,
                        });
                        tx.send(evt).unwrap();
                    }
                    Err(e) => tx.send(Event::Error(e.into())).unwrap(),
                },
                Response::InfoMemory(res) => match res {
                    Ok(info) => {
                        let evt = Event::InfoMemory(Memory {
                            heap: Heap {
                                allocated: info.heap.allocated,
                                free: info.heap.free,
                                minimum_free: info.heap.minimum_free,
                            },
                        });
                        tx.send(evt).unwrap();
                    }
                    Err(e) => tx.send(Event::Error(e.into())).unwrap(),
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
                        let evt = Event::InfoSPIFlash(SPIFlash {
                            total: info.total,
                            free: info.free,
                            files,
                        });
                        tx.send(evt).unwrap();
                    }
                    Err(e) => tx.send(Event::Error(e.into())).unwrap(),
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
                        let evt = Event::WiFiScanResult(networks);
                        tx.send(evt).unwrap();
                    }
                    Err(e) => tx.send(Event::Error(e.into())).unwrap(),
                },
                Response::NetworkList(res) => match res {
                    Ok(list) => {
                        let mut networks = Vec::new();
                        for e in list {
                            networks.push(e.ssid);
                        }
                        let evt = Event::WiFiNetworkList(networks);
                        tx.send(evt).unwrap();
                    }
                    Err(e) => tx.send(Event::Error(e.into())).unwrap(),
                },
                Response::SetNetwork(res) => match res {
                    Ok(_empty) => {
                        let evt = Event::WiFiSetNetwork;
                        tx.send(evt).unwrap();
                    }
                    Err(e) => tx.send(Event::Error(e.into())).unwrap(),
                },
                Response::DeleteNetwork(res) => match res {
                    Ok(_empty) => {
                        let evt = Event::WiFiDeleteNetwork;
                        tx.send(evt).unwrap();
                    }
                    Err(e) => tx.send(Event::Error(e.into())).unwrap(),
                },
                Response::FileList(resp) => {
                    if let Some(fs) = fs {
                        fs.insert_response(resp);
                    }
                }
                Response::TrackInfo(resp) => {
                    if let Some(ts) = ts {
                        ts.insert_response(resp);
                    }
                }
            },
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
        self.cmd.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}

impl From<&PathContent> for DirectoryContent {
    fn from(value: &PathContent) -> Self {
        Self {
            cover: value.cover.clone(),
            dirs: value.dirs.clone(),
            tracks: value.tracks.clone(),
        }
    }
}

impl From<json::Error> for Error {
    fn from(value: json::Error) -> Self {
        Error::RPC(value.to_string())
    }
}

impl From<jsonrpc::ExecError> for Error {
    fn from(value: jsonrpc::ExecError) -> Self {
        Error::Remote {
            code: value.code,
            message: value.message,
        }
    }
}
