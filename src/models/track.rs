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
    pub fn new<T>(title: T) -> Track
    where
        T: Into<Text>,
    {
        Track {
            title: title.into(),
            artists: None,
            year: None,
            genre: None,
            comment: None,
            lyrics: None,
        }
    }

    pub fn title(&self) -> &Text {
        &self.title
    }

    pub fn artists(&self) -> Option<&[Text]> {
        self.artists.as_ref().map(Vec::as_slice)
    }

    pub fn push_artist<T>(&mut self, artist: T)
    where
        T: Into<Text>,
    {
        self.artists
            .get_or_insert_with(Vec::new)
            .push(artist.into())
    }

    pub fn with_artist<T>(mut self, artist: T) -> Self
    where
        T: Into<Text>,
    {
        self.push_artist(artist);
        self
    }

    pub fn set_year(&mut self, year: usize) {
        self.year = Some(year);
    }

    pub fn set_genre<T>(&mut self, genre: T)
    where
        T: Into<Text>,
    {
        self.genre = Some(genre.into())
    }

    pub fn set_comment<T>(&mut self, comment: T)
    where
        T: Into<Text>,
    {
        self.comment = Some(comment.into())
    }

    pub fn set_lyrics<T>(&mut self, lyrics: T)
    where
        T: Into<Text>,
    {
        self.lyrics = Some(lyrics.into())
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
            self.album().covers_path(),
            &self.track.title().file_safe(),
            transform_image,
        )
        .or_else(|_| self.album().cover())
    }

    pub fn cover_vw(&self) -> Result<Image, ImageError> {
        Image::load_with_cache(
            self.album().image_path(),
            self.album().covers_vw_path(),
            &self.track.title().file_safe(),
            transform_image_vw,
        )
        .or_else(|_| self.album().cover_vw())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::disc::Disc;

    #[test]
    fn artists_are_inherited_from_album() {
        let album = Album::new("title", PathBuf::from("."))
            .with_artist("a")
            .with_artist(Text::with_ascii("b", "c"))
            .with_disc(Disc::new().with_track(Track::new("song")));
        let disc = album.disc(1);
        let track = disc.track(1);
        assert_eq!(
            track.artists(),
            &[Text::new("a"), Text::with_ascii("b", "c")]
        );
    }

    #[test]
    fn artists_are_overridden_by_track() {
        let album = Album::new("title", PathBuf::from("."))
            .with_artist("a")
            .with_artist(Text::with_ascii("b", "c"))
            .with_disc(Disc::new().with_track(Track::new("song").with_artist("d")));
        let disc = album.disc(1);
        let track = disc.track(1);
        assert_eq!(track.artists(), &[Text::new("d")]);
    }

    #[test]
    fn no_album_artists_without_override() {
        let album = Album::new("title", PathBuf::from("."))
            .with_artist("a")
            .with_artist(Text::with_ascii("b", "c"))
            .with_disc(Disc::new().with_track(Track::new("song")));
        let disc = album.disc(1);
        let track = disc.track(1);
        assert_eq!(track.album_artists(), None);
    }

    #[test]
    fn album_artists_are_set_when_overridden() {
        let album = Album::new("title", PathBuf::from("."))
            .with_artist("a")
            .with_artist(Text::with_ascii("b", "c"))
            .with_disc(Disc::new().with_track(Track::new("song").with_artist("d")));
        let disc = album.disc(1);
        let track = disc.track(1);
        assert_eq!(
            track.album_artists(),
            Some(&[Text::new("a"), Text::with_ascii("b", "c")][..])
        );
    }
}
