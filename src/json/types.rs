use serde::Deserialize;

pub const GET_INFO: &str = "get-info";
pub const GET_FILE_LIST: &str = "get-file-list";

#[derive(Deserialize)]
pub struct Info {
    pub project: String,
    pub version: String,
    #[serde(rename = "esp-idf")]
    pub esp_idf: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Result {
    Info(Info),
    FileList(Vec<String>),
}

pub fn check_type(result: &Result, method: &str) -> bool {
    match result {
        Result::Info(_) => return method == GET_INFO,
        Result::FileList(_) => return method == GET_FILE_LIST,
    }
}
