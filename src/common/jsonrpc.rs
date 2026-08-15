use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::error;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const RPC_VERSION: &str = "2.0";

#[derive(Debug, Clone)]
pub(crate) enum Error {
    Parsing(String),
    InvalidVersion(String),
    InvalidMessage,
    InvalidId,
}

#[derive(Default)]
pub(crate) struct Handler {
    id: Cell<u32>,
    map: RefCell<HashMap<u32, &'static str>>,
}

pub(crate) enum Message<'a> {
    Response {
        method: &'a str,
        data: Result<Value, ExecError>,
    },
    //Notification {
    //    method: &'a str,
    //    data: Value,
    //},
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecError {
    pub code: i16,
    pub message: String,
}

#[derive(Serialize)]
struct Request<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    id: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response<'a> {
    jsonrpc: &'a str,
    result: Option<Value>,
    error: Option<ExecError>,
    id: Option<u32>,
}

impl Handler {
    pub(crate) fn build_request(&self, method: &'static str, params: Option<Value>) -> String {
        let id = self.id.get() + 1;
        self.id.set(id);
        self.map.borrow_mut().insert(id, method);
        let request = Request {
            jsonrpc: RPC_VERSION,
            method,
            params,
            id,
        };
        serde_json::to_string(&request).unwrap()
    }

    pub(crate) fn parse(&self, msg: &str) -> Result<Message<'_>, Error> {
        match serde_json::from_str::<Response>(msg) {
            Ok(rpc) => {
                if rpc.jsonrpc != RPC_VERSION {
                    Err(Error::InvalidVersion(rpc.jsonrpc.to_owned()))
                } else if let Some(id) = rpc.id
                    && let Some(method) = self.map.borrow_mut().remove(&id)
                {
                    if let Some(error) = rpc.error {
                        Ok(Message::Response {
                            method,
                            data: Err(error),
                        })
                    } else if let Some(result) = rpc.result {
                        Ok(Message::Response {
                            method,
                            data: Ok(result),
                        })
                    } else {
                        Err(Error::InvalidMessage)
                    }
                } else {
                    Err(Error::InvalidId)
                }
            }
            Err(e) => Err(e.into()),
        }
    }
}

impl error::Error for ExecError {}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parsing(s) => write!(f, "could not parse response: {s}"),
            Error::InvalidVersion(s) => write!(f, "invalid version: {s}"),
            Error::InvalidMessage => write!(f, "received neither result nor error"),
            Error::InvalidId => write!(f, "received unrelated message id"),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Parsing(value.to_string())
    }
}
