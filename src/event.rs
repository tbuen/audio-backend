#[derive(Debug)]
pub struct Version {
    pub project: String,
    pub version: String,
    pub esp_idf: String,
}

pub enum Event {
    Connected,
    Version(Version),
    Synchronized,
    Disconnected,
}
