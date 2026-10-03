use std::cmp::Ordering;
use std::collections::HashMap;

use crate::backend::{ChangeDirection, TagViewContent};
use crate::sync::{SyncTags, TagSyncEntry};
use crate::{Element, Error, Result};

#[derive(Default)]
pub(crate) struct TagView {
    current: Vec<String>,
    map: HashMap<String, Tag>,
    genres: Genres,
}

pub(crate) struct Tag {
    pub genre: String,
    pub artist: String,
    pub album: String,
    pub title: String,
    pub date: Option<u16>,
    pub track: u16,
    pub _duration: u16,
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
    file: String,
    number: u16,
}

impl TagView {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn rebuild(&mut self, sync: SyncTags) {
        self.current.clear();
        self.map = sync.map.into_iter().map(|(k, v)| (k, v.into())).collect();
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
            if !album.tracks.contains_key(&v.title) {
                album.tracks.insert(
                    v.title.clone(),
                    Track {
                        file: k.clone(),
                        number: v.track,
                    },
                );
            }
        }
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
                                name: s.clone(),
                                file: t.file.clone(),
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
            _duration: value.duration,
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
