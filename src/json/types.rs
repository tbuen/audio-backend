use serde::Deserialize;

pub const GET_VERSION: &str = "get-version";
pub const GET_FILE_LIST: &str = "get-file-list";

#[derive(Deserialize)]
pub struct Version {
    pub project: String,
    pub version: String,
    #[serde(rename = "esp-idf")]
    pub esp_idf: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum RpcResult {
    Version(Version),
    FileList(Vec<String>),
}

pub fn check_type(result: &RpcResult, method: &str) -> bool {
    match result {
        RpcResult::Version(_) => return method == GET_VERSION,
        RpcResult::FileList(_) => return method == GET_FILE_LIST,
    }
}
