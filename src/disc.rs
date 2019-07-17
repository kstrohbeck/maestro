use super::{album::Album, track::Track};
use crate::{raw, utils::num_digits};
use std::{borrow::Cow, path::Path};

pub struct Disc<'a> {
    pub album: &'a Album,
    disc: &'a raw::Disc,
    pub disc_number: usize,
}

impl<'a> Disc<'a> {
    pub fn new(album: &'a Album, disc: &'a raw::Disc, disc_number: usize) -> Self {
        Self {
            album,
            disc,
            disc_number,
        }
    }

    pub fn num_tracks(&self) -> usize {
        self.disc.num_tracks()
    }

    pub fn is_only_disc(&self) -> bool {
        self.album.num_discs() == 1
    }

    pub fn track(&self, track_number: usize) -> Option<Track<&Disc>> {
        self.disc
            .tracks()
            .get(track_number - 1)
            .map(|t| Track::new(self, t, track_number))
    }

    pub fn into_track(self, track_number: usize) -> Track<'a, Disc<'a>> {
        let track = &self.disc.tracks()[track_number - 1];
        Track::new(self, track, track_number)
    }

    pub fn tracks(&self) -> impl Iterator<Item = Track<&Disc>> {
        self.disc
            .tracks()
            .iter()
            .zip(1..)
            .map(move |(t, i)| Track::new(self, t, i))
    }

    pub fn filename(&self) -> Option<String> {
        if self.is_only_disc() {
            None
        } else {
            let digits = num_digits(self.album.num_discs());
            Some(format!("Disc {:0width$}", self.disc_number, width = digits))
        }
    }

    pub fn path(&self) -> Cow<Path> {
        let album_path = self.album.path();
        match self.filename() {
            None => album_path.into(),
            Some(name) => album_path.join(name).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn only_disc_has_no_filename() {
        let album = raw::Album::new("foo").with_discs(vec![raw::Disc::new()]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        assert!(disc.filename().is_none());
    }

    #[test]
    fn discs_are_named_correctly() {
        let album = raw::Album::new("foo").with_discs(vec![raw::Disc::new(), raw::Disc::new()]);
        let album = Album::new(album, PathBuf::from("."));
        assert_eq!(
            Some(String::from("Disc 1")),
            album.disc(1).unwrap().filename()
        );
        assert_eq!(
            Some(String::from("Disc 2")),
            album.disc(2).unwrap().filename()
        );
    }

    #[test]
    fn only_disc_has_same_path_as_album() {
        let album = raw::Album::new("foo").with_discs(vec![raw::Disc::new()]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        assert_eq!(album.path(), disc.path());
    }
}
