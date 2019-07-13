use crate::{
    image::{transform_image, transform_image_vw, Image, LoadWithCacheError},
    models::{
        disc::{self, Disc, DiscInContext},
        track::{TrackInContext, UpdateId3Error, UpdateId3VwError},
    },
    text::{self, Text},
    utils::{
        comma_separated, parse_key_from_hash, parse_singular_or_plural, try_parse_key_from_hash,
        ParseKeyError, ParseSingularOrPluralError,
    },
};
use once_cell::sync::OnceCell;
use std::{
    fmt,
    path::{Path, PathBuf},
};
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct Album {
    title: Text,
    artists: Vec<Text>,
    pub year: Option<usize>,
    genre: Option<Text>,
    discs: Vec<Disc>,
    path: PathBuf,
    cover: OnceCell<Option<Image>>,
    cover_vw: OnceCell<Option<Image>>,
}

impl Album {
    /// Create a new album with only essential information.
    pub fn new<T>(title: T, path: PathBuf) -> Album
    where
        T: Into<Text>,
    {
        Album {
            title: title.into(),
            artists: Vec::new(),
            year: None,
            genre: None,
            discs: Vec::new(),
            path,
            cover: OnceCell::new(),
            cover_vw: OnceCell::new(),
        }
    }

    /// Load an album's data from YAML.
    ///
    /// # Examples
    ///
    /// ```
    /// # use songmaster_rs::models::album::Album;
    /// use std::path::PathBuf;
    /// use songmaster_rs::text::Text;
    /// use yaml_rust::YamlLoader;
    ///
    /// let yaml = YamlLoader::load_from_str(
    ///     "
    /// title: Foo
    /// artist: Bar
    /// tracks:
    ///   - Track 1
    ///   - title:
    ///       text: Track 2❤️
    ///       ascii: Track 2
    ///     "
    /// )?
    ///     .pop()
    ///     .ok_or("no yaml found")?;
    ///
    /// let album = Album::from_yaml_and_path(yaml, PathBuf::from("."))?;
    /// assert_eq!(&Text::new("Foo"), album.title());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_yaml_and_path(yaml: Yaml, path: PathBuf) -> Result<Album, FromYamlError> {
        let mut hash = yaml.into_hash().ok_or(FromYamlError::NotHash)?;

        let title =
            parse_key_from_hash(&mut hash, "title", Text::from_yaml).map_err(|e| match e {
                ParseKeyError::KeyNotFound => FromYamlError::MissingTitle,
                ParseKeyError::InvalidValue(v) => FromYamlError::InvalidTitle(v),
            })?;

        let artists = parse_singular_or_plural(&mut hash, "artist", "artists", Text::from_yaml)
            .map_err(|e| match e {
                ParseSingularOrPluralError::KeysNotFound => FromYamlError::MissingArtists,
                ParseSingularOrPluralError::NotAnArray(v) => FromYamlError::InvalidArtists(v),
                ParseSingularOrPluralError::InvalidValue(v) => FromYamlError::InvalidArtist(v),
            })?;

        let discs = parse_singular_or_plural(&mut hash, "tracks", "discs", Disc::from_yaml)
            .map_err(|e| match e {
                ParseSingularOrPluralError::KeysNotFound => FromYamlError::MissingDiscOrTracks,
                ParseSingularOrPluralError::NotAnArray(v) => FromYamlError::InvalidDiscs(v),
                ParseSingularOrPluralError::InvalidValue(v) => FromYamlError::InvalidDisc(v),
            })?;

        let year = try_parse_key_from_hash(&mut hash, "year", |y| match y {
            Yaml::Integer(n) => Ok(n as usize),
            yaml => Err(yaml),
        })
        .map_err(FromYamlError::InvalidYear)?;

        let genre = try_parse_key_from_hash(&mut hash, "genre", Text::from_yaml)
            .map_err(FromYamlError::InvalidGenre)?;

        Ok(Album {
            title,
            artists,
            year,
            genre,
            discs,
            path,
            cover: OnceCell::new(),
            cover_vw: OnceCell::new(),
        })
    }

    pub fn title(&self) -> &Text {
        &self.title
    }

    pub fn artists(&self) -> &[Text] {
        &self.artists[..]
    }

    pub fn push_artist<T>(&mut self, artist: T)
    where
        T: Into<Text>,
    {
        self.artists.push(artist.into());
    }

    pub fn with_artist<T>(mut self, artist: T) -> Self
    where
        T: Into<Text>,
    {
        self.push_artist(artist);
        self
    }

    pub fn artist(&self) -> Text {
        comma_separated(self.artists())
    }

    pub fn set_year(&mut self, year: usize) {
        self.year = Some(year);
    }

    pub fn with_year(mut self, year: usize) -> Self {
        self.year = Some(year);
        self
    }

    pub fn genre(&self) -> Option<&Text> {
        self.genre.as_ref()
    }

    pub fn set_genre<T>(&mut self, genre: T)
    where
        T: Into<Text>,
    {
        self.genre = Some(genre.into());
    }

    pub fn with_genre<T>(mut self, genre: T) -> Self
    where
        T: Into<Text>,
    {
        self.genre = Some(genre.into());
        self
    }

    pub fn num_discs(&self) -> usize {
        self.discs.len()
    }

    pub fn disc(&self, disc_number: usize) -> DiscInContext {
        DiscInContext::new(&self, &self.discs[disc_number - 1], disc_number)
    }

    pub fn discs(&self) -> impl Iterator<Item = DiscInContext> {
        self.discs
            .iter()
            .zip(1..)
            .map(move |(d, i)| DiscInContext::new(&self, d, i))
    }

    pub fn push_disc(&mut self, disc: Disc) {
        self.discs.push(disc);
    }

    pub fn with_disc(mut self, disc: Disc) -> Self {
        self.discs.push(disc);
        self
    }

    pub fn tracks(&self) -> impl Iterator<Item = TrackInContext<DiscInContext>> {
        Tracks::new(self)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn extras_path(&self) -> PathBuf {
        self.path().join("extras")
    }

    pub fn image_path(&self) -> PathBuf {
        let mut path = self.extras_path();
        path.push("images");
        path
    }

    pub fn cache_path(&self) -> PathBuf {
        let mut path = self.extras_path();
        path.push(".cache");
        path
    }

    pub fn covers_path(&self) -> PathBuf {
        let mut path = self.cache_path();
        path.push("covers");
        path
    }

    pub fn covers_vw_path(&self) -> PathBuf {
        let mut path = self.cache_path();
        path.push("covers-vw");
        path
    }

    pub fn cover(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.cover
            .get_or_try_init(|| {
                Image::try_load_with_cache(
                    self.image_path(),
                    self.covers_path(),
                    "Front Cover",
                    transform_image,
                )
            })
            .map(Option::as_ref)
    }

    pub fn cover_vw(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        self.cover
            .get_or_try_init(|| {
                Image::try_load_with_cache(
                    self.image_path(),
                    self.covers_vw_path(),
                    "Front Cover",
                    transform_image_vw,
                )
            })
            .map(Option::as_ref)
    }

    pub fn update_id3(&self) -> Result<(), Vec<UpdateId3Error>> {
        let errors = self
            .tracks()
            .map(|t| t.update_id3())
            .filter_map(Result::err)
            .collect::<Vec<_>>();

        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    pub fn update_id3_vw<P: AsRef<Path>>(&self, path: P) -> Result<(), Vec<UpdateId3VwError>> {
        let errors = self
            .tracks()
            .map(|t| t.update_id3_vw(&path))
            .filter_map(Result::err)
            .collect::<Vec<_>>();

        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
pub enum FromYamlError {
    NotHash,
    MissingTitle,
    InvalidTitle(text::FromYamlError),
    MissingArtists,
    InvalidArtists(Yaml),
    InvalidArtist(text::FromYamlError),
    InvalidDiscs(Yaml),
    InvalidDisc(disc::FromYamlError),
    MissingDiscOrTracks,
    InvalidYear(Yaml),
    InvalidGenre(text::FromYamlError),
}

impl fmt::Display for FromYamlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FromYamlError::NotHash => write!(f, "album must be a hash"),
            FromYamlError::MissingTitle => write!(f, "missing \"title\""),
            FromYamlError::InvalidTitle(e) => write!(f, "invalid title: {}", e),
            FromYamlError::MissingArtists => write!(f, "missing \"artists\""),
            FromYamlError::InvalidArtists(_) => write!(f, "invalid artists"),
            FromYamlError::InvalidArtist(e) => write!(f, "invalid artist: {}", e),
            FromYamlError::InvalidDiscs(_) => write!(f, "invalid discs"),
            FromYamlError::InvalidDisc(e) => write!(f, "invalid disc: {}", e),
            FromYamlError::MissingDiscOrTracks => write!(f, "missing \"discs\" or \"tracks\""),
            FromYamlError::InvalidYear(_) => write!(f, "invalid year"),
            FromYamlError::InvalidGenre(e) => write!(f, "invalid genre: {}", e),
        }
    }
}

impl std::error::Error for FromYamlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FromYamlError::InvalidTitle(e)
            | FromYamlError::InvalidArtist(e)
            | FromYamlError::InvalidGenre(e) => Some(e),
            FromYamlError::InvalidDisc(e) => Some(e),
            _ => None,
        }
    }
}

struct Tracks<'a> {
    album: &'a Album,
    disc_number: usize,
    track_number: usize,
}

impl<'a> Tracks<'a> {
    fn new(album: &'a Album) -> Self {
        Tracks {
            album,
            disc_number: 1,
            track_number: 1,
        }
    }
}

impl<'a> Iterator for Tracks<'a> {
    type Item = TrackInContext<'a, DiscInContext<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let disc = loop {
            if self.disc_number >= self.album.num_discs() {
                return None;
            }

            let disc = self.album.disc(self.disc_number);

            if self.track_number < disc.num_tracks() {
                break disc;
            }

            self.disc_number += 1;
            self.track_number = 1;
        };

        Some(disc.into_track(self.track_number))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artist_is_only_artist_in_list() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_artist(Text::with_ascii("b", "c"));
        let artist = album.artist();
        assert_eq!(artist, Text::with_ascii("b", "c"));
    }

    #[test]
    fn artist_is_comma_separated_if_multiple() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_artist("a");
        album.push_artist(Text::with_ascii("b", "c"));
        let artist = album.artist();
        assert_eq!(artist, Text::with_ascii("a, b", "a, c"));
    }

    macro_rules! yaml_to_album {
        ($s:expr, $path:expr ) => {
            Album::from_yaml_and_path(
                yaml_rust::YamlLoader::load_from_str($s)
                    .unwrap()
                    .pop()
                    .unwrap(),
                $path,
            )
        };
    }

    #[test]
    fn from_yaml_parses_title() {
        let album = yaml_to_album!(
            "
            title: foo
            artist: bar
            tracks:
                - a
                - b
            ",
            PathBuf::from("album")
        )
        .unwrap();
        assert_eq!(&Text::new("foo"), album.title());
    }
}
