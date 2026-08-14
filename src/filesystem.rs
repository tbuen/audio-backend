use std::collections::HashMap;

use crate::backend::ChangeDirectory;
use crate::sync;
use crate::{Error, Result};

#[derive(Default)]
pub(crate) struct FileSystem {
    prefix: String,
    current: String,
    map: HashMap<String, PathContent>,
}

pub(crate) struct PathContent {
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}

impl FileSystem {
    pub(crate) fn new() -> Self {
        FileSystem::default()
    }

    pub(crate) fn rebuild(&mut self, sync: sync::Files) {
        self.prefix = sync.prefix.unwrap();
        self.current.clear();
        self.map = sync.map.into_iter().map(|(k, v)| (k, v.into())).collect();
    }

    pub(crate) fn current_directory(&self) -> Result<&str> {
        if self.prefix.is_empty() {
            Err(Error::FilesNotSynced)
        } else {
            Ok(&self.current)
        }
    }

    pub(crate) fn change_directory(&mut self, to: ChangeDirectory) -> Result<()> {
        if self.prefix.is_empty() {
            Err(Error::FilesNotSynced)
        } else {
            match to {
                ChangeDirectory::ToRoot => {
                    self.current.clear();
                    Ok(())
                }
                ChangeDirectory::ToParent => {
                    if let Some((s, _)) = self.current.rsplit_once('/') {
                        self.current = s.to_owned();
                        Ok(())
                    } else {
                        Err(Error::FileNotFound)
                    }
                }
                ChangeDirectory::ToChild(c) => {
                    let p = format!("{}{}/{c}", self.prefix, self.current);
                    if self.map.contains_key(&p) {
                        self.current.push('/');
                        self.current.push_str(c);
                        Ok(())
                    } else {
                        Err(Error::FileNotFound)
                    }
                }
            }
        }
    }

    pub(crate) fn directory_content(&self) -> Result<&PathContent> {
        if self.prefix.is_empty() {
            Err(Error::FilesNotSynced)
        } else {
            let p = format!("{}{}", self.prefix, self.current);
            let cnt = self.map.get(&p).unwrap();
            Ok(cnt)
        }
    }
}

impl From<sync::FileSyncEntry> for PathContent {
    fn from(value: sync::FileSyncEntry) -> Self {
        PathContent {
            dirs: value.dirs,
            files: value.files,
        }
    }
}

/*use super::DirEntry;
use crate::json::types::FileInfo;

#[derive(PartialEq)]
enum SyncState {
    Cached,
    Unsynced,
    Synced,
}

struct TrackInfo {
    genre: String,
    artist: String,
    album: String,
    title: String,
    track: u16,
    duration: u16,
}

struct Track {
    filename: Vec<String>,
    sync_state: SyncState,
    info: Option<TrackInfo>,
}

pub struct Data {
    current_dir: Vec<String>,
    tracks: Vec<Track>,
    num_to_sync: usize,
    num_synced: usize,
}

impl Data {
    pub fn new() -> Self {
        Self {
            current_dir: Vec::new(),
            tracks: Vec::new(),
            num_to_sync: 0,
            num_synced: 0,
        }
    }

    pub fn save(&self) {
        // TODO
    }

    pub fn dir_current(&self) -> String {
        match self.current_dir.last() {
            Some(d) => String::from(d),
            None => String::new(),
        }
    }

    pub fn dir_up(&mut self) {
        self.current_dir.pop();
    }

    pub fn dir_enter(&mut self, dir: &str) {
        self.current_dir.push(String::from(dir));
    }

    pub fn dir_content(&self) -> Vec<DirEntry> {
        let mut list: Vec<DirEntry> = self
            .tracks
            .iter()
            .filter_map(|t| {
                if t.filename.starts_with(&self.current_dir) {
                    if self.current_dir.len() + 1 == t.filename.len() {
                        Some(DirEntry::File(t.filename[self.current_dir.len()].clone()))
                    } else {
                        Some(DirEntry::Dir(t.filename[self.current_dir.len()].clone()))
                    }
                } else {
                    None
                }
            })
            .collect();

        list.sort();
        list.dedup();

        list
    }

    pub fn update_file_list(&mut self, lst: Vec<String>, last: bool) {
        for f in lst {
            let name = f.split("/").map(str::to_string).collect();
            if let Some(t) = self.tracks.iter_mut().find(|t| t.filename == name) {
                // TODO
                // if date ok, set to synced instead
                t.sync_state = SyncState::Unsynced;
            } else {
                self.tracks.push(Track {
                    filename: name,
                    sync_state: SyncState::Unsynced,
                    info: None,
                });
            }
        }
        if last {
            self.tracks.retain(|t| t.sync_state != SyncState::Cached);
            self.num_to_sync = self
                .tracks
                .iter()
                .filter(|t| t.sync_state == SyncState::Unsynced)
                .count();
            self.num_synced = 0;
        }
    }

    pub fn get_unsynced_file(&self) -> Option<String> {
        let t = self
            .tracks
            .iter()
            .filter(|t| {
                t.filename.last().unwrap().ends_with(".ogg") && t.sync_state == SyncState::Unsynced
            })
            .next();
        match t {
            Some(t) => Some(t.filename.join("/")),
            None => None,
        }
    }

    pub fn set_file_info(&mut self, info: FileInfo) {
        if let Some(t) = self
            .tracks
            .iter_mut()
            .find(|t| t.filename.join("/") == info.filename)
        {
            t.sync_state = SyncState::Synced;
            self.num_synced += 1;
        }
    }

    pub fn sync_stats(&self) -> (usize, usize) {
        (self.num_synced, self.num_to_sync)
    }
}
*/
