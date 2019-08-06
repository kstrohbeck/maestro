use super::{album::Album, track::Track};
use crate::{
    image::{self as img, Image, LoadWithCacheError},
    raw,
    utils::num_digits,
};
use once_cell::sync::OnceCell;
use std::{borrow::Cow, path::Path};

#[derive(Clone)]
pub struct Disc<'a> {
    pub album: &'a Album,
    disc: &'a raw::Disc,
    pub disc_number: usize,
    cover: OnceCell<Option<Image>>,
    cover_vw: OnceCell<Option<Image>>,
}

impl<'a> Disc<'a> {
    pub fn new(album: &'a Album, disc: &'a raw::Disc, disc_number: usize) -> Self {
        Self {
            album,
            disc,
            disc_number,
            cover: OnceCell::new(),
            cover_vw: OnceCell::new(),
        }
    }

    pub fn num_tracks(&self) -> usize {
        self.disc.num_tracks()
    }

    pub fn is_only_disc(&self) -> bool {
        self.album.num_discs() == 1
    }

    pub fn track(&self, track_number: usize) -> Option<Track> {
        self.disc
            .tracks()
            .get(track_number - 1)
            .map(|t| Track::new(Cow::Borrowed(self), t, track_number))
    }

    pub fn into_track(self, track_number: usize) -> Option<Track<'a>> {
        self.disc
            .tracks()
            .get(track_number - 1)
            .map(|track| Track::new(Cow::Owned(self), track, track_number))
    }

    pub fn tracks(&self) -> impl Iterator<Item = Track> {
        self.disc
            .tracks()
            .iter()
            .zip(1..)
            .map(move |(t, i)| Track::new(Cow::Borrowed(self), t, i))
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

    fn get_cover<'b, P, F, G>(
        &'b self,
        cover: &'b OnceCell<Option<Image>>,
        covers_path: P,
        transform: F,
        fallback: G,
    ) -> Result<Option<&'b Image>, LoadWithCacheError>
    where
        P: AsRef<Path>,
        F: Fn(image::DynamicImage) -> Result<Image, image::ImageError>,
        G: Fn() -> Result<Option<&'b Image>, LoadWithCacheError>,
    {
        cover
            .get_or_try_init(|| {
                let name = match self.filename() {
                    Some(name) => name,
                    None => return Ok(None),
                };

                Image::try_load_with_cache(self.album.image_path(), covers_path, &name, transform)
            })
            .and_then(|o| match o {
                Some(x) => Ok(Some(x)),
                None => fallback(),
            })
    }

    pub fn cover(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.get_cover(
            &self.cover,
            self.album.covers_path(),
            img::transform_image,
            || self.album.cover(),
        )
    }

    pub fn cover_vw(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.get_cover(
            &self.cover_vw,
            self.album.covers_vw_path(),
            img::transform_image_vw,
            || self.album.cover_vw(),
        )
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
