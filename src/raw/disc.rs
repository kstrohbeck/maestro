use super::track::Track;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
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

    pub fn tracks(&self) -> &[Track] {
        &self.tracks[..]
    }

    pub fn tracks_mut(&mut self) -> &mut [Track] {
        &mut self.tracks[..]
    }

    pub fn num_tracks(&self) -> usize {
        self.tracks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Text;

    #[test]
    fn parsed_disc_has_tracks() {
        let disc = serde_yaml::from_str::<Disc>(
            "
            - foo
            - title:
                text: bar
                ascii: baz
            - title: quux
              artists:
                - a
                - b
            ",
        )
        .unwrap();
        let tracks = vec![
            Track::new("foo"),
            Track::new(Text::from(("bar", "baz"))),
            Track::new("quux").with_artists(vec![Text::from("a"), Text::from("b")]),
        ];

        assert_eq!(tracks, disc.tracks);
    }
}
