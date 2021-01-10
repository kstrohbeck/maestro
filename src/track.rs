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
    borrow::Cow,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

pub struct Track<'a> {
    disc: Cow<'a, Disc<'a>>,
    track: &'a raw::Track,
    pub track_number: usize,
    cover: OnceCell<Option<Image>>,
    cover_vw: OnceCell<Option<Image>>,
}

impl<'a> Track<'a> {
    pub fn new(disc: Cow<'a, Disc<'a>>, track: &'a raw::Track, track_number: usize) -> Track<'a> {
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
        self.track.year.or_else(|| self.album().year())
    }

    pub fn genre(&self) -> Option<&Text> {
        self.track.genre().or_else(|| self.album().genre())
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
        &self.disc
    }

    pub fn canonical_filename(&self) -> String {
        let digits = num_digits(self.disc().num_tracks());
        format!(
            "{:0width$} - {}.mp3",
            self.track_number,
            self.title().file_safe(),
            width = digits,
        )
    }

    pub fn filename(&self) -> Cow<str> {
        match self.track.filename() {
            Some(filename) => filename.into(),
            None => self.canonical_filename().into(),
        }
    }

    pub fn filename_vw(&self) -> String {
        if self.album().num_discs() == 1 {
            return self.canonical_filename();
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

    pub fn canonical_path(&self) -> PathBuf {
        self.disc().path().join(self.canonical_filename())
    }

    pub fn path(&self) -> PathBuf {
        match self.filename() {
            Cow::Borrowed(filename) => self.album().path().join(filename),
            Cow::Owned(filename) => self.disc().path().join(filename),
        }
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
            || self.disc().cover(),
        )
    }

    pub fn cover_vw(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.get_cover(
            &self.cover_vw,
            self.album().covers_vw_path(),
            img::transform_image_vw,
            || self.disc().cover_vw(),
        )
    }

    pub fn validate(&self) -> Result<(), Vec<ValidateError>> {
        let tag =
            Tag::read_from_path(self.path()).map_err(|e| vec![ValidateError::CouldntReadTag(e)])?;

        let mut errors = Vec::new();

        macro_rules! push_err {
            ( $e:expr ) => {
                if let Some(err) = $e {
                    errors.push(err);
                }
            };
        }

        push_err! {
            match tag.title() {
                None => Some(ValidateError::MissingFrame("title")),
                Some(title) if title != self.title().value() => {
                    Some(ValidateError::IncorrectDataInFrame("title", title.to_string()))
                }
                _ => None,
            }
        }

        push_err! {
            match (
                !self.artists().is_empty(),
                self.artist().value(),
                tag.artist(),
            ) {
                (false, _, Some(_)) => Some(ValidateError::UnexpectedFrame("artist")),
                (true, _, None) => Some(ValidateError::MissingFrame("artist")),
                (_, artist, Some(t_artist)) if artist != t_artist => {
                    Some(ValidateError::IncorrectDataInFrame("artist", t_artist.to_string()))
                }
                _ => None,
            }
        }

        push_err! {
            match tag.track() {
                None => Some(ValidateError::MissingFrame("track")),
                Some(track) if track != self.track_number as u32 => {
                    Some(ValidateError::IncorrectDataInFrame("track", track.to_string()))
                }
                _ => None,
            }
        }

        push_err! {
            match (self.album_artist(), tag.album_artist()) {
                (Some(_), None) => Some(ValidateError::MissingFrame("album artist")),
                (None, Some(_)) => Some(ValidateError::UnexpectedFrame("album artist")),
                (Some(ref a), Some(b)) if a.value() != b => {
                    Some(ValidateError::IncorrectDataInFrame("album artist", b.to_string()))
                }
                _ => None,
            }
        }

        push_err! {
            match (
                !self.disc().is_only_disc(),
                self.disc().disc_number as u32,
                tag.disc(),
            ) {
                (false, _, Some(_)) => Some(ValidateError::UnexpectedFrame("disc")),
                (true, _, None) => Some(ValidateError::MissingFrame("disc")),
                (true, disc, Some(t_disc)) if disc != t_disc => {
                    Some(ValidateError::IncorrectDataInFrame("disc", t_disc.to_string()))
                }
                _ => None,
            }
        }

        push_err! {
            match tag.album() {
                None => Some(ValidateError::MissingFrame("album")),
                Some(album) if album != self.album().title().value() => {
                    Some(ValidateError::IncorrectDataInFrame("album", album.to_string()))
                }
                _ => None,
            }
        }

        push_err! {
            match (self.id3_date_recorded(), tag.date_recorded()) {
                (None, Some(_)) => Some(ValidateError::UnexpectedFrame("year")),
                (Some(_), None) => Some(ValidateError::MissingFrame("year")),
                // TODO: Does comparing date_recordeds work?
                (Some(a), Some(b)) if a != b => {
                    Some(ValidateError::IncorrectDataInFrame("year", b.to_string()))
                }
                _ => None,
            }
        }

        push_err! {
            match (self.genre().map(Text::value), tag.genre()) {
                (None, Some(_)) => Some(ValidateError::UnexpectedFrame("genre")),
                (Some(_), None) => Some(ValidateError::MissingFrame("genre")),
                (Some(a), Some(b)) if a != b => {
                    Some(ValidateError::IncorrectDataInFrame("genre", b.to_string()))
                }
                _ => None,
            }
        }

        push_err! {
            match (self.id3_comment(), tag.comments().next()) {
                (None, Some(_)) => Some(ValidateError::UnexpectedFrame("comments")),
                (Some(_), None) => Some(ValidateError::MissingFrame("comments")),
                // TODO: Does comparing comments work?
                (Some(ref a), Some(b)) if a != b => {
                    Some(ValidateError::IncorrectDataInFrame("comments", format!("{:?}", b)))
                }
                _ => None,
            }
        }

        push_err! {
            match (self.id3_lyrics(), tag.lyrics().next()) {
                (None, Some(_)) => Some(ValidateError::UnexpectedFrame("lyrics")),
                (Some(_), None) => Some(ValidateError::MissingFrame("lyrics")),
                // TODO: Does comparing lyrics work?
                (Some(ref a), Some(b)) if a != b => {
                    Some(ValidateError::IncorrectDataInFrame("lyrics", format!("{:?}", b)))
                }
                _ => None,
            }
        }

        push_err! {
            match self.cover_id3_picture() {
                Ok(cover) => match (cover, tag.pictures().next()) {
                    (None, Some(_)) => Some(ValidateError::UnexpectedFrame("cover")),
                    (Some(_), None) => Some(ValidateError::MissingFrame("cover")),
                    // TODO: Does comparing pictures work?
                    (Some(ref a), Some(b)) if a != b => {
                        Some(ValidateError::IncorrectDataInFrame("cover", String::from("...")))
                    }
                    _ => None,
                },
                Err(err) => Some(ValidateError::CouldntLoadCover(err)),
            }
        }

        // TODO: Check for duplicate and erroneous frames.

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn clear(&self) -> Result<(), UpdateId3Error> {
        // Check if the file exists before trying to create a tag.
        let path = self.path();
        if !path.exists() {
            return Err(UpdateId3Error::FileNotFound);
        }

        // Remove the old tag.
        // TODO: Remove unwraps.
        // TODO: This seems redundant with the path check? We should remove that.
        // TODO: See if we can avoid doing this.
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(&path)
            .unwrap();
        Tag::remove_from(&mut file).unwrap();

        Ok(())
    }

    pub fn update_id3(&self) -> Result<(), UpdateId3Error> {
        // TODO: Use clear?
        // Check if the file exists before trying to create a tag.
        let path = self.path();
        if !path.exists() {
            return Err(UpdateId3Error::FileNotFound);
        }

        // Remove the old tag.
        // TODO: Remove unwraps.
        // TODO: This seems redundant with the path check? We should remove that.
        // TODO: See if we can avoid doing this.
        self.clear()?;

        let mut tag = Tag::new();

        tag.set_title(self.title().value());

        if !self.artists().is_empty() {
            tag.set_artist(self.artist().value());
        }

        tag.set_track(self.track_number as u32);

        if let Some(album_artist) = self.album_artist() {
            tag.set_album_artist(album_artist.value());
        }

        if !self.disc().is_only_disc() {
            tag.set_disc(self.disc().disc_number as u32);
        }

        tag.set_album(self.album().title().value());

        if let Some(date_recorded) = self.id3_date_recorded() {
            tag.set_date_recorded(date_recorded);
        }

        if let Some(genre) = self.genre() {
            tag.set_genre(genre.value());
        }

        if let Some(comment) = self.id3_comment() {
            tag.add_comment(comment);
        }

        if let Some(lyrics) = self.id3_lyrics() {
            tag.add_lyrics(lyrics);
        }

        if let Some(picture) = self
            .cover_id3_picture()
            .map_err(UpdateId3Error::CoverError)?
        {
            tag.add_picture(picture);
        }

        tag.write_to_path(path, Version::Id3v24)
            .map_err(UpdateId3Error::WriteError)
    }

    pub fn export<P: AsRef<Path>>(&self, folder: P) -> Result<(), UpdateId3VwError> {
        let orig_path = self.path();
        if !orig_path.exists() {
            return Err(UpdateId3VwError::FileNotFound);
        }

        let mut folder: Cow<Path> = folder.as_ref().into();
        if !folder.exists() {
            return Err(UpdateId3VwError::FolderNotFound);
        }

        // Check for disc, if it's needed.
        if let Some(disc) = self.disc().filename() {
            folder = folder.join(disc).into();
            // TODO: Remove unwrap.
            std::fs::create_dir_all(&folder).unwrap();
        }

        let path = folder.join(&self.filename_vw());
        fs::copy(orig_path, &path).map_err(UpdateId3VwError::CopyError)?;

        Ok(())
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
        let path = folder.join(&self.filename_vw() as &str);
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

    fn id3_date_recorded(&self) -> Option<id3::Timestamp> {
        self.year().map(|year| id3::Timestamp {
            year: year as i32,
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
        })
    }

    fn id3_comment(&self) -> Option<id3::frame::Comment> {
        self.comment().map(|comment| id3::frame::Comment {
            lang: "eng".to_string(),
            description: "".to_string(),
            text: comment.value().to_string(),
        })
    }

    fn id3_lyrics(&self) -> Option<id3::frame::Lyrics> {
        // TODO: Handle non-English lyrics.
        self.lyrics().map(|lyrics| id3::frame::Lyrics {
            lang: "eng".to_string(),
            description: "".to_string(),
            text: lyrics.value().to_string(),
        })
    }

    fn cover_id3_picture(&self) -> Result<Option<id3::frame::Picture>, LoadWithCacheError> {
        Ok(self.cover()?.map(|img| id3::frame::Picture {
            mime_type: img.format.mime().to_string(),
            picture_type: id3::frame::PictureType::CoverFront,
            description: "".to_string(),
            data: img.data.clone(),
        }))
    }
}

// TODO: Implement Error.
#[derive(Debug)]
pub enum ValidateError {
    CouldntReadTag(id3::Error),
    MissingFrame(&'static str),
    DuplicateFrame(&'static str),
    IncorrectDataInFrame(&'static str, String),
    UnexpectedFrame(&'static str),
    CouldntLoadCover(LoadWithCacheError),
}

// TODO: Implement Error.
#[derive(Debug)]
pub enum UpdateId3Error {
    FileNotFound,
    CoverError(LoadWithCacheError),
    WriteError(id3::Error),
}

// TODO: Implement Error.
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
            .with_artists(vec![Text::from("a"), Text::from(("b", "c"))])
            .with_discs(vec![raw::Disc::from_tracks(vec![raw::Track::new("song")])]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        let track = disc.track(1).unwrap();
        assert_eq!(&[Text::from("a"), Text::from(("b", "c"))], track.artists());
    }

    #[test]
    fn artists_are_overridden_by_track() {
        let album = raw::Album::new("foo")
            .with_artists(vec![Text::from("a"), Text::from(("b", "c"))])
            .with_discs(vec![raw::Disc::from_tracks(vec![
                raw::Track::new("song").with_artists(vec![Text::from("d")])
            ])]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        let track = disc.track(1).unwrap();
        assert_eq!(&[Text::from("d")], track.artists());
    }

    #[test]
    fn no_album_artists_without_override() {
        let album = raw::Album::new("foo")
            .with_artists(vec![Text::from("a"), Text::from(("b", "c"))])
            .with_discs(vec![raw::Disc::from_tracks(vec![raw::Track::new("song")])]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        let track = disc.track(1).unwrap();
        assert!(track.album_artists().is_none());
    }

    #[test]
    fn album_artists_are_set_when_overridden() {
        let album = raw::Album::new("foo")
            .with_artists(vec![Text::from("a"), Text::from(("b", "c"))])
            .with_discs(vec![raw::Disc::from_tracks(vec![
                raw::Track::new("song").with_artists(vec![Text::from("d")])
            ])]);
        let album = Album::new(album, PathBuf::from("."));
        let disc = album.disc(1).unwrap();
        let track = disc.track(1).unwrap();
        assert_eq!(
            Some(&[Text::from("a"), Text::from(("b", "c"))][..]),
            track.album_artists(),
        );
    }
}
