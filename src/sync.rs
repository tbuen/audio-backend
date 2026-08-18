use std::collections::HashMap;
use std::sync::MutexGuard;
use std::time::{Duration, Instant};

use log::{error, info};

use crate::common::jsonrpc;
use crate::filesystem::FileSystem;
use crate::json::{FileList, TrackInfo};

const PARALLEL_REQUESTS: usize = 5;
const SYNC_TIMEOUT_S: u64 = 2;

pub(crate) struct SyncFiles {
    started: bool,
    starttime: Instant,
    timestamp: Instant,
    error: Option<jsonrpc::ExecError>,
    pub prefix: Option<String>,
    pub map: HashMap<String, FileSyncEntry>,
}

#[derive(Debug, Default)]
pub(crate) struct FileSyncEntry {
    step: SyncStep,
    pub cover: Option<String>,
    pub dirs: Vec<String>,
    pub tracks: Vec<String>,
}

pub(crate) struct SyncTags {
    starttime: Instant,
    timestamp: Instant,
    error: Option<jsonrpc::ExecError>,
    map: HashMap<String, TagSyncEntry>,
}

#[derive(Debug, Default)]
pub(crate) struct TagSyncEntry {
    step: SyncStep,
    pub genre: String,
    pub artist: String,
    pub album: String,
    pub title: String,
    pub date: Option<u16>,
    pub track: u16,
    pub duration: u16,
}

#[derive(Debug)]
pub(crate) enum FileSyncState<'a> {
    NextToSync(Vec<&'a str>),
    Waiting,
    Timeout,
    Finished,
    Error(jsonrpc::ExecError),
}

#[derive(Debug)]
pub(crate) enum TagSyncState<'a> {
    NextToSync(Vec<&'a str>, usize, usize),
    Waiting,
    Timeout,
    Finished,
    Error(jsonrpc::ExecError),
}

#[derive(Debug, Default)]
enum SyncStep {
    #[default]
    New,
    Requested,
    Received,
}

impl<'a> SyncFiles {
    pub(crate) fn start() -> Self {
        Self {
            started: false,
            starttime: Instant::now(),
            timestamp: Instant::now(),
            error: None,
            prefix: None,
            map: HashMap::new(),
        }
    }

    pub(crate) fn state(&'a mut self) -> FileSyncState<'a> {
        let n_total = self.map.len();
        let mut n_new = self
            .map
            .values()
            .filter(|e| matches!(e.step, SyncStep::New))
            .count();
        let mut n_req = self
            .map
            .values()
            .filter(|e| matches!(e.step, SyncStep::Requested))
            .count();

        if !self.started {
            self.started = true;
            FileSyncState::NextToSync(Vec::new())
        } else if let Some(e) = self.error.take() {
            error!("file sync error");
            FileSyncState::Error(e)
        } else if n_total > 0 && n_new == 0 && n_req == 0 {
            info!(
                "file sync finished in {}ms",
                self.starttime.elapsed().as_millis()
            );
            FileSyncState::Finished
        } else if Instant::now() > self.timestamp + Duration::from_secs(SYNC_TIMEOUT_S) {
            error!("file sync timeout");
            FileSyncState::Timeout
        } else if n_new > 0 && n_req < PARALLEL_REQUESTS {
            let mut vec = Vec::new();
            for (k, v) in &mut self.map {
                if matches!(v.step, SyncStep::New) {
                    vec.push(k.as_str());
                    v.step = SyncStep::Requested;
                    n_new -= 1;
                    n_req += 1;
                    if n_new == 0 || n_req == PARALLEL_REQUESTS {
                        break;
                    }
                }
            }
            FileSyncState::NextToSync(vec)
        } else {
            FileSyncState::Waiting
        }
    }

    pub(crate) fn insert_response(&mut self, resp: Result<FileList, jsonrpc::ExecError>) {
        match resp {
            Ok(list) => {
                if self.prefix.is_none() {
                    self.prefix = Some(list.path.clone());
                }
                if let Some(dirs) = &list.dirs {
                    for d in dirs {
                        let p = format!("{}/{}", list.path, d);
                        self.map.insert(p, FileSyncEntry::default());
                    }
                }
                let entry = self.map.entry(list.path).or_default();
                entry.step = SyncStep::Received;
                if let Some(dirs) = list.dirs {
                    for d in dirs {
                        entry.dirs.push(d);
                    }
                }
                if let Some(tracks) = list.tracks {
                    for t in tracks {
                        entry.tracks.push(t);
                    }
                }
                entry.cover = list.cover;
                self.timestamp = Instant::now();
            }
            Err(e) => {
                self.error = Some(e);
                self.timestamp = Instant::now();
            }
        }
    }
}

impl<'a> SyncTags {
    pub(crate) fn start(dir: &str, fs: MutexGuard<FileSystem>) -> Self {
        let mut map = HashMap::new();
        for t in fs.get_all_tracks_in_tree(dir) {
            map.insert(t, TagSyncEntry::default());
        }
        info!("try to sync {} tags", map.len());
        Self {
            starttime: Instant::now(),
            timestamp: Instant::now(),
            error: None,
            map,
        }
    }

    pub(crate) fn state(&'a mut self) -> TagSyncState<'a> {
        let n_total = self.map.len();
        let mut n_new = self
            .map
            .values()
            .filter(|e| matches!(e.step, SyncStep::New))
            .count();
        let mut n_req = self
            .map
            .values()
            .filter(|e| matches!(e.step, SyncStep::Requested))
            .count();

        if let Some(e) = self.error.take() {
            error!("tag sync error");
            TagSyncState::Error(e)
        } else if n_new == 0 && n_req == 0 {
            info!(
                "tag sync finished in {}ms",
                self.starttime.elapsed().as_millis()
            );
            TagSyncState::Finished
        } else if Instant::now() > self.timestamp + Duration::from_secs(SYNC_TIMEOUT_S) {
            error!("tag sync timeout");
            TagSyncState::Timeout
        } else if n_new > 0 && n_req < PARALLEL_REQUESTS {
            let mut vec = Vec::new();
            for (k, v) in &mut self.map {
                if matches!(v.step, SyncStep::New) {
                    vec.push(k.as_str());
                    v.step = SyncStep::Requested;
                    n_new -= 1;
                    n_req += 1;
                    if n_new == 0 || n_req == PARALLEL_REQUESTS {
                        break;
                    }
                }
            }
            TagSyncState::NextToSync(vec, n_total - n_new - n_req, n_total)
        } else {
            TagSyncState::Waiting
        }
    }

    pub(crate) fn insert_response(&mut self, resp: Result<TrackInfo, jsonrpc::ExecError>) {
        match resp {
            Ok(tag) => {
                if let Some(entry) = self.map.get_mut(&tag.file) {
                    entry.genre = tag.genre;
                    entry.artist = tag.artist;
                    entry.album = tag.album;
                    entry.title = tag.title;
                    entry.date = tag.date;
                    entry.track = tag.track;
                    entry.duration = tag.duration;
                    entry.step = SyncStep::Received;
                } else {
                    error!("received unexpected tag");
                }
                self.timestamp = Instant::now();
            }
            Err(e) => {
                self.error = Some(e);
                self.timestamp = Instant::now();
            }
        }
    }
}
