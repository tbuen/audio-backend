use log::error;
use serde::Deserialize;

use crate::common::jsonrpc;

const GET_INFO_CON: &str = "get-info-con";
const GET_INFO_ABOUT: &str = "get-info-about";
const GET_WIFI_SCAN_RESULT: &str = "get-wifi-scan-result";
const GET_WIFI_NETWORK_LIST: &str = "get-wifi-network-list";
const SET_WIFI_NETWORK: &str = "set-wifi-network";
const DELETE_WIFI_NETWORK: &str = "delete-wifi-network";

#[derive(Default)]
pub(crate) struct Handler {
    jsonrpc: jsonrpc::Handler,
}

pub(crate) enum Message {
    Response(Response),
    //Notification,
}

pub(crate) enum Response {
    InfoCon(Result<Con, jsonrpc::ExecError>),
    InfoAbout(Result<About, jsonrpc::ExecError>),
    ScanResult(Result<Vec<Network>, jsonrpc::ExecError>),
    NetworkList(Result<Vec<String>, jsonrpc::ExecError>),
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
pub(crate) struct Con {
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
pub(crate) struct Network {
    pub ssid: String,
    pub rssi: i8,
}

impl Handler {
    pub(crate) fn get_info_con(&self) -> String {
        self.jsonrpc.build_request(GET_INFO_CON)
    }

    pub(crate) fn get_info_about(&self) -> String {
        self.jsonrpc.build_request(GET_INFO_ABOUT)
    }

    pub(crate) fn get_wifi_scan_result(&self) -> String {
        self.jsonrpc.build_request(GET_WIFI_SCAN_RESULT)
    }

    pub(crate) fn get_wifi_network_list(&self) -> String {
        self.jsonrpc.build_request(GET_WIFI_NETWORK_LIST)
    }

    pub(crate) fn set_wifi_network(&self, _ssid: &str, _key: &str) -> String {
        self.jsonrpc.build_request(SET_WIFI_NETWORK)
    }

    pub(crate) fn delete_wifi_network(&self, _ssid: &str) -> String {
        self.jsonrpc.build_request(DELETE_WIFI_NETWORK)
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
                    GET_INFO_CON => match data {
                        Ok(v) => match serde_json::from_value(v) {
                            Ok(o) => Some(Message::Response(Response::InfoCon(Ok(o)))),
                            Err(e) => {
                                error!("Could not parse response: {e}");
                                None
                            }
                        },
                        Err(e) => Some(Message::Response(Response::InfoCon(Err(e)))),
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
