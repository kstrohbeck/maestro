use crate::{
    image::{transform_image, transform_image_vw, Image, ImageError},
    models::{album::Album, disc::DiscInContext},
    text::Text,
    utils::{comma_separated, num_digits},
};
use id3::{frame::Content, Frame, Tag, Version};
use std::path::PathBuf;
use yaml_rust::Yaml;

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

    pub fn from_yaml(yaml: Yaml) -> Option<Self> {
        macro_rules! pop {
            ($hash:ident[$key:expr]) => {
                $hash.remove(&Yaml::from_str($key))
            };
            ($hash:ident[$key:expr] as Text) => {
                $hash
                    .remove(&Yaml::from_str($key))
                    .and_then(Text::from_yaml)
            };
        }

        // TODO: Return Result.
        match yaml {
            Yaml::String(title) => Some(Track::new(title)),
            Yaml::Hash(mut hash) => {
                let title = pop!(hash["title"] as Text)?;

                let artists = pop!(hash["artists"])
                    .and_then(Yaml::into_vec)
                    .and_then(|a| {
                        a.into_iter()
                            .map(Text::from_yaml)
                            .collect::<Option<Vec<_>>>()
                    })
                    .or_else(|| pop!(hash["artist"] as Text).map(|t| vec![t]));

                let year = pop!(hash["year"])
                    .and_then(Yaml::into_i64)
                    .map(|y| y as usize);
                let genre = pop!(hash["genre"] as Text);
                let comment = pop!(hash["comment"] as Text);
                let lyrics = pop!(hash["lyrics"] as Text);

                Some(Track {
                    title,
                    artists,
                    year,
                    genre,
                    comment,
                    lyrics,
                })
            }
            _ => None,
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

    pub fn update_id3(&self) {
        let mut tag = Tag::new();
        tag.set_title(self.title().text());
        if !self.artists().is_empty() {
            tag.set_artist(self.artist().text());
        }
        tag.set_track(self.track_number as u32);
        if let Some(album_artist) = self.album_artist() {
            tag.set_album_artist(album_artist.text());
        }
        if !self.disc.is_only_disc() {
            tag.set_disc(self.disc.disc_number as u32);
        }
        tag.set_album(self.album().title().text());
        if let Some(year) = self.year() {
            let timestamp = id3::Timestamp {
                year: year as i32,
                month: None,
                day: None,
                hour: None,
                minute: None,
                second: None,
            };
            tag.set_date_recorded(timestamp);
        }
        if let Some(genre) = self.genre() {
            tag.set_genre(genre.text());
        }
        if let Some(comment) = self.comment() {
            // TODO: Maybe make comments a dictionary from description to text?
            let comment = id3::frame::Comment {
                lang: "eng".to_string(),
                description: "".to_string(),
                text: comment.text().to_string(),
            };
            tag.add_comment(comment)
        }
        if let Some(lyrics) = self.lyrics() {
            // TODO: Handle non-English lyrics.
            let lyrics = id3::frame::Lyrics {
                lang: "eng".to_string(),
                description: "".to_string(),
                text: lyrics.text().to_string(),
            };
            // TODO: As soon as the next version of id3 is released, update this to `add_lyrics`.
            tag.add_frame(Frame::with_content("USLT", Content::Lyrics(lyrics)));
        }
        // TODO: Return result.
        if let Ok(Image { data, format }) = self.cover() {
            let cover = id3::frame::Picture {
                mime_type: format.as_mime().to_string(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_string(),
                data,
            };
            tag.add_picture(cover);
        }

        // TODO: Remove unwraps & return Result.
        tag.write_to_path(self.path(), Version::Id3v24).unwrap();
    }

    pub fn update_id3_vw(&self) {
        let mut tag = Tag::new();
        tag.set_title(self.title().ascii());
        if !self.artists().is_empty() {
            tag.set_artist(self.artist().ascii());
        }
        tag.set_track(self.track_number as u32);
        if let Some(album_artist) = self.album_artist() {
            tag.set_album_artist(album_artist.ascii());
        }
        if !self.disc.is_only_disc() {
            tag.set_disc(self.disc.disc_number as u32);
        }
        tag.set_album(self.album().title().ascii());
        // TODO: Return result.
        if let Ok(Image { data, format }) = self.cover_vw() {
            let cover = id3::frame::Picture {
                mime_type: format.as_mime().to_string(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_string(),
                data,
            };
            tag.add_picture(cover);
        }

        // TODO: Remove unwraps & return result
        // TODO: Where do we write to?
        // tag.write_to_path(self.path(), Version::Id3v24).unwrap();
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
