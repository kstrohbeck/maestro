use super::{album::Album, disc::Disc};
use crate::{
    image::{self as img, Image, LoadWithCacheError},
    raw,
    utils::{comma_separated, num_digits},
    Text,
};
use id3::{Tag, Version};
use once_cell::sync::OnceCell;
use std::{
    borrow::{Borrow, Cow},
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

pub struct Track<'a, T>
where
    T: Borrow<Disc<'a>>,
{
    pub disc: T,
    track: &'a raw::Track,
    pub track_number: usize,
    cover: OnceCell<Option<Image>>,
    cover_vw: OnceCell<Option<Image>>,
}

impl<'a, T> Track<'a, T>
where
    T: Borrow<Disc<'a>>,
{
    pub fn new(disc: T, track: &'a raw::Track, track_number: usize) -> Track<'a, T> {
        Track {
            disc,
            track,
            track_number,
            cover: OnceCell::new(),
            cover_vw: OnceCell::new(),
        }
    }

    pub fn title(&self) -> &Text {
        &self.track.title
    }

    pub fn artists(&self) -> &[Text] {
        self.track
            .artists()
            .unwrap_or_else(|| self.album().artists())
    }

    pub fn artist(&self) -> Cow<Text> {
        self.track
            .artists()
            .map(comma_separated)
            .unwrap_or_else(|| self.album().artist())
    }

    pub fn album_artists(&self) -> Option<&[Text]> {
        let album_artists = self.album().artists();
        if self.artists() != album_artists {
            Some(album_artists)
        } else {
            None
        }
    }

    pub fn album_artist(&self) -> Option<Cow<Text>> {
        self.album_artists().map(comma_separated)
    }

    pub fn year(&self) -> Option<usize> {
        self.track.year
    }

    pub fn genre(&self) -> Option<&Text> {
        self.track.genre()
    }

    pub fn comment(&self) -> Option<&Text> {
        self.track.comment()
    }

    pub fn lyrics(&self) -> Option<&Text> {
        self.track.lyrics()
    }

    pub fn album(&self) -> &Album {
        self.disc().album
    }

    pub fn disc(&self) -> &Disc {
        self.disc.borrow()
    }

    pub fn filename(&self) -> String {
        let digits = num_digits(self.disc().num_tracks());
        format!(
            "{:0width$} - {}.mp3",
            self.track_number,
            self.title().file_safe(),
            width = digits,
        )
    }

    pub fn filename_vw(&self) -> String {
        if self.album().num_discs() == 1 {
            return self.filename();
        }
        let disc_digits = num_digits(self.album().num_discs());
        let track_digits = num_digits(self.disc().num_tracks());
        format!(
            "{:0disc_width$}-{:0track_width$} - {}.mp3",
            self.disc().disc_number,
            self.track_number,
            self.title().file_safe(),
            disc_width = disc_digits,
            track_width = track_digits,
        )
    }

    pub fn path(&self) -> PathBuf {
        self.disc().path().join(self.filename())
    }

    pub fn exists(&self) -> bool {
        self.path().exists()
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
                Image::try_load_with_cache(
                    self.album().image_path(),
                    covers_path,
                    &self.track.title.file_safe(),
                    transform,
                )
            })
            .and_then(|o| match o {
                Some(x) => Ok(Some(x)),
                None => fallback(),
            })
    }

    pub fn cover(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.get_cover(
            &self.cover,
            self.album().covers_path(),
            img::transform_image,
            || self.album().cover(),
        )
    }

    pub fn cover_vw(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.get_cover(
            &self.cover_vw,
            self.album().covers_vw_path(),
            img::transform_image_vw,
            || self.album().cover_vw(),
        )
    }

    pub fn update_id3(&self) -> Result<(), UpdateId3Error> {
        use id3::{Content, Frame};

        // Check if the file exists before trying to create a tag.
        let path = self.path();
        if !path.exists() {
            return Err(UpdateId3Error::FileNotFound);
        }

        // Remove the old tag.
        // TODO: Remove unwraps.
        // TODO: See if we can avoid doing this.
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(&path)
            .unwrap();
        Tag::remove_from(&mut file).unwrap();

        let mut tag = Tag::new();
        tag.set_title(self.title().text());
        if !self.artists().is_empty() {
            tag.set_artist(self.artist().text());
        }
        tag.set_track(self.track_number as u32);
        if let Some(album_artist) = self.album_artist() {
            tag.set_album_artist(album_artist.text());
        }
        if !self.disc().is_only_disc() {
            tag.set_disc(self.disc().disc_number as u32);
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

        if let Some(Image {
            ref data,
            ref format,
        }) = self.cover().map_err(UpdateId3Error::CoverError)?
        {
            let cover = id3::frame::Picture {
                mime_type: format.mime().to_string(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_string(),
                data: data.clone(),
            };
            tag.add_picture(cover);
        }

        tag.write_to_path(path, Version::Id3v24)
            .map_err(UpdateId3Error::WriteError)
    }

    pub fn update_id3_vw<P: AsRef<Path>>(&self, folder: P) -> Result<(), UpdateId3VwError> {
        let orig_path = self.path();
        if !orig_path.exists() {
            return Err(UpdateId3VwError::FileNotFound);
        }

        let folder = folder.as_ref();
        if !folder.exists() {
            return Err(UpdateId3VwError::FolderNotFound);
        }

        // Copy file to destination.
        let path = folder.join(self.filename_vw());
        fs::copy(orig_path, &path).map_err(UpdateId3VwError::CopyError)?;

        // Remove the old tag.
        // TODO: Remove unwraps.
        // TODO: See if we can avoid doing this.
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(&path)
            .unwrap();
        Tag::remove_from(&mut file).unwrap();

        let mut tag = Tag::new();
        tag.set_title(self.title().ascii());
        if !self.artists().is_empty() {
            tag.set_artist(self.artist().ascii());
        }
        tag.set_track(self.track_number as u32);
        if let Some(album_artist) = self.album_artist() {
            tag.set_album_artist(album_artist.ascii());
        }
        if !self.disc().is_only_disc() {
            tag.set_disc(self.disc().disc_number as u32);
        }
        tag.set_album(self.album().title().ascii());

        if let Some(Image { data, format }) =
            self.cover_vw().map_err(UpdateId3VwError::CoverError)?
        {
            let cover = id3::frame::Picture {
                mime_type: format.mime().to_string(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_string(),
                data: data.clone(),
            };
            tag.add_picture(cover);
        }

        tag.write_to_path(path, Version::Id3v24)
            .map_err(UpdateId3VwError::WriteError)
    }
}

#[derive(Debug)]
pub enum UpdateId3Error {
    FileNotFound,
    CoverError(LoadWithCacheError),
    WriteError(id3::Error),
}

#[derive(Debug)]
pub enum UpdateId3VwError {
    FileNotFound,
    FolderNotFound,
    CopyError(std::io::Error),
    CoverError(LoadWithCacheError),
    WriteError(id3::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artists_are_inherited_from_album() {
        let album = raw::Album::new("foo")
            .with_artists(vec![Text::new("a"), Text::with_ascii("b", "c")])
            .with_discs(vec![raw::Disc::from_tracks(vec![raw::Track::new("song")])]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        let track = disc.track(1).unwrap();
        assert_eq!(
            &[Text::new("a"), Text::with_ascii("b", "c")],
            track.artists()
        );
    }

    #[test]
    fn artists_are_overridden_by_track() {
        let album = raw::Album::new("foo")
            .with_artists(vec![Text::new("a"), Text::with_ascii("b", "c")])
            .with_discs(vec![raw::Disc::from_tracks(vec![
                raw::Track::new("song").with_artists(vec![Text::new("d")])
            ])]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        let track = disc.track(1).unwrap();
        assert_eq!(&[Text::new("d")], track.artists());
    }

    #[test]
    fn no_album_artists_without_override() {
        let album = raw::Album::new("foo")
            .with_artists(vec![Text::new("a"), Text::with_ascii("b", "c")])
            .with_discs(vec![raw::Disc::from_tracks(vec![raw::Track::new("song")])]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        let track = disc.track(1).unwrap();
        assert!(track.album_artists().is_none());
    }

    #[test]
    fn album_artists_are_set_when_overridden() {
        let album = raw::Album::new("foo")
            .with_artists(vec![Text::new("a"), Text::with_ascii("b", "c")])
            .with_discs(vec![raw::Disc::from_tracks(vec![
                raw::Track::new("song").with_artists(vec![Text::new("d")])
            ])]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        let track = disc.track(1).unwrap();
        assert_eq!(
            Some(&[Text::new("a"), Text::with_ascii("b", "c")][..]),
            track.album_artists(),
        );
    }
}
