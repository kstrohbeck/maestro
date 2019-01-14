use crate::{
    image::{transform_image, transform_image_vw, Image, ImageError},
    models::{album::Album, disc::DiscInContext},
    text::Text,
    utils::{comma_separated, num_digits},
};
use std::path::PathBuf;

pub struct Track {
    title: Text,
    artists: Option<Vec<Text>>,
    year: Option<usize>,
    genre: Option<Text>,
    pub comment: Option<Text>,
    pub lyrics: Option<Text>,
}

impl Track {
    pub fn title(&self) -> &Text {
        &self.title
    }

    pub fn artists(&self) -> Option<&[Text]> {
        self.artists.as_ref().map(Vec::as_slice)
    }
}

pub struct TrackInContext<'a> {
    pub disc: &'a DiscInContext<'a>,
    track: &'a Track,
    pub track_number: usize,
}

impl<'a> TrackInContext<'a> {
    pub fn new(
        disc: &'a DiscInContext<'a>,
        track: &'a Track,
        track_number: usize,
    ) -> TrackInContext<'a> {
        TrackInContext {
            disc,
            track,
            track_number,
        }
    }

    fn album(&self) -> &Album {
        self.disc.album
    }

    pub fn title(&self) -> &Text {
        self.track.title()
    }

    pub fn artists(&self) -> &[Text] {
        self.track
            .artists()
            .unwrap_or_else(|| self.disc.album.artists())
    }

    pub fn artist(&self) -> Text {
        comma_separated(self.artists())
    }

    pub fn album_artists(&self) -> Option<&[Text]> {
        if self.artists() != self.album().artists() {
            Some(self.album().artists())
        } else {
            None
        }
    }

    pub fn album_artist(&self) -> Option<Text> {
        self.album_artists().map(comma_separated)
    }

    pub fn year(&self) -> Option<usize> {
        self.track.year.or(self.album().year)
    }

    pub fn genre(&self) -> Option<&Text> {
        self.track.genre.as_ref().or_else(|| self.album().genre())
    }

    pub fn comment(&self) -> Option<&Text> {
        self.track.comment.as_ref()
    }

    pub fn lyrics(&self) -> Option<&Text> {
        self.track.lyrics.as_ref()
    }

    pub fn filename(&self) -> String {
        let digits = num_digits(self.disc.num_tracks());
        format!(
            "{:0width$} - {}.mp3",
            self.track_number,
            self.title().file_safe(),
            width = digits
        )
    }

    pub fn path(&self) -> PathBuf {
        self.disc.path().join(self.filename())
    }

    pub fn cover(&self) -> Result<Image, ImageError> {
        Image::load_with_cache(
            self.album().image_path(),
            self.album().cache_path().join("covers"),
            &self.track.title().file_safe(),
            transform_image,
        )
        .or_else(|_| self.album().cover())
    }

    pub fn cover_vw(&self) -> Result<Image, ImageError> {
        Image::load_with_cache(
            self.album().image_path(),
            self.album().cache_path().join("covers-vw"),
            &self.track.title().file_safe(),
            transform_image_vw,
        )
        .or_else(|_| self.album().cover_vw())
    }
}
