use log::error;
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
}

//#[derive(Deserialize)]
//#[serde(untagged)]
//pub(crate) enum RpcResult {
//   InfoCon(InfoCon),
//  ScanResult(Vec<Network>),
// NetworkList(Vec<String>),
//FileList(FileList),
//FileInfo(FileInfo),
//}

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
    pub dirs: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
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

    pub(crate) fn parse(&self, msg: &str) -> Option<Message> {
        /*Response {
            method: &'a str,
            data: Result<Value, Error>,
        },
        Notification {
            method: &'a str,
            data: Value,
        },*/

        /*InfoCon(Result<InfoCon, Error>),
        ScanResult(Result<Vec<Network>, Error>),
        NetworkList(Result<Vec<String>, Error>),*/

        if let Some(msg) = self.jsonrpc.parse(msg) {
            match msg {
                jsonrpc::Message::Response { method, data } => match method {
                    GET_INFO_CONNECTION => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::InfoConnection(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::InfoConnection(Err(e)))),
                    },
                    GET_INFO_ABOUT => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::InfoAbout(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::InfoAbout(Err(e)))),
                    },
                    GET_INFO_MEMORY => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::InfoMemory(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::InfoMemory(Err(e)))),
                    },
                    GET_INFO_SPIFLASH => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::InfoSPIFlash(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::InfoSPIFlash(Err(e)))),
                    },
                    GET_WIFI_SCAN_RESULT => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::ScanResult(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::ScanResult(Err(e)))),
                    },
                    GET_WIFI_NETWORK_LIST => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::NetworkList(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::NetworkList(Err(e)))),
                    },
                    SET_WIFI_NETWORK => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::SetNetwork(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::SetNetwork(Err(e)))),
                    },
                    DELETE_WIFI_NETWORK => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::DeleteNetwork(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::DeleteNetwork(Err(e)))),
                    },
                    GET_FILE_LIST => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::FileList(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::FileList(Err(e)))),
                    },
                    _ => {
                        error!("Received response with unknown method: {method}");
                        None
                    }
                },
            }
        } else {
            None
        }
    }
}

/*use serde::{Deserialize, Serialize};

//pub(crate) const GET_FILE_LIST: &str = "get-file-list";
//pub(crate) const GET_FILE_INFO: &str = "get-file-info";


//#[derive(Serialize)]
//pub(crate) struct ParamGetFileList {
//    pub start: bool,
//}

//#[derive(Serialize)]
//pub(crate) struct ParamGetFileInfo {
//    pub filename: String,
//}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Params {
    //FileList(ParamGetFileList),
    //FileInfo(ParamGetFileInfo),
}

//#[derive(Deserialize)]
//pub(crate) struct FileList {
//    pub first: bool,
//    pub last: bool,
//    pub files: Vec<String>,
//}

//#[derive(Deserialize)]
//pub(crate) struct FileInfo {
//    pub filename: String,
//    pub genre: String,
//    pub artist: String,
//    pub album: String,
//    pub title: String,
//    pub date: Option<u16>,
//    pub track: u16,
//    pub duration: u16,
//}

*/

/*pub(crate) fn get_file_list(&self, start: bool) -> String {
    let rpc = self.request(
        types::GET_FILE_LIST,
        Some(Params::FileList(ParamGetFileList { start })),
    );
    serde_json::to_string(&rpc).unwrap()
}

pub(crate) fn get_file_info(&self, filename: String) -> String {
    let rpc = self.request(
        types::GET_FILE_INFO,
        Some(Params::FileInfo(ParamGetFileInfo { filename })),
    );
    serde_json::to_string(&rpc).unwrap()
}*/
