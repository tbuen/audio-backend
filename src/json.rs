pub(crate) mod types;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub(crate) use self::types::{ErrReq, Params, RpcError, RpcResult};

const RPC_VERSION: &str = "2.0";

// TODO Notifications, Params

pub(crate) struct Rpc {
    id: Cell<u32>,
    map: RefCell<HashMap<u32, &'static str>>,
}

pub(crate) enum Message {
    Response(Result<RpcResult, RpcError>),
    //Notification,
}

#[derive(Serialize)]
struct Request<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Params>,
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

impl Rpc {
    pub(crate) fn new() -> Self {
        Self {
            id: Cell::new(0),
            map: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn get_info_con(&self) -> String {
        let rpc = self.request(types::GET_INFO_CON, None);
        serde_json::to_string(&rpc).unwrap()
    }

    pub(crate) fn get_scan_result(&self) -> String {
        let rpc = self.request(types::GET_WIFI_SCAN_RESULT, None);
        serde_json::to_string(&rpc).unwrap()
    }

    pub(crate) fn get_network_list(&self) -> String {
        let rpc = self.request(types::GET_WIFI_NETWORK_LIST, None);
        serde_json::to_string(&rpc).unwrap()
    }

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

    pub(crate) fn parse(&self, msg: &str) -> Option<Message> {
        // TODO parse notification, maybe as untagged enum :-)
        match serde_json::from_str::<Response<'_>>(msg) {
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

    fn check_id(&self, res: &RpcResult, id: Option<u32>) -> bool {
        println!("Size of map: {}", self.map.borrow().len());
        match id {
            Some(id) => match self.map.borrow_mut().remove(&id) {
                Some(m) => {
                    println!("Method found for id {}: {}", id, m);
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

    fn check_error(&self, e: &mut RpcError, id: Option<u32>) -> bool {
        println!("Size of map: {}", self.map.borrow().len());
        match id {
            Some(id) => match self.map.borrow_mut().remove(&id) {
                Some(m) => {
                    println!("Method found for id {}: {}", id, m);
                    e.method = String::from(m);
                    match m {
                        // TODO move to types.rs
                        types::GET_INFO_CON => {
                            e.request = ErrReq::InfoCon;
                            return true;
                        }
                        /*types::GET_FILE_LIST => {
                            e.request = ErrReq::FileList;
                            return true;
                        }
                        types::GET_FILE_INFO => {
                            e.request = ErrReq::FileInfo;
                            return true;
                        }*/
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

    fn request(&self, method: &'static str, params: Option<Params>) -> Request<'static> {
        let id = self.id.get() + 1;
        self.id.set(id);
        self.map.borrow_mut().insert(id, method);
        Request {
            jsonrpc: RPC_VERSION,
            method,
            params,
            id,
        }
    }
}
