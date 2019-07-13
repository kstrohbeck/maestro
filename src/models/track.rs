use crate::{
    image::{transform_image, transform_image_vw, Image, LoadWithCacheError},
    models::{album::Album, disc::DiscInContext},
    text::{self, Text},
    utils::{
        comma_separated, num_digits, parse_key_from_hash, parse_singular_or_plural,
        try_parse_key_from_hash, yaml_into_usize, ParseKeyError, ParseSingularOrPluralError,
    },
};
use id3::{frame::Content, Frame, Tag, Version};
use once_cell::sync::OnceCell;
use std::{
    borrow::Borrow,
    fmt,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};
use yaml_rust::Yaml;

/// A music track in an album.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    /// The title of the track.
    title: Text,

    /// A list of artists that created the track, or None if the album's artists should be used.
    artists: Option<Vec<Text>>,

    /// The year the track was created, or None if the album's year should be used.
    year: Option<usize>,

    /// The genre of the track, or None if the album's genre should be used.
    genre: Option<Text>,

    /// Any comments on the track.
    pub comment: Option<Text>,

    /// The track's lyrics.
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

    pub fn from_yaml(yaml: Yaml) -> Result<Self, FromYamlError> {
        match yaml {
            Yaml::String(title) => Ok(Track::new(title)),
            Yaml::Hash(mut hash) => {
                let title = parse_key_from_hash(&mut hash, "title", Text::from_yaml).map_err(
                    |e| match e {
                        ParseKeyError::KeyNotFound => FromYamlError::MissingTitle,
                        ParseKeyError::InvalidValue(y) => FromYamlError::InvalidTitle(y),
                    },
                )?;

                let artists =
                    match parse_singular_or_plural(&mut hash, "artist", "artists", Text::from_yaml)
                    {
                        Ok(artists) => Ok(Some(artists)),
                        Err(ParseSingularOrPluralError::KeysNotFound) => Ok(None),
                        Err(ParseSingularOrPluralError::NotAnArray(v)) => {
                            Err(FromYamlError::InvalidArtists(v))
                        }
                        Err(ParseSingularOrPluralError::InvalidValue(v)) => {
                            Err(FromYamlError::InvalidArtist(v))
                        }
                    }?;

                let year = try_parse_key_from_hash(&mut hash, "year", yaml_into_usize)
                    .map_err(FromYamlError::InvalidYear)?;

                let genre = try_parse_key_from_hash(&mut hash, "genre", Text::from_yaml)
                    .map_err(FromYamlError::InvalidGenre)?;

                let comment = try_parse_key_from_hash(&mut hash, "comment", Text::from_yaml)
                    .map_err(FromYamlError::InvalidComment)?;

                let lyrics = try_parse_key_from_hash(&mut hash, "lyrics", Text::from_yaml)
                    .map_err(FromYamlError::InvalidLyrics)?;

                Ok(Track {
                    title,
                    artists,
                    year,
                    genre,
                    comment,
                    lyrics,
                })
            }
            _ => Err(FromYamlError::InvalidTrack(yaml)),
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FromYamlError {
    MissingTitle,
    InvalidTitle(text::FromYamlError),
    InvalidArtists(Yaml),
    InvalidArtist(text::FromYamlError),
    InvalidYear(Yaml),
    InvalidGenre(text::FromYamlError),
    InvalidComment(text::FromYamlError),
    InvalidLyrics(text::FromYamlError),
    InvalidTrack(Yaml),
}

impl fmt::Display for FromYamlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FromYamlError::MissingTitle => write!(f, "missing title"),
            FromYamlError::InvalidTitle(e) => write!(f, "invalid title: {}", e),
            FromYamlError::InvalidArtists(y) => write!(f, "invalid artists: {:?}", y),
            FromYamlError::InvalidArtist(e) => write!(f, "invalid artist: {}", e),
            FromYamlError::InvalidYear(_) => write!(f, "year must be integer"),
            FromYamlError::InvalidGenre(e) => write!(f, "invalid genre: {}", e),
            FromYamlError::InvalidComment(e) => write!(f, "invalid comment: {}", e),
            FromYamlError::InvalidLyrics(e) => write!(f, "invalid lyrics: {}", e),
            FromYamlError::InvalidTrack(_) => write!(f, "track must be a string or hash"),
        }
    }
}

impl std::error::Error for FromYamlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FromYamlError::InvalidTitle(e)
            | FromYamlError::InvalidArtist(e)
            | FromYamlError::InvalidGenre(e)
            | FromYamlError::InvalidComment(e)
            | FromYamlError::InvalidLyrics(e) => Some(e),
            _ => None,
        }
    }
}

pub struct TrackInContext<'a, T>
where
    T: Borrow<DiscInContext<'a>>,
{
    pub disc: T,
    track: &'a Track,
    pub track_number: usize,
    cover: OnceCell<Option<Image>>,
    cover_vw: OnceCell<Option<Image>>,
}

impl<'a, T> TrackInContext<'a, T>
where
    T: Borrow<DiscInContext<'a>>,
{
    pub fn new(disc: T, track: &'a Track, track_number: usize) -> TrackInContext<'a, T> {
        TrackInContext {
            disc,
            track,
            track_number,
            cover: OnceCell::new(),
            cover_vw: OnceCell::new(),
        }
    }

    fn disc(&self) -> &DiscInContext {
        self.disc.borrow()
    }

    fn album(&self) -> &Album {
        self.disc().album
    }

    pub fn title(&self) -> &Text {
        self.track.title()
    }

    pub fn artists(&self) -> &[Text] {
        self.track
            .artists()
            .unwrap_or_else(|| self.album().artists())
    }

    pub fn artist(&self) -> Text {
        comma_separated(self.artists())
    }

    pub fn album_artists(&self) -> Option<&[Text]> {
        let album_artists = self.album().artists();

        if self.artists() != album_artists {
            Some(album_artists)
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

    pub fn cover(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.cover
            .get_or_try_init(|| {
                Image::try_load_with_cache(
                    self.album().image_path(),
                    self.album().covers_path(),
                    &self.track.title().file_safe(),
                    transform_image,
                )
            })
            // TODO: Are there combinators for this?
            .and_then(|o| match o {
                Some(x) => Ok(Some(x)),
                None => self.album().cover(),
            })
    }

    pub fn cover_vw(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.cover_vw
            .get_or_try_init(|| {
                Image::try_load_with_cache(
                    self.album().image_path(),
                    self.album().covers_vw_path(),
                    &self.track.title().file_safe(),
                    transform_image_vw,
                )
            })
            // TODO: Are there combinators for this?
            .and_then(|o| match o {
                Some(x) => Ok(Some(x)),
                None => self.album().cover_vw(),
            })
    }

    pub fn exists(&self) -> bool {
        self.path().exists()
    }

    pub fn update_id3(&self) -> Result<(), UpdateId3Error> {
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
    use crate::models::disc::Disc;
    use yaml_rust::YamlLoader;

    macro_rules! yaml_to_track {
        ($s:expr) => {
            Track::from_yaml(YamlLoader::load_from_str($s).unwrap().pop().unwrap())
        };
    }

    #[test]
    fn string_is_parsed_to_track_with_title() {
        let yaml = YamlLoader::load_from_str("\"foo\"").unwrap().pop().unwrap();
        let track = Track::from_yaml(yaml).unwrap();
        assert_eq!(track.title().text(), "foo");
    }

    #[test]
    fn simple_title_is_parsed_from_yaml() {
        let track = yaml_to_track!("title: foo").unwrap();
        assert_eq!(track.title(), &Text::new("foo"));
    }

    #[test]
    fn complex_title_is_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title:
                text: foo
                ascii: bar
            "
        )
        .unwrap();
        assert_eq!(track.title(), &Text::with_ascii("foo", "bar"));
    }

    #[test]
    fn single_simple_artist_is_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            artist: bar
            "
        )
        .unwrap();
        assert_eq!(track.artists(), Some(&[Text::new("bar")][..]));
    }

    #[test]
    fn single_complex_artist_is_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            artist:
                text: bar
                ascii: baz
            "
        )
        .unwrap();
        assert_eq!(track.artists(), Some(&[Text::with_ascii("bar", "baz")][..]));
    }

    #[test]
    fn array_in_artist_is_not_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            artist:
                - foo
                - bar
            "
        );
        assert!(track.is_err());
    }

    #[test]
    fn multi_simple_artists_are_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            artists:
                - bar
                - baz
            "
        )
        .unwrap();
        assert_eq!(
            track.artists(),
            Some(&[Text::new("bar"), Text::new("baz")][..])
        );
    }

    #[test]
    fn multi_mixed_artists_are_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            artists:
                - bar
                - text: baz
                  ascii: quux
            "
        )
        .unwrap();
        assert_eq!(
            track.artists(),
            Some(&[Text::new("bar"), Text::with_ascii("baz", "quux")][..])
        );
    }

    #[test]
    fn single_artist_in_multi_is_not_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            artists: bar
            "
        );
        assert!(track.is_err());
    }

    #[test]
    fn year_is_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            year: 1990
            "
        )
        .unwrap();
        assert_eq!(track.year, Some(1990));
    }

    #[test]
    fn simple_genre_is_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            genre: Music
            "
        )
        .unwrap();
        assert_eq!(track.genre, Some(Text::new("Music")));
    }

    #[test]
    fn complex_genre_is_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            genre:
                text: Music
                ascii: Not Music
            "
        )
        .unwrap();
        assert_eq!(track.genre, Some(Text::with_ascii("Music", "Not Music")));
    }

    #[test]
    fn simple_comment_is_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            comment: stuff
            "
        )
        .unwrap();
        assert_eq!(track.comment, Some(Text::new("stuff")));
    }

    #[test]
    fn complex_comment_is_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            comment:
                text: stuff
                ascii: other
            "
        )
        .unwrap();
        assert_eq!(track.comment, Some(Text::with_ascii("stuff", "other")));
    }

    #[test]
    fn simple_lyrics_are_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            lyrics: stuff
            "
        )
        .unwrap();
        assert_eq!(track.lyrics, Some(Text::new("stuff")));
    }

    #[test]
    fn complex_lyrics_are_parsed_from_yaml() {
        let track = yaml_to_track!(
            "
            title: foo
            lyrics:
                text: stuff
                ascii: other
            "
        )
        .unwrap();
        assert_eq!(track.lyrics, Some(Text::with_ascii("stuff", "other")));
    }

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
