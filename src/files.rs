use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use log::{error, warn};
use serde::{Deserialize, Serialize};

use crate::backend::{ChangeDirection, FileViewContent};
use crate::sync::{FileSyncEntry, SyncFiles};
use crate::{Element, Error, Result};

#[derive(Default)]
pub(crate) struct FileView {
    cache_file: Option<PathBuf>,
    prefix: String,
    current: String,
    map: HashMap<String, FileViewContent>,
}

#[derive(Serialize, Deserialize)]
struct Cache {
    prefix: String,
    entries: Vec<CacheEntry>,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    folders: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tracks: Option<Vec<String>>,
}

impl FileView {
    pub(crate) fn new(cache_dir: Option<&Path>) -> Self {
        let mut fileview = Self {
            cache_file: cache_dir.map(|d| d.join("files.json")),
            ..Default::default()
        };
        fileview.read_cache();
        fileview
    }

    pub(crate) fn rebuild(&mut self, sync: SyncFiles) {
        self.prefix = sync.prefix.unwrap();
        self.current.clear();
        self.map = sync.map.into_iter().map(|(k, v)| (k, v.into())).collect();
        self.write_cache();
    }

    pub(crate) fn prefix(&self) -> &str {
        &self.prefix
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
                            vec.push(t.file.clone());
                        }
                    }
                }
            }
        }

        let mut vec = Vec::new();
        add_all_tracks_in_tree(dir, &mut vec, &self.map);
        vec
    }

    fn read_cache(&mut self) {
        self.prefix.clear();
        self.current.clear();
        self.map.clear();
        if let Some(filename) = &self.cache_file {
            match File::open(filename) {
                Ok(f) => {
                    let mut reader = BufReader::new(f);
                    let mut content = String::new();
                    match reader.read_to_string(&mut content) {
                        Ok(_) => {
                            if let Ok(cache) = serde_json::from_str::<Cache>(&content) {
                                self.prefix = cache.prefix;
                                for entry in cache.entries {
                                    if let Some(folders) = entry.folders {
                                        let cnt = FileViewContent::Folders(folders);
                                        self.map.insert(entry.path, cnt);
                                    } else if let Some(tracks) = entry.tracks {
                                        let tracks = tracks
                                            .into_iter()
                                            .map(|n| Element {
                                                file: format!("{}/{}", entry.path, n),
                                                name: n,
                                            })
                                            .collect();
                                        let cover = entry.cover.map(|n| Element {
                                            file: format!("{}/{}", entry.path, n),
                                            name: n,
                                        });
                                        let cnt = FileViewContent::Files { cover, tracks };
                                        self.map.insert(entry.path, cnt);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("could not read from {}: {e}", filename.display());
                        }
                    }
                }
                Err(e) => warn!("could not open {}: {e}", filename.display()),
            }
        }
    }

    fn write_cache(&self) {
        if let Some(filename) = &self.cache_file
            && let Some(dir) = filename.parent()
        {
            if !dir.exists()
                && let Err(e) = fs::create_dir_all(dir)
            {
                error!("could not create {}: {e}", dir.display());
            }
            if dir.is_dir() {
                match File::create(filename) {
                    Ok(f) => {
                        let mut writer = BufWriter::new(f);
                        let mut entries = Vec::new();
                        for (k, v) in &self.map {
                            let entry = match v {
                                FileViewContent::Folders(folders) => CacheEntry {
                                    path: k.clone(),
                                    folders: Some(folders.clone()),
                                    cover: None,
                                    tracks: None,
                                },
                                FileViewContent::Files { cover, tracks } => CacheEntry {
                                    path: k.clone(),
                                    folders: None,
                                    cover: cover.as_ref().map(Into::into),
                                    tracks: Some(tracks.iter().map(Into::into).collect()),
                                },
                            };
                            entries.push(entry);
                        }
                        entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));
                        let cache = Cache {
                            prefix: self.prefix.clone(),
                            entries,
                        };
                        if let Ok(content) = serde_json::to_string_pretty(&cache)
                            && let Err(e) = writer.write(&content.into_bytes())
                        {
                            error!("could not write to {}: {e}", filename.display());
                        }
                    }
                    Err(e) => error!("could not create {}: {e}", filename.display()),
                }
            }
        }
    }
}

impl From<FileSyncEntry> for FileViewContent {
    fn from(value: FileSyncEntry) -> Self {
        if value.dirs.is_empty() {
            let cover = value.cover;
            let mut tracks = value.tracks;
            tracks.sort_unstable_by(|a, b| a.name.cmp(&b.name));
            FileViewContent::Files { cover, tracks }
        } else {
            let mut folders = value.dirs;
            folders.sort_unstable();
            FileViewContent::Folders(folders)
        }
    }
}

impl From<&Element> for String {
    fn from(value: &Element) -> Self {
        value.name.clone()
    }
}
