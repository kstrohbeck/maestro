use crate::{
    models::{
        album::Album,
        track::{Track, TrackInContext},
    },
    utils::num_digits,
};
use std::{borrow::Cow, path::Path};

pub struct Disc {
    tracks: Vec<Track>,
}

impl Disc {
    pub fn num_tracks(&self) -> usize {
        self.tracks.len()
    }

    pub fn push_track(&mut self, track: Track) {
        self.tracks.push(track)
    }
}

pub struct DiscInContext<'a> {
    pub album: &'a Album,
    disc: &'a Disc,
    pub disc_number: usize,
}

impl<'a> DiscInContext<'a> {
    pub fn new(album: &'a Album, disc: &'a Disc, disc_number: usize) -> DiscInContext<'a> {
        DiscInContext {
            album,
            disc,
            disc_number,
        }
    }

    pub fn num_tracks(&self) -> usize {
        self.disc.num_tracks()
    }

    pub fn track(&self, track_number: usize) -> TrackInContext {
        TrackInContext::new(&self, &self.disc.tracks[track_number - 1], track_number)
    }

    pub fn tracks(&self) -> impl Iterator<Item = TrackInContext> {
        self.disc
            .tracks
            .iter()
            .zip(1..)
            .map(move |(t, i)| TrackInContext::new(&self, t, i))
    }

    fn filename(&self) -> Option<String> {
        if self.album.num_discs() == 1 {
            None
        } else {
            let digits = num_digits(self.album.num_discs());
            Some(format!("Disc {:0width$}", self.disc_number, width = digits))
        }
    }

    pub fn path(&self) -> Cow<Path> {
        match self.filename() {
            None => self.album.path().into(),
            Some(name) => self.album.path().join(name).into(),
        }
    }
}
