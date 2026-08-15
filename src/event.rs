use crate::Error;

#[derive(Debug, Clone)]
pub enum Event {
    Connected,
    Disconnected,
    InfoConnection(Connection),
    InfoAbout(About),
    InfoMemory(Memory),
    InfoSPIFlash(SPIFlash),
    WiFiScanResult(Vec<Network>),
    WiFiNetworkList(Vec<String>),
    WiFiSetNetwork,
    WiFiDeleteNetwork,
    FileSync(Sync),
    Error(Error),
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

#[derive(Debug, Clone)]
pub enum Sync {
    Running,
    Completed,
}
