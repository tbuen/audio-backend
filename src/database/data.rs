use super::DirEntry;
use crate::json::types::FileInfo;
use regex::Regex;

pub struct File {
    name: String,
    synced: bool,
}

pub struct Data {
    current_dir: String,
    file_list: Vec<File>,
}

impl Data {
    pub fn new() -> Self {
        Self {
            current_dir: String::from("/"),
            file_list: Vec::new(),
        }
    }

    pub fn dir_current(&self) -> String {
        let re = Regex::new(r"/([^/]+)/$").unwrap();
        if let Some(c) = re.captures(&self.current_dir) {
            String::from(c.get(1).unwrap().as_str())
        } else {
            String::new()
        }
    }

    pub fn dir_up(&mut self) {
        if self.current_dir != "/" {
            let re = Regex::new(r"/[^/]+/$").unwrap();
            self.current_dir = re.replace(&self.current_dir, "/").into_owned();
        }
    }

    pub fn dir_enter(&mut self, dir: &str) {
        self.current_dir.push_str(dir);
        self.current_dir.push('/');
    }

    pub fn dir_content(&self) -> Vec<DirEntry> {
        let re_dir = Regex::new(&format!("^{}([^/]+)/.*$", &self.current_dir)).unwrap();
        let re_file = Regex::new(&format!(r"^{}([^/]+\.ogg)$", &self.current_dir)).unwrap();

        let mut list: Vec<DirEntry> = self
            .file_list
            .iter()
            .filter_map(|f| {
                if let Some(c) = re_dir.captures(&f.name) {
                    Some(DirEntry::Dir(String::from(c.get(1).unwrap().as_str())))
                } else if let Some(c) = re_file.captures(&f.name) {
                    Some(DirEntry::File(String::from(c.get(1).unwrap().as_str())))
                } else {
                    None
                }
            })
            .collect();

        list.sort();
        list.dedup();

        list
    }

    pub fn clear_file_list(&mut self) {
        self.file_list.clear();
    }

    pub fn append_file_list(&mut self, lst: Vec<String>) {
        for f in lst.into_iter() {
            self.file_list.push(File {
                name: f,
                synced: false,
            });
        }
    }

    pub fn get_unsynced_file(&self) -> Option<String> {
        let f = self
            .file_list
            .iter()
            .filter(|f| f.name.ends_with(".ogg") && !f.synced)
            .next();
        match f {
            Some(f) => Some(String::from(&f.name)),
            None => None,
        }
    }

    pub fn set_file_info(&mut self, info: FileInfo) {
        if let Some(f) = self.file_list.iter_mut().find(|f| f.name == info.filename) {
            f.synced = true;
        }
    }
}
