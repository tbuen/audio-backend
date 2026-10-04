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
const PLAY_TRACK: &str = "play-track";
const STOP_PLAY: &str = "stop-play";
const SET_VOLUME: &str = "set-volume";

const VOLUME: &str = "volume";

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
    Notification(Notification),
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
    PlayTrack(Result<Empty, jsonrpc::ExecError>),
    StopPlay(Result<Empty, jsonrpc::ExecError>),
    SetVolume(Result<Empty, jsonrpc::ExecError>),
}

pub(crate) enum Notification {
    Volume(Volume),
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

#[derive(Deserialize)]
pub(crate) struct Volume {
    pub left: i32,
    pub right: i32,
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

    pub(crate) fn play_track(&self, file: &str) -> String {
        let params = json!({"file":file});
        self.jsonrpc.build_request(PLAY_TRACK, Some(params))
    }

    pub(crate) fn stop_play(&self) -> String {
        self.jsonrpc.build_request(STOP_PLAY, None)
    }

    pub(crate) fn set_volume(&self, left: i32, right: i32) -> String {
        let params = json!({"left":left,"right":right});
        self.jsonrpc.build_request(SET_VOLUME, Some(params))
    }

    pub(crate) fn parse(&self, msg: &str) -> Result<Message, Error> {
        macro_rules! parse_resp {
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
        macro_rules! parse_ntfn {
            ($params:expr, $type:path) => {
                match $params {
                    Some(v) => match serde_json::from_value(v) {
                        Ok(o) => Ok(Message::Notification($type(o))),
                        Err(e) => Err(e.into()),
                    },
                    None => Err(Error::Parsing("missing parameters".into())),
                }
            };
        }

        match self.jsonrpc.parse(msg) {
            Ok(msg) => match msg {
                jsonrpc::Message::Response { method, data } => match method {
                    GET_INFO_CONNECTION => parse_resp!(data, Response::InfoConnection),
                    GET_INFO_ABOUT => parse_resp!(data, Response::InfoAbout),
                    GET_INFO_MEMORY => parse_resp!(data, Response::InfoMemory),
                    GET_INFO_SPIFLASH => parse_resp!(data, Response::InfoSPIFlash),
                    GET_WIFI_SCAN_RESULT => parse_resp!(data, Response::ScanResult),
                    GET_WIFI_NETWORK_LIST => parse_resp!(data, Response::NetworkList),
                    SET_WIFI_NETWORK => parse_resp!(data, Response::SetNetwork),
                    DELETE_WIFI_NETWORK => parse_resp!(data, Response::DeleteNetwork),
                    GET_FILE_LIST => parse_resp!(data, Response::FileList),
                    GET_TRACK_INFO => parse_resp!(data, Response::TrackInfo),
                    PLAY_TRACK => parse_resp!(data, Response::PlayTrack),
                    STOP_PLAY => parse_resp!(data, Response::StopPlay),
                    SET_VOLUME => parse_resp!(data, Response::SetVolume),
                    _ => Err(Error::UnknownMethod(method.to_owned())),
                },
                jsonrpc::Message::Notification { method, params } => match method {
                    VOLUME => parse_ntfn!(params, Notification::Volume),
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
            Error::Parsing(s) => write!(f, "could not parse message from device: {s}"),
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
