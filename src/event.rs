#[derive(Debug)]
pub struct Version {
    pub project: String,
    pub version: String,
    pub esp_idf: String,
}

pub enum Reload {
    Start,
    Step,
    Stop,
}

pub enum Event {
    Connected,
    Disconnected,
    Version(Version),
    Reload(Reload),
    Error(String),
}
