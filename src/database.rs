use super::Command;
use crate::json::types::FileInfo;
use data::Data;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

mod data;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DirEntry {
    Dir(String),
    File(String),
}

#[derive(Clone)]
pub struct Database {
    data: Arc<Mutex<Data>>,
    sender: Sender<Command>,
}

impl Database {
    pub(super) fn new(sender: Sender<Command>) -> Self {
        Self {
            data: Arc::new(Mutex::new(Data::new())),
            sender,
        }
    }

    pub(super) fn save(&self) {
        let data = self.data.lock().unwrap();
        data.save();
    }

    pub fn resync(&self) {
        self.sender.send(Command::Resync).unwrap();
    }

    pub fn dir_current(&self) -> String {
        let data = self.data.lock().unwrap();
        data.dir_current()
    }

    pub fn dir_up(&self) {
        let mut data = self.data.lock().unwrap();
        data.dir_up();
    }

    pub fn dir_enter(&self, dir: &str) {
        let mut data = self.data.lock().unwrap();
        data.dir_enter(dir);
    }

    pub fn dir_content(&self) -> Vec<DirEntry> {
        let data = self.data.lock().unwrap();
        data.dir_content()
    }

    pub(super) fn update_file_list(&self, lst: Vec<String>, last: bool) {
        let mut data = self.data.lock().unwrap();
        data.update_file_list(lst, last);
    }

    pub(super) fn get_unsynced_file(&self) -> Option<String> {
        let data = self.data.lock().unwrap();
        data.get_unsynced_file()
    }

    pub(super) fn set_file_info(&self, info: FileInfo) {
        let mut data = self.data.lock().unwrap();
        data.set_file_info(info);
    }

    pub(super) fn sync_stats(&self) -> (usize, usize) {
        let data = self.data.lock().unwrap();
        data.sync_stats()
    }
}
