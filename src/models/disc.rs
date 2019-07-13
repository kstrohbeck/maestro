use crate::{
    models::{
        album::Album,
        track::{self, Track, TrackInContext},
    },
    utils::num_digits,
};
use std::{borrow::Cow, fmt, path::Path};
use yaml_rust::Yaml;

#[derive(Debug, Default)]
pub struct Disc {
    tracks: Vec<Track>,
}

impl Disc {
    pub fn new() -> Disc {
        Default::default()
    }

    pub fn from_tracks(tracks: Vec<Track>) -> Disc {
        Disc { tracks }
    }

    pub fn from_yaml(yaml: Yaml) -> Result<Disc, FromYamlError> {
        let tracks = yaml
            .into_vec()
            .ok_or(FromYamlError::InvalidTracks)?
            .into_iter()
            .map(Track::from_yaml)
            .collect::<Result<Vec<_>, _>>()
            .map_err(FromYamlError::InvalidTrack)?;
        Ok(Disc::from_tracks(tracks))
    }

    pub fn num_tracks(&self) -> usize {
        self.tracks.len()
    }

    pub fn push_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn with_track(mut self, track: Track) -> Self {
        self.tracks.push(track);
        self
    }
}

#[derive(Clone, Debug)]
pub enum FromYamlError {
    InvalidTracks,
    InvalidTrack(track::FromYamlError),
}

impl fmt::Display for FromYamlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FromYamlError::InvalidTracks => write!(f, "invalid tracks"),
            FromYamlError::InvalidTrack(e) => write!(f, "invalid track: {}", e),
        }
    }
}

impl std::error::Error for FromYamlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FromYamlError::InvalidTracks => None,
            FromYamlError::InvalidTrack(e) => Some(e),
        }
    }
}

pub struct DiscInContext<'a> {
    pub album: &'a Album,
    disc: &'a Disc,
    pub disc_number: usize,
}

impl<'a> DiscInContext<'a> {
    pub fn new(album: &'a Album, disc: &'a Disc, disc_number: usize) -> DiscInContext<'a> {
        DiscInContext {
            album,
            disc,
            disc_number,
        }
    }

    pub fn num_tracks(&self) -> usize {
        self.disc.num_tracks()
    }

    pub fn track(&self, track_number: usize) -> TrackInContext<&DiscInContext> {
        TrackInContext::new(self, &self.disc.tracks[track_number - 1], track_number)
    }

    pub fn into_track(self, track_number: usize) -> TrackInContext<'a, DiscInContext<'a>> {
        let track = &self.disc.tracks[track_number - 1];
        TrackInContext::new(self, track, track_number)
    }

    pub fn tracks(
        &'a self,
    ) -> impl Iterator<Item = TrackInContext<'a, &'a DiscInContext<'a>>> + 'a {
        self.disc
            .tracks
            .iter()
            .zip(1..)
            .map(move |(t, i)| TrackInContext::new(self, t, i))
    }

    fn filename(&self) -> Option<String> {
        if self.is_only_disc() {
            None
        } else {
            let digits = num_digits(self.album.num_discs());
            Some(format!("Disc {:0width$}", self.disc_number, width = digits))
        }
    }

    pub fn path(&self) -> Cow<Path> {
        match self.filename() {
            None => self.album.path().into(),
            Some(name) => self.album.path().join(name).into(),
        }
    }

    pub fn is_only_disc(&self) -> bool {
        self.album.num_discs() == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Text;
    use std::path::PathBuf;
    use yaml_rust::YamlLoader;

    #[test]
    fn only_disc_has_empty_filename() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_disc(Disc::new());
        let disc = album.disc(1);
        assert_eq!(disc.filename(), None);
    }

    #[test]
    fn first_disc_is_named_correctly() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_disc(Disc::new());
        album.push_disc(Disc::new());
        let disc = album.disc(1);
        assert_eq!(disc.filename().unwrap(), "Disc 1");
    }

    #[test]
    fn second_disc_is_named_correctly() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_disc(Disc::new());
        album.push_disc(Disc::new());
        let disc = album.disc(2);
        assert_eq!(disc.filename().unwrap(), "Disc 2");
    }

    #[test]
    fn only_disc_has_same_path_as_album() {
        let mut album = Album::new("title", PathBuf::from("."));
        album.push_disc(Disc::new());
        let disc = album.disc(1);
        assert_eq!(disc.path(), album.path());
    }

    macro_rules! yaml_to_disc {
        ($s:expr) => {
            Disc::from_yaml(YamlLoader::load_from_str($s).unwrap().pop().unwrap())
        };
    }

    // TODO: Put PartialEq back on tracks.
    /*
    #[test]
    fn from_yaml_has_tracks() {
        let disc = yaml_to_disc!(
            "
            - foo
            - title:
                text: bar
                ascii: baz
            - title: quux
              artists:
                - a
                - b
            "
        )
        .unwrap();
        let tracks = vec![
            Track::new("foo"),
            Track::new(Text::with_ascii("bar", "baz")),
            {
                let mut track = Track::new("quux");
                track.push_artist("a");
                track.push_artist("b");
                track
            },
        ];

        assert_eq!(tracks, disc.tracks);
    }
    */
}
