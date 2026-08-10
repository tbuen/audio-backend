use std::collections::HashMap;

use crate::FileSync;

#[derive(Default)]
pub struct FileSystem {
    prefix: String,
    pwd: String,
    map: HashMap<String, PathContent>,
}

pub enum FsError {
    NotSynced,
    NotFound,
}

pub enum Dir<'a> {
    Root,
    Up,
    Down(&'a str),
}

struct PathContent {
    dirs: Vec<String>,
    files: Vec<String>,
}

impl FileSystem {
    pub(crate) fn new() -> Self {
        FileSystem::default()
    }

    pub(crate) fn rebuild(&mut self, sync: &mut FileSync) {
        self.prefix.clone_from(&sync.prefix);
        self.pwd.clear();
        self.map.clear();
        for (k, v) in sync.map.drain() {
            self.map.insert(
                k,
                PathContent {
                    dirs: v.dirs,
                    files: v.files,
                },
            );
        }
    }

    pub fn pwd(&self) -> Result<String, FsError> {
        if self.prefix.is_empty() {
            Err(FsError::NotSynced)
        } else if self.pwd.is_empty() {
            Ok(String::new())
        } else {
            Ok(self.pwd.clone())
        }
    }

    pub fn cd(&mut self, dir: Dir) -> Result<(), FsError> {
        if self.prefix.is_empty() {
            Err(FsError::NotSynced)
        } else {
            match dir {
                Dir::Root => {
                    self.pwd.clear();
                    Ok(())
                }
                Dir::Up => {
                    if let Some((s, _)) = self.pwd.rsplit_once('/') {
                        self.pwd = s.to_owned();
                    }
                    Ok(())
                }
                Dir::Down(d) => {
                    let p = format!("{}{}/{d}", self.prefix, self.pwd);
                    if self.map.contains_key(&p) {
                        self.pwd.push('/');
                        self.pwd.push_str(d);
                        Ok(())
                    } else {
                        Err(FsError::NotFound)
                    }
                }
            }
        }
    }

    pub fn ls(&self) -> Result<(Vec<String>, Vec<String>), FsError> {
        if self.prefix.is_empty() {
            Err(FsError::NotSynced)
        } else {
            let p = format!("{}{}", self.prefix, self.pwd);
            let cnt = self.map.get(&p).unwrap();
            Ok((cnt.dirs.clone(), cnt.files.clone()))
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
