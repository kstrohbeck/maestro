use crate::{
    models::disc::{Disc, DiscInContext},
    text::Text,
    utils::comma_separated,
};
use std::path::{Path, PathBuf};

pub struct Album {
    title: Text,
    artists: Vec<Text>,
    pub year: Option<usize>,
    genre: Option<Text>,
    discs: Vec<Disc>,
    path: PathBuf,
}

impl Album {
    pub fn title(&self) -> &Text {
        &self.title
    }

    pub fn artists(&self) -> &[Text] {
        &self.artists[..]
    }

    pub fn artist(&self) -> Text {
        comma_separated(self.artists())
    }

    pub fn genre(&self) -> Option<&Text> {
        self.genre.as_ref()
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

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn cache_path(&self) -> PathBuf {
        self.path().join("extras/.cache")
    }

    pub fn cover_path(&self) -> Option<PathBuf> {
        ["png", "jpg", "jpeg"]
            .iter()
            .map(|ext| {
                let fname = format!("Front Cover.{}", ext);
                self.cache_path().join(fname)
            })
            .find(|p| p.exists())
    }
}
