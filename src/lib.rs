mod backend;
mod com;
mod common;
mod event;
mod filesystem;
mod json;
mod sync;

pub use crate::backend::*;
pub use crate::event::*;

use std::error;
use std::fmt;
use std::result;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("VERSION");

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    NotConnected,
    AlreadyRunning,
    Timeout,
    Disconnected,
    FilesNotSynced,
    DirectoryNotFound,
    FileNotFound,
    RPC(String),
    Remote { code: i16, message: String },
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotConnected => write!(f, "not connected to device"),
            Error::AlreadyRunning => write!(f, "process is already running"),
            Error::Timeout => write!(f, "timeout"),
            Error::Disconnected => write!(f, "disconnected from device"),
            Error::FilesNotSynced => write!(f, "files are not synchronized"),
            Error::DirectoryNotFound => write!(f, "directory not found"),
            Error::FileNotFound => write!(f, "file not found"),
            Error::RPC(s) => write!(f, "{s}"),
            Error::Remote { code, message } => write!(f, "device returned: ({code}) {message}"),
        }
    }
}
