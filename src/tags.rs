use std::collections::HashMap;

use crate::backend::{ChangeDirection, TagViewContent};
use crate::sync::{SyncTags, TagSyncEntry};
use crate::{Error, Result};

type Genres = HashMap<String, Artists>;
type Artists = HashMap<String, Albums>;
type Albums = HashMap<String, Titles>;
type Titles = HashMap<String, String>;

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
    pub _date: Option<u16>,
    pub _track: u16,
    pub _duration: u16,
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
                albums.insert(v.album.clone(), HashMap::new());
            }
            let titles = albums.get_mut(&v.album).unwrap();
            if !titles.contains_key(&v.title) {
                titles.insert(v.title.clone(), k.clone());
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
                    let titles = albums.get(album).unwrap();
                    TagViewContent::Titles(titles.keys().cloned().collect())
                } else {
                    TagViewContent::Albums(albums.keys().cloned().collect())
                }
            } else {
                TagViewContent::Artists(artists.keys().cloned().collect())
            }
        } else {
            TagViewContent::Genres(self.genres.keys().cloned().collect())
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
            _date: value.date,
            _track: value.track,
            _duration: value.duration,
        }
    }
}
