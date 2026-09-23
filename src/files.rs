use std::collections::HashMap;

use crate::backend::{ChangeDirection, FileViewContent};
use crate::sync::{FileSyncEntry, SyncFiles};
use crate::{Error, Result};

#[derive(Default)]
pub(crate) struct FileView {
    prefix: String,
    current: String,
    map: HashMap<String, FileViewContent>,
}

impl FileView {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn rebuild(&mut self, sync: SyncFiles) {
        self.prefix = sync.prefix.unwrap();
        self.current.clear();
        self.map = sync.map.into_iter().map(|(k, v)| (k, v.into())).collect();
    }

    pub(crate) fn current(&self) -> &str {
        &self.current
    }

    pub(crate) fn current_with_prefix(&self) -> Result<String> {
        if self.prefix.is_empty() {
            Err(Error::FilesNotSynced)
        } else {
            Ok(format!("{}{}", self.prefix, self.current))
        }
    }

    pub(crate) fn change(&mut self, to: ChangeDirection) -> Result<()> {
        match to {
            ChangeDirection::ToRoot => {
                self.current.clear();
                Ok(())
            }
            ChangeDirection::ToParent => {
                if let Some((s, _)) = self.current.rsplit_once('/') {
                    self.current = s.to_owned();
                    Ok(())
                } else {
                    Err(Error::DirectoryNotFound)
                }
            }
            ChangeDirection::ToChild(c) => {
                let p = format!("{}{}/{c}", self.prefix, self.current);
                if self.map.contains_key(&p) {
                    self.current.push('/');
                    self.current.push_str(c);
                    Ok(())
                } else {
                    Err(Error::DirectoryNotFound)
                }
            }
        }
    }

    pub(crate) fn content(&self) -> FileViewContent {
        if self.prefix.is_empty() {
            FileViewContent::Folders(Vec::new())
        } else {
            let p = format!("{}{}", self.prefix, self.current);
            let cnt = self.map.get(&p).unwrap();
            cnt.clone()
        }
    }

    pub(crate) fn get_all_tracks_in_tree(&self, dir: &str) -> Vec<String> {
        fn add_all_tracks_in_tree(
            dir: &str,
            vec: &mut Vec<String>,
            map: &HashMap<String, FileViewContent>,
        ) {
            if let Some(v) = map.get(dir) {
                match v {
                    FileViewContent::Folders(folders) => {
                        for f in folders {
                            add_all_tracks_in_tree(&format!("{dir}/{f}"), vec, map);
                        }
                    }
                    FileViewContent::Files { tracks, .. } => {
                        for t in tracks {
                            vec.push(format!("{dir}/{t}"));
                        }
                    }
                }
            }
        }

        let mut vec = Vec::new();
        add_all_tracks_in_tree(dir, &mut vec, &self.map);
        vec
    }
}

impl From<FileSyncEntry> for FileViewContent {
    fn from(value: FileSyncEntry) -> Self {
        if value.dirs.is_empty() {
            let cover = value.cover;
            let mut tracks = value.tracks;
            tracks.sort_unstable();
            FileViewContent::Files { cover, tracks }
        } else {
            let mut folders = value.dirs;
            folders.sort_unstable();
            FileViewContent::Folders(folders)
        }
    }
}
