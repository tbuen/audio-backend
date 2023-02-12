use serde::Deserialize;

pub const GET_INFO: &str = "get-info";

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
}

pub fn check_type(result: &Result, method: &str) -> bool {
    match result {
        Result::Info(_) => return method == GET_INFO,
    }
}
