use crate::{
    models::{
        album::Album,
        track::{Track, TrackInContext},
    },
    utils::num_digits,
};
use std::{borrow::Cow, path::Path};
use yaml_rust::Yaml;

#[derive(Default)]
pub struct Disc {
    tracks: Vec<Track>,
}

impl Disc {
    pub fn new() -> Disc {
        Default::default()
    }

    pub fn from_tracks(tracks: Vec<Track>) -> Disc {
        Disc { tracks }
    }

    pub fn from_yaml(yaml: Yaml) -> Option<Disc> {
        let tracks = yaml
            .into_vec()?
            .into_iter()
            .map(Track::from_yaml)
            .collect::<Option<Vec<_>>>()?;
        Some(Disc::from_tracks(tracks))
    }

    pub fn num_tracks(&self) -> usize {
        self.tracks.len()
    }

    pub fn push_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn with_track(mut self, track: Track) -> Self {
        self.tracks.push(track);
        self
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
        if self.is_only_disc() {
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

    pub fn is_only_disc(&self) -> bool {
        self.album.num_discs() == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn only_disc_has_empty_filename() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_disc(Disc::new());
        let disc = album.disc(1);
        assert_eq!(disc.filename(), None);
    }

    #[test]
    fn first_disc_is_named_correctly() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_disc(Disc::new());
        album.push_disc(Disc::new());
        let disc = album.disc(1);
        assert_eq!(disc.filename().unwrap(), "Disc 1");
    }

    #[test]
    fn second_disc_is_named_correctly() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_disc(Disc::new());
        album.push_disc(Disc::new());
        let disc = album.disc(2);
        assert_eq!(disc.filename().unwrap(), "Disc 2");
    }

    #[test]
    fn only_disc_has_same_path_as_album() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_disc(Disc::new());
        let disc = album.disc(1);
        assert_eq!(disc.path(), album.path());
    }
}
