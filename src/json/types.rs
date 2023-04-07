use serde::{Deserialize, Serialize};

pub const GET_VERSION: &str = "get-version";
pub const GET_FILE_LIST: &str = "get-file-list";
pub const GET_FILE_INFO: &str = "get-file-info";

#[derive(Serialize)]
pub struct ParamGetFileList {
    pub start: bool,
}

#[derive(Serialize)]
pub struct ParamGetFileInfo {
    pub filename: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Params {
    FileList(ParamGetFileList),
    FileInfo(ParamGetFileInfo),
}

#[derive(Deserialize)]
pub struct Version {
    pub project: String,
    pub version: String,
    #[serde(rename = "esp-idf")]
    pub esp_idf: String,
}

#[derive(Deserialize)]
pub struct FileList {
    pub first: bool,
    pub last: bool,
    pub files: Vec<String>,
}

#[derive(Deserialize)]
pub struct FileInfo {
    pub filename: String,
    pub genre: String,
    pub artist: String,
    pub album: String,
    pub title: String,
    pub date: Option<u16>,
    pub track: u16,
    pub duration: u16,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum RpcResult {
    Version(Version),
    FileList(FileList),
    FileInfo(FileInfo),
}

#[derive(Default)]
pub enum ErrReq {
    #[default]
    Unknown,
    Version,
    FileList,
    FileInfo,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcError {
    #[serde(skip_deserializing)]
    pub request: ErrReq,
    #[serde(skip_deserializing)]
    pub method: String,
    pub code: i16,
    pub message: String,
}

pub fn check_type(result: &RpcResult, method: &str) -> bool {
    match result {
        RpcResult::Version(_) => return method == GET_VERSION,
        RpcResult::FileList(_) => return method == GET_FILE_LIST,
        RpcResult::FileInfo(_) => return method == GET_FILE_INFO,
    }
}
