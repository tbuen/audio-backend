use std::error;
use std::fmt;

use serde::Deserialize;
use serde_json::json;

use crate::common::jsonrpc;

const GET_INFO_CONNECTION: &str = "get-info-connection";
const GET_INFO_ABOUT: &str = "get-info-about";
const GET_INFO_MEMORY: &str = "get-info-memory";
const GET_INFO_SPIFLASH: &str = "get-info-spiflash";
const GET_WIFI_SCAN_RESULT: &str = "get-wifi-scan-result";
const GET_WIFI_NETWORK_LIST: &str = "get-wifi-network-list";
const SET_WIFI_NETWORK: &str = "set-wifi-network";
const DELETE_WIFI_NETWORK: &str = "delete-wifi-network";
const GET_FILE_LIST: &str = "get-file-list";
const GET_TRACK_INFO: &str = "get-track-info";

#[derive(Debug, Clone)]
pub(crate) enum Error {
    JsonRpc(String),
    Parsing(String),
    UnknownMethod(String),
}

#[derive(Default)]
pub(crate) struct Handler {
    jsonrpc: jsonrpc::Handler,
}

pub(crate) enum Message {
    Response(Response),
    //Notification,
}

pub(crate) enum Response {
    InfoConnection(Result<Connection, jsonrpc::ExecError>),
    InfoAbout(Result<About, jsonrpc::ExecError>),
    InfoMemory(Result<Memory, jsonrpc::ExecError>),
    InfoSPIFlash(Result<SPIFlash, jsonrpc::ExecError>),
    ScanResult(Result<Vec<ScannedNetwork>, jsonrpc::ExecError>),
    NetworkList(Result<Vec<StoredNetwork>, jsonrpc::ExecError>),
    SetNetwork(Result<Empty, jsonrpc::ExecError>),
    DeleteNetwork(Result<Empty, jsonrpc::ExecError>),
    FileList(Result<FileList, jsonrpc::ExecError>),
    TrackInfo(Result<TrackInfo, jsonrpc::ExecError>),
}

#[derive(Deserialize)]
pub(crate) struct Connection {
    pub mode: String,
}

#[derive(Deserialize)]
pub(crate) struct About {
    pub project: String,
    pub version: String,
    #[serde(rename = "esp-idf")]
    pub esp_idf: String,
}

#[derive(Deserialize)]
pub(crate) struct Memory {
    pub heap: Heap,
}

#[derive(Deserialize)]
pub(crate) struct Heap {
    pub allocated: u32,
    pub free: u32,
    #[serde(rename = "minimum-free")]
    pub minimum_free: u32,
}

#[derive(Deserialize)]
pub(crate) struct SPIFlash {
    pub total: u32,
    pub free: u32,
    pub files: Vec<File>,
}

#[derive(Deserialize)]
pub(crate) struct File {
    pub name: String,
    #[serde(rename = "content-type")]
    pub content_type: String,
    pub size: u32,
    pub md5: String,
}

#[derive(Deserialize)]
pub(crate) struct ScannedNetwork {
    pub ssid: String,
    pub rssi: i8,
}

#[derive(Deserialize)]
pub(crate) struct StoredNetwork {
    pub ssid: String,
}

#[derive(Deserialize)]
pub(crate) struct FileList {
    pub path: String,
    pub cover: Option<String>,
    pub dirs: Option<Vec<String>>,
    pub tracks: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub(crate) struct TrackInfo {
    pub file: String,
    pub genre: String,
    pub artist: String,
    pub album: String,
    pub title: String,
    pub date: Option<u16>,
    pub track: u16,
    pub duration: u16,
}

#[allow(clippy::empty_structs_with_brackets)]
#[derive(Deserialize)]
pub(crate) struct Empty {}

impl Handler {
    pub(crate) fn get_info_connection(&self) -> String {
        self.jsonrpc.build_request(GET_INFO_CONNECTION, None)
    }

    pub(crate) fn get_info_about(&self) -> String {
        self.jsonrpc.build_request(GET_INFO_ABOUT, None)
    }

    pub(crate) fn get_info_memory(&self) -> String {
        self.jsonrpc.build_request(GET_INFO_MEMORY, None)
    }

    pub(crate) fn get_info_spiflash(&self) -> String {
        self.jsonrpc.build_request(GET_INFO_SPIFLASH, None)
    }

    pub(crate) fn get_wifi_scan_result(&self) -> String {
        self.jsonrpc.build_request(GET_WIFI_SCAN_RESULT, None)
    }

    pub(crate) fn get_wifi_network_list(&self) -> String {
        self.jsonrpc.build_request(GET_WIFI_NETWORK_LIST, None)
    }

    pub(crate) fn set_wifi_network(&self, ssid: &str, key: &str) -> String {
        let params = json!({"ssid":ssid,"key":key});
        self.jsonrpc.build_request(SET_WIFI_NETWORK, Some(params))
    }

    pub(crate) fn delete_wifi_network(&self, ssid: &str) -> String {
        let params = json!({"ssid":ssid});
        self.jsonrpc
            .build_request(DELETE_WIFI_NETWORK, Some(params))
    }

    pub(crate) fn get_file_list(&self, path: Option<&str>) -> String {
        let params = path.map(|p| json!({"path":p}));
        self.jsonrpc.build_request(GET_FILE_LIST, params)
    }

    pub(crate) fn get_track_info(&self, file: &str) -> String {
        let params = json!({"file":file});
        self.jsonrpc.build_request(GET_TRACK_INFO, Some(params))
    }

    pub(crate) fn parse(&self, msg: &str) -> Result<Message, Error> {
        macro_rules! parse {
            ($data:expr, $type:path) => {
                match $data {
                    Ok(v) => match serde_json::from_value(v) {
                        Ok(o) => Ok(Message::Response($type(Ok(o)))),
                        Err(e) => Err(e.into()),
                    },
                    Err(e) => Ok(Message::Response($type(Err(e)))),
                }
            };
        }

        match self.jsonrpc.parse(msg) {
            Ok(msg) => match msg {
                jsonrpc::Message::Response { method, data } => match method {
                    GET_INFO_CONNECTION => parse!(data, Response::InfoConnection),
                    GET_INFO_ABOUT => parse!(data, Response::InfoAbout),
                    GET_INFO_MEMORY => parse!(data, Response::InfoMemory),
                    GET_INFO_SPIFLASH => parse!(data, Response::InfoSPIFlash),
                    GET_WIFI_SCAN_RESULT => parse!(data, Response::ScanResult),
                    GET_WIFI_NETWORK_LIST => parse!(data, Response::NetworkList),
                    SET_WIFI_NETWORK => parse!(data, Response::SetNetwork),
                    DELETE_WIFI_NETWORK => parse!(data, Response::DeleteNetwork),
                    GET_FILE_LIST => parse!(data, Response::FileList),
                    GET_TRACK_INFO => parse!(data, Response::TrackInfo),
                    _ => Err(Error::UnknownMethod(method.to_owned())),
                },
            },
            Err(e) => Err(e.into()),
        }
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::JsonRpc(s) => write!(f, "jsonrpc: {s}"),
            Error::Parsing(s) => write!(f, "could not parse response: {s}"),
            Error::UnknownMethod(s) => write!(f, "received unknown method: {s}"),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Parsing(value.to_string())
    }
}

impl From<jsonrpc::Error> for Error {
    fn from(value: jsonrpc::Error) -> Self {
        Error::JsonRpc(value.to_string())
    }
}
