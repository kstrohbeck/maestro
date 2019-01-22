use crate::{
    image::{transform_image, transform_image_vw, Image, ImageError},
    models::disc::{Disc, DiscInContext},
    text::Text,
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

    pub fn from_yaml_and_path(yaml: Yaml, path: PathBuf) -> Option<Album> {
        let mut hash = yaml.into_hash()?;

        let title = pop!(hash["title"]).and_then(Text::from_yaml)?;

        // TODO: Abstract this plural/singular pattern into a util.
        let artists = match pop!(hash["artists"]) {
            Some(artists) => artists
                .into_vec()?
                .into_iter()
                .map(Text::from_yaml)
                .collect::<Option<Vec<_>>>(),
            None => pop!(hash["artist"])
                .and_then(Text::from_yaml)
                .map(|t| vec![t]),
        }?;

        let discs = match pop!(hash["discs"]) {
            Some(discs) => Some(
                discs
                    .into_vec()?
                    .into_iter()
                    .map(Disc::from_yaml)
                    .collect::<Option<Vec<_>>>()?,
            ),
            None => match pop!(hash["tracks"]) {
                Some(tracks) => Some(vec![Disc::from_yaml(tracks)?]),
                None => None,
            },
        }?;

        let year = pop!(hash["year"])
            .and_then(Yaml::into_i64)
            .map(|y| y as usize);

        let genre = pop!(hash["genre"]).and_then(Text::from_yaml);

        Some(Album {
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
}
