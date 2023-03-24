use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use types::{ErrReq, RpcError, RpcResult};

mod types;

const RPC_VERSION: &str = "2.0";

// TODO Notifications, Params

#[derive(Serialize)]
struct Request<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<String>,
    id: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response<'a> {
    jsonrpc: &'a str,
    result: Option<RpcResult>,
    error: Option<RpcError>,
    id: Option<u32>,
}

/*#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Notification {
    jsonrpc: String,
    method: String,
    params: Option<String>,
}*/

pub enum Message {
    Response(Result<RpcResult, RpcError>),
    //Notification,
}

pub struct Rpc {
    id: u32,
    map: HashMap<u32, &'static str>,
}

impl Rpc {
    pub fn new() -> Self {
        Self {
            id: 0,
            map: HashMap::new(),
        }
    }

    pub fn get_version(&mut self) -> String {
        let rpc = self.request(types::GET_VERSION, None);
        serde_json::to_string(&rpc).unwrap()
    }

    pub fn get_file_list(&mut self) -> String {
        let rpc = self.request(types::GET_FILE_LIST, None);
        serde_json::to_string(&rpc).unwrap()
    }

    pub fn parse(&mut self, msg: &str) -> Option<Message> {
        // TODO parse notification, maybe as untagged enum :-)
        match serde_json::from_str::<Response>(msg) {
            Ok(rpc) => {
                if rpc.jsonrpc == RPC_VERSION {
                    if let Some(mut e) = rpc.error {
                        println!("Received RPC error {}: {}", e.code, e.message);
                        if self.check_error(&mut e, rpc.id) {
                            return Some(Message::Response(Err(e)));
                        }
                    } else if let Some(r) = rpc.result {
                        if self.check_id(&r, rpc.id) {
                            return Some(Message::Response(Ok(r)));
                        }
                    } else {
                        println!("Received neither result nor error...");
                    }
                }
            }
            Err(e) => {
                println!("Could not parse jsonrpc: {}", e)
            }
        }
        None
    }

    fn check_id(&mut self, res: &RpcResult, id: Option<u32>) -> bool {
        match id {
            Some(id) => match self.map.remove(&id) {
                Some(m) => {
                    println!("Method found for id {}: {}", id, m);
                    println!("Size of map: {}", self.map.len());
                    if types::check_type(res, m) {
                        println!("type check ok");
                        return true;
                    } else {
                        println!("type check failed");
                    }
                }
                None => {
                    println!("ID missing in map!");
                }
            },
            None => {
                println!("ID missing in response!");
            }
        }
        false
    }

    fn check_error(&mut self, e: &mut RpcError, id: Option<u32>) -> bool {
        match id {
            Some(id) => match self.map.remove(&id) {
                Some(m) => {
                    println!("Method found for id {}: {}", id, m);
                    println!("Size of map: {}", self.map.len());
                    e.method = String::from(m);
                    match m {
                        types::GET_VERSION => {
                            e.request = ErrReq::Version;
                            return true;
                        }
                        types::GET_FILE_LIST => {
                            e.request = ErrReq::FileList;
                            return true;
                        }
                        _ => {}
                    }
                }
                None => {
                    println!("ID missing in map!");
                }
            },
            None => {
                println!("ID missing in response!");
            }
        }
        false
    }

    fn request(&mut self, method: &'static str, params: Option<String>) -> Request {
        self.id += 1;
        self.map.insert(self.id, method);
        Request {
            jsonrpc: RPC_VERSION,
            method: method,
            params,
            id: self.id,
        }
    }
}
