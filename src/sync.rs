use std::collections::HashMap;
use std::time::{Duration, Instant};

use log::{error, info};

use crate::json::FileList;
// TODO use general Error instead of jsronrpc::ExecError
use crate::common::jsonrpc;

const PARALLEL_REQUESTS: usize = 5;
const SYNC_TIMEOUT_S: u64 = 2;

pub(crate) struct Files {
    started: bool,
    starttime: Instant,
    timestamp: Instant,
    error: Option<jsonrpc::ExecError>,
    pub prefix: Option<String>,
    pub map: HashMap<String, FileSyncEntry>,
}

#[derive(Debug)]
pub(crate) enum State<'a> {
    NextToSync(Vec<&'a str>),
    Waiting,
    Timeout,
    Finished,
    Error(jsonrpc::ExecError),
}

#[derive(Debug, Default)]
pub(crate) struct FileSyncEntry {
    step: FileSyncStep,
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Default)]
enum FileSyncStep {
    #[default]
    New,
    Requested,
    Received,
}

impl<'a> Files {
    pub(crate) fn start() -> Self {
        Files {
            started: false,
            starttime: Instant::now(),
            timestamp: Instant::now(),
            error: None,
            prefix: None,
            map: HashMap::new(),
        }
    }

    pub(crate) fn state(&'a mut self) -> State<'a> {
        let n_total = self.map.len();
        let mut n_new = self
            .map
            .values()
            .filter(|e| matches!(e.step, FileSyncStep::New))
            .count();
        let mut n_req = self
            .map
            .values()
            .filter(|e| matches!(e.step, FileSyncStep::Requested))
            .count();

        if !self.started {
            self.started = true;
            State::NextToSync(Vec::new())
        } else if let Some(e) = self.error.take() {
            error!("file sync error");
            State::Error(e)
        } else if n_total > 0 && n_new == 0 && n_req == 0 {
            info!(
                "file sync finished in {}ms",
                self.starttime.elapsed().as_millis()
            );
            State::Finished
        } else if Instant::now() > self.timestamp + Duration::from_secs(SYNC_TIMEOUT_S) {
            error!("file sync timeout");
            State::Timeout
        } else if n_new > 0 && n_req < PARALLEL_REQUESTS {
            let mut vec = Vec::new();
            for (k, v) in &mut self.map {
                if matches!(v.step, FileSyncStep::New) {
                    vec.push(k.as_str());
                    v.step = FileSyncStep::Requested;
                    n_new -= 1;
                    n_req += 1;
                    if n_new == 0 || n_req == PARALLEL_REQUESTS {
                        break;
                    }
                }
            }
            State::NextToSync(vec)
        } else {
            State::Waiting
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
                entry.step = FileSyncStep::Received;
                if let Some(dirs) = list.dirs {
                    for d in dirs {
                        entry.dirs.push(d);
                    }
                }
                if let Some(files) = list.files {
                    for f in files {
                        entry.files.push(f);
                    }
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
