use serde::{Deserialize, Serialize};

pub(crate) const GET_INFO_CON: &str = "get-info-con";
pub(crate) const GET_WIFI_SCAN_RESULT: &str = "get-wifi-scan-result";
pub(crate) const GET_WIFI_NETWORK_LIST: &str = "get-wifi-network-list";
//pub(crate) const GET_FILE_LIST: &str = "get-file-list";
//pub(crate) const GET_FILE_INFO: &str = "get-file-info";

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum RpcResult {
    InfoCon(InfoCon),
    ScanResult(Vec<Network>),
    NetworkList(Vec<String>),
    //FileList(FileList),
    //FileInfo(FileInfo),
}

#[derive(Deserialize)]
pub(crate) struct InfoCon {
    pub mode: String,
    pub about: About,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RpcError {
    #[serde(skip_deserializing)]
    pub request: ErrReq,
    #[serde(skip_deserializing)]
    pub method: String,
    pub code: i16,
    pub message: String,
}

#[derive(Default)]
pub(crate) enum ErrReq {
    #[default]
    Unknown,
    InfoCon,
    //FileList,
    //FileInfo,
}

pub(crate) fn check_type(result: &RpcResult, method: &str) -> bool {
    match result {
        RpcResult::InfoCon(_) => return method == GET_INFO_CON,
        RpcResult::ScanResult(_) => return method == GET_WIFI_SCAN_RESULT,
        RpcResult::NetworkList(_) => return method == GET_WIFI_NETWORK_LIST,
        //RpcResult::FileList(_) => return method == GET_FILE_LIST,
        //RpcResult::FileInfo(_) => return method == GET_FILE_INFO,
    }
}
