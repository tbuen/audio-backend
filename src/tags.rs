use std::collections::HashMap;

use crate::backend::{ChangeDirection, TagViewContent};
use crate::sync::{SyncTags, TagSyncEntry};
use crate::{Error, Result};

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
type Titles = HashMap<String, Title>;

struct Album {
    date: Option<u16>,
    titles: Titles,
}

struct Title {
    _file: String,
    track: u16,
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
                        titles: HashMap::new(),
                    },
                );
            }
            let album = albums.get_mut(&v.album).unwrap();
            if !album.titles.contains_key(&v.title) {
                album.titles.insert(
                    v.title.clone(),
                    Title {
                        _file: k.clone(),
                        track: v.track,
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
                    for (k, v) in &album.titles {
                        vec.push((k, v.track));
                    }
                    vec.sort_unstable_by_key(|t| t.1);
                    TagViewContent::Titles(vec.into_iter().map(|(t, _)| t.clone()).collect())
                } else {
                    let mut vec = Vec::new();
                    for (k, v) in albums {
                        vec.push((k, v.date));
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
