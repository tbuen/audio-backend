use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use log::{error, warn};
use serde::{Deserialize, Serialize};

use crate::sync::{SyncTags, TagSyncEntry};
use crate::{ChangeDirection, Element, Tag, TagViewContent};
use crate::{Error, Result};

#[derive(Default)]
pub(crate) struct TagView {
    cache_file: Option<PathBuf>,
    current: Vec<String>,
    map: HashMap<String, Tag>,
    genres: Genres,
}

type Genres = HashMap<String, Artists>;
type Artists = HashMap<String, Albums>;
type Albums = HashMap<String, Album>;
type Tracks = HashMap<String, Track>;

struct Album {
    date: Option<u16>,
    tracks: Tracks,
}

struct Track {
    title: String,
    number: u16,
}

#[derive(Serialize, Deserialize)]
struct Cache {
    entries: Vec<CacheEntry>,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    file: String,
    genre: String,
    artist: String,
    album: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<u16>,
    track: u16,
    duration: u16,
}

impl TagView {
    pub(crate) fn new(cache_dir: Option<&Path>) -> Self {
        let mut tagview = Self {
            cache_file: cache_dir.map(|d| d.join("tags.json")),
            ..Default::default()
        };
        tagview.read_cache();
        tagview.rebuild_database();
        tagview
    }

    pub(crate) fn rebuild(&mut self, sync: SyncTags) {
        self.current.clear();
        self.map
            .extend(sync.map.into_iter().map(|(k, v)| (k, v.into())));
        self.write_cache();
        self.rebuild_database();
    }

    pub(crate) fn remove_tracks_except(&mut self, filelist: Vec<String>) {
        let mut new_map = HashMap::new();
        for file in filelist {
            if let Some(v) = self.map.remove(&file) {
                new_map.insert(file, v);
            }
        }
        self.map = new_map;
        self.rebuild_database();
    }

    pub(crate) fn current(&self) -> &Vec<String> {
        &self.current
    }

    pub(crate) fn change(&mut self, to: ChangeDirection) -> Result<()> {
        match to {
            ChangeDirection::ToRoot => {
                self.current.clear();
                Ok(())
            }
            ChangeDirection::ToParent => {
                if self.current.is_empty() {
                    Err(Error::DirectoryNotFound)
                } else {
                    self.current.pop();
                    Ok(())
                }
            }
            ChangeDirection::ToChild(c) => {
                if let Some(genre) = self.current.first() {
                    let artists = self.genres.get(genre).unwrap();
                    if let Some(artist) = self.current.get(1) {
                        let albums = artists.get(artist).unwrap();
                        if let Some(_album) = self.current.get(2) {
                            Err(Error::DirectoryNotFound)
                        } else if albums.contains_key(c) {
                            self.current.push(c.to_owned());
                            Ok(())
                        } else {
                            Err(Error::DirectoryNotFound)
                        }
                    } else if artists.contains_key(c) {
                        self.current.push(c.to_owned());
                        Ok(())
                    } else {
                        Err(Error::DirectoryNotFound)
                    }
                } else if self.genres.contains_key(c) {
                    self.current.push(c.to_owned());
                    Ok(())
                } else {
                    Err(Error::DirectoryNotFound)
                }
            }
        }
    }

    pub(crate) fn content(&self) -> TagViewContent {
        if let Some(genre) = self.current.first() {
            let artists = self.genres.get(genre).unwrap();
            if let Some(artist) = self.current.get(1) {
                let albums = artists.get(artist).unwrap();
                if let Some(album) = self.current.get(2) {
                    let album = albums.get(album).unwrap();
                    let mut vec = Vec::new();
                    for (k, v) in &album.tracks {
                        vec.push((k, v));
                    }
                    vec.sort_unstable_by_key(|t| t.1);
                    TagViewContent::Tracks(
                        vec.into_iter()
                            .map(|(s, t)| Element {
                                name: t.title.clone(),
                                file: s.clone(),
                            })
                            .collect(),
                    )
                } else {
                    let mut vec = Vec::new();
                    for (k, v) in albums {
                        vec.push((k, v));
                    }
                    vec.sort_unstable_by_key(|a| a.0);
                    vec.sort_by_key(|a| a.1);
                    TagViewContent::Albums(vec.into_iter().map(|(a, _)| a.clone()).collect())
                }
            } else {
                let mut artists: Vec<String> = artists.keys().cloned().collect();
                artists.sort_unstable();
                TagViewContent::Artists(artists)
            }
        } else {
            let mut genres: Vec<String> = self.genres.keys().cloned().collect();
            genres.sort_unstable();
            TagViewContent::Genres(genres)
        }
    }

    pub(crate) fn tag(&self, file: &str) -> Option<&Tag> {
        self.map.get(file)
    }

    fn rebuild_database(&mut self) {
        self.genres.clear();
        for (k, v) in &self.map {
            if !self.genres.contains_key(&v.genre) {
                self.genres.insert(v.genre.clone(), HashMap::new());
            }
            let artists = self.genres.get_mut(&v.genre).unwrap();
            if !artists.contains_key(&v.artist) {
                artists.insert(v.artist.clone(), HashMap::new());
            }
            let albums = artists.get_mut(&v.artist).unwrap();
            if !albums.contains_key(&v.album) {
                albums.insert(
                    v.album.clone(),
                    Album {
                        date: v.date,
                        tracks: HashMap::new(),
                    },
                );
            }
            let album = albums.get_mut(&v.album).unwrap();
            if !album.tracks.contains_key(k) {
                album.tracks.insert(
                    k.clone(),
                    Track {
                        title: v.title.clone(),
                        number: v.track,
                    },
                );
            }
        }
    }

    fn read_cache(&mut self) {
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
                                for entry in cache.entries {
                                    let tag = Tag {
                                        genre: entry.genre,
                                        artist: entry.artist,
                                        album: entry.album,
                                        title: entry.title,
                                        date: entry.date,
                                        track: entry.track,
                                        duration: entry.duration,
                                    };
                                    self.map.insert(entry.file, tag);
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
                            let entry = CacheEntry {
                                file: k.clone(),
                                genre: v.genre.clone(),
                                artist: v.artist.clone(),
                                album: v.album.clone(),
                                title: v.title.clone(),
                                date: v.date,
                                track: v.track,
                                duration: v.duration,
                            };
                            entries.push(entry);
                        }
                        entries.sort_unstable_by(|a, b| a.file.cmp(&b.file));
                        let cache = Cache { entries };
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

impl From<TagSyncEntry> for Tag {
    fn from(value: TagSyncEntry) -> Self {
        Tag {
            genre: value.genre,
            artist: value.artist,
            album: value.album,
            title: value.title,
            date: value.date,
            track: value.track,
            duration: value.duration,
        }
    }
}

impl Ord for Album {
    fn cmp(&self, other: &Self) -> Ordering {
        self.date.cmp(&other.date)
    }
}

impl PartialOrd for Album {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Album {
    fn eq(&self, other: &Self) -> bool {
        self.date == other.date
    }
}

impl Eq for Album {}

impl Ord for Track {
    fn cmp(&self, other: &Self) -> Ordering {
        self.number.cmp(&other.number)
    }
}

impl PartialOrd for Track {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Track {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
}

impl Eq for Track {}
