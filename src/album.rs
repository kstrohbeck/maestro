use super::{disc::Disc, track::Track};
use crate::{
    image::{Image, LoadWithCacheError},
    raw,
    text::Text,
};
use once_cell::sync::OnceCell;
use std::{
    borrow::Cow,
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Album {
    album: raw::Album,
    path: PathBuf,
    cover: OnceCell<Option<Image>>,
    cover_vw: OnceCell<Option<Image>>,
}

impl Album {
    pub fn new<P: Into<PathBuf>>(album: raw::Album, path: P) -> Self {
        Self {
            album,
            path: path.into(),
            cover: OnceCell::new(),
            cover_vw: OnceCell::new(),
        }
    }

    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, AlbumLoadError> {
        use std::fs::File;
        let path = path.into();
        let definition = File::open(path.join("extras/album.yaml"))
            .map_err(AlbumLoadError::CouldntLoadDefinition)?;
        let album =
            serde_yaml::from_reader(definition).map_err(AlbumLoadError::InvalidDefinition)?;
        Ok(Self::new(album, path))
    }

    pub fn generate<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let album = raw::Album::generate(&path);
        /*
        let mut album = Self::new(album, path);
        for disc in &mut album.album.discs {
            for track in &mut disc.tracks {

            }
        }
        album
        */
        Self::new(album, path)
    }

    pub fn raw(&self) -> &raw::Album {
        &self.album
    }

    pub fn title(&self) -> &Text {
        &self.album.title
    }

    pub fn artists(&self) -> &[Text] {
        &self.album.artists
    }

    pub fn artist(&self) -> Cow<Text> {
        self.album.artist()
    }

    pub fn year(&self) -> Option<raw::AlbumYear> {
        self.album.year
    }

    pub fn genre(&self) -> Option<&Text> {
        self.album.genre()
    }

    pub fn num_discs(&self) -> usize {
        self.album.num_discs()
    }

    pub fn disc(&self, disc_number: usize) -> Option<Disc> {
        self.album
            .discs
            .get(disc_number - 1)
            .map(|disc| Disc::new(self, disc, disc_number))
    }

    pub fn discs(&self) -> impl Iterator<Item = Disc> {
        self.album
            .discs
            .iter()
            .zip(1..)
            .map(move |(disc, disc_number)| Disc::new(self, disc, disc_number))
    }

    pub fn num_tracks(&self) -> usize {
        self.album.num_tracks()
    }

    pub fn tracks(&self) -> Tracks {
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

    fn get_cover<'a, P, F>(
        &'a self,
        cover: &'a OnceCell<Option<Image>>,
        covers_path: P,
        transform: F,
    ) -> Result<Option<&'a Image>, LoadWithCacheError>
    where
        P: AsRef<Path>,
        F: Fn(image::DynamicImage) -> Result<Image, image::ImageError>,
    {
        cover
            .get_or_try_init(|| {
                Image::try_load_with_cache(self.image_path(), covers_path, "Front Cover", transform)
            })
            .map(Option::as_ref)
    }

    pub fn cover(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        use crate::image::transform_image;
        self.get_cover(&self.cover, self.covers_path(), transform_image)
    }

    pub fn cover_vw(&self) -> Result<Option<&Image>, LoadWithCacheError> {
        use crate::image::transform_image_vw;
        self.get_cover(&self.cover_vw, self.covers_vw_path(), transform_image_vw)
    }

    pub fn save(&mut self) -> Result<(), ()> {
        todo!()
    }
}

#[derive(Debug)]
pub enum AlbumLoadError {
    CouldntLoadDefinition(std::io::Error),
    InvalidDefinition(serde_yaml::Error),
}

impl fmt::Display for AlbumLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AlbumLoadError::CouldntLoadDefinition(e) => {
                write!(f, "couldn't load definition: {}", e)
            }
            AlbumLoadError::InvalidDefinition(e) => write!(f, "invalid definition: {}", e),
        }
    }
}

impl std::error::Error for AlbumLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AlbumLoadError::CouldntLoadDefinition(e) => Some(e),
            AlbumLoadError::InvalidDefinition(e) => Some(e),
        }
    }
}

pub struct Tracks<'a> {
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
    type Item = Track<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let disc = self.album.disc(self.disc_number)?;
            match disc.into_track(self.track_number) {
                None => {
                    self.disc_number += 1;
                    self.track_number = 1;
                }
                Some(track) => {
                    self.track_number += 1;
                    break Some(track);
                }
            }
        }
    }
}
