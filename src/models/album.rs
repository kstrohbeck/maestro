use crate::{
    image::{transform_image, transform_image_vw, Image, ImageError},
    models::disc::{self, Disc, DiscInContext},
    text::{self, Text},
    utils::comma_separated,
};
use std::path::{Path, PathBuf};
use yaml_rust::Yaml;

pub struct Album {
    title: Text,
    artists: Vec<Text>,
    pub year: Option<usize>,
    genre: Option<Text>,
    discs: Vec<Disc>,
    path: PathBuf,
}

impl Album {
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
        }
    }

    pub fn from_yaml_and_path(yaml: Yaml, path: PathBuf) -> Result<Album, FromYamlError> {
        let mut hash = yaml.into_hash().ok_or(FromYamlError::NotHash)?;

        let title = {
            let yaml = pop!(hash["title"]).ok_or(FromYamlError::MissingTitle)?;
            Text::from_yaml(yaml).map_err(FromYamlError::InvalidTitle)?
        };

        // TODO: Abstract this plural/singular pattern into a util.
        let artists = match pop!(hash["artists"]) {
            Some(artists) => artists
                .into_vec()
                .ok_or(FromYamlError::InvalidArtists)?
                .into_iter()
                .map(Text::from_yaml)
                .collect::<Result<Vec<_>, _>>()
                .map_err(FromYamlError::InvalidArtist),
            None => {
                let yaml = pop!(hash["artist"]).ok_or(FromYamlError::MissingArtists)?;
                Text::from_yaml(yaml)
                    .map_err(FromYamlError::InvalidArtist)
                    .map(|v| vec![v])
            }
        }?;

        let discs = match pop!(hash["discs"]) {
            Some(discs) => Ok(discs
                .into_vec()
                .ok_or(FromYamlError::InvalidDiscs)?
                .into_iter()
                .map(Disc::from_yaml)
                .collect::<Result<Vec<_>, _>>()
                .map_err(FromYamlError::InvalidDisc)?),
            None => match pop!(hash["tracks"]) {
                Some(tracks) => Ok(vec![
                    Disc::from_yaml(tracks).map_err(FromYamlError::InvalidDisc)?
                ]),
                None => Err(FromYamlError::MissingDiscOrTracks),
            },
        }?;

        let year = pop!(hash["year"])
            .and_then(Yaml::into_i64)
            .map(|y| y as usize);

        let genre = pop!(hash["genre"])
            .map(Text::from_yaml)
            .transpose()
            .map_err(FromYamlError::InvalidGenre)?;

        Ok(Album {
            title,
            artists,
            year,
            genre,
            discs,
            path,
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

    pub fn cover(&self) -> Result<Image, ImageError> {
        Image::load_with_cache(
            self.image_path(),
            self.covers_path(),
            "Front Cover",
            transform_image,
        )
    }

    pub fn cover_vw(&self) -> Result<Image, ImageError> {
        Image::load_with_cache(
            self.image_path(),
            self.covers_vw_path(),
            "Front Cover",
            transform_image_vw,
        )
    }
}

#[derive(Clone, Debug)]
pub enum FromYamlError {
    NotHash,
    MissingTitle,
    InvalidTitle(text::FromYamlError),
    MissingArtists,
    InvalidArtists,
    InvalidArtist(text::FromYamlError),
    InvalidDiscs,
    InvalidDisc(disc::FromYamlError),
    MissingDiscOrTracks,
    InvalidYear,
    InvalidGenre(text::FromYamlError),
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
