use crate::Text;
use serde::{de, ser, Deserialize, Serialize};

// TODO: Add "featuring" property.

/// A music track in an album.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Track {
    /// The title of the track.
    pub title: Text,

    /// A list of artists that created the track, or None if the album's artists should be used.
    artists: Option<Vec<Text>>,

    /// The year the track was created, or None if the album's year should be used.
    pub year: Option<usize>,

    /// The genre of the track, or None if the album's genre should be used.
    genre: Option<Text>,

    /// Any comments on the track.
    comment: Option<Text>,

    /// The track's lyrics.
    lyrics: Option<Text>,

    /// Artists a track features.
    featuring: Option<Vec<Text>>,

    /// The track's filename, if it isn't derived from the title.
    filename: Option<String>,
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
            featuring: None,
            filename: None,
        }
    }

    pub fn artists(&self) -> Option<&[Text]> {
        self.artists.as_ref().map(Vec::as_slice)
    }

    pub fn genre(&self) -> Option<&Text> {
        self.genre.as_ref()
    }

    pub fn comment(&self) -> Option<&Text> {
        self.comment.as_ref()
    }

    pub fn lyrics(&self) -> Option<&Text> {
        self.lyrics.as_ref()
    }

    pub fn featuring(&self) -> Option<&[Text]> {
        self.featuring.as_ref().map(Vec::as_slice)
    }

    pub fn filename(&self) -> Option<&str> {
        self.filename.as_ref().map(String::as_str)
    }

    pub fn with_artists<T: Into<Option<Vec<Text>>>>(mut self, artists: T) -> Self {
        self.artists = artists.into();
        self
    }

    pub fn with_year<T: Into<Option<usize>>>(mut self, year: T) -> Self {
        self.year = year.into();
        self
    }

    pub fn with_genre<T: Into<Text>>(mut self, genre: T) -> Self {
        self.genre = Some(genre.into());
        self
    }

    pub fn with_comment<T: Into<Text>>(mut self, comment: T) -> Self {
        self.comment = Some(comment.into());
        self
    }

    pub fn with_lyrics<T: Into<Text>>(mut self, lyrics: T) -> Self {
        self.lyrics = Some(lyrics.into());
        self
    }

    pub fn with_featuring<T: Into<Option<Vec<Text>>>>(mut self, featuring: T) -> Self {
        self.featuring = featuring.into();
        self
    }
}

impl Serialize for Track {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeStruct;

        let num_fields = [
            self.artists.is_some(),
            self.year.is_some(),
            self.genre.is_some(),
            self.comment.is_some(),
            self.lyrics.is_some(),
            self.featuring.is_some(),
            self.filename.is_some(),
        ]
        .iter()
        .copied()
        .filter(|x| *x)
        .count()
            + 1;

        if num_fields == 1 && !self.title.has_ascii() {
            return serializer.serialize_str(self.title.text());
        }

        let mut state = serializer.serialize_struct("Track", num_fields)?;

        state.serialize_field("title", &self.title)?;

        if let Some(artists) = self.artists() {
            if artists.len() == 1 {
                state.serialize_field("artist", &artists[0])?;
            } else {
                state.serialize_field("artists", &artists)?;
            }
        }

        ser_field!(state, "year", self.year);
        ser_field!(state, "genre", self.genre());
        ser_field!(state, "comment", self.comment());
        ser_field!(state, "lyrics", self.lyrics());
        if let Some(feat) = self.featuring() {
            if feat.len() == 1 {
                state.serialize_field("featuring", &feat[0])?;
            } else {
                state.serialize_field("featuring", feat)?;
            }
        }
        ser_field!(state, "filename", self.filename());

        state.end()
    }
}

impl<'de> Deserialize<'de> for Track {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Fields {
            Title,
            Artists,
            Artist,
            Year,
            Genre,
            Comment,
            Lyrics,
            Featuring,
            Filename,
            #[serde(other)]
            Other,
        }

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Track;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a track definition")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Track::new(value))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut title = None;
                let mut artists = None;
                let mut year = None;
                let mut genre = None;
                let mut comment = None;
                let mut lyrics = None;
                let mut featuring = None;
                let mut filename = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Fields::Title => field!(map, title),
                        Fields::Artists => field!(map, artists),
                        Fields::Artist => field!(artists { vec![map.next_value()?] }),
                        Fields::Year => field!(map, year),
                        Fields::Genre => field!(map, genre),
                        Fields::Comment => field!(map, comment),
                        Fields::Lyrics => field!(map, lyrics),
                        Fields::Featuring => field!(featuring {
                            // TODO: Make this generic so we can reuse it for other things (like listing artists.)
                            #[derive(Deserialize)]
                            #[serde(untagged)]
                            enum TextOrList {
                                Text(Text),
                                List(Vec<Text>),
                            }

                            let value: TextOrList = map.next_value()?;
                            match value {
                                TextOrList::Text(t) => vec![t],
                                TextOrList::List(l) => l,
                            }
                        }),
                        Fields::Filename => field!(map, filename),
                        Fields::Other => {}
                    }
                }

                let title = title.ok_or_else(|| de::Error::missing_field("title"))?;

                Ok(Track {
                    title,
                    artists,
                    year,
                    genre,
                    comment,
                    lyrics,
                    featuring,
                    filename,
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_is_parsed_to_track_with_title() {
        let track = serde_yaml::from_str::<Track>("\"foo\"").unwrap();
        assert_eq!(Text::new("foo"), track.title);
    }

    #[test]
    fn simple_title_is_parsed() {
        let track: Track = serde_yaml::from_str("title: foo").unwrap();
        assert_eq!(Text::new("foo"), track.title);
    }

    #[test]
    fn complex_title_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title:
                text: foo
                ascii: bar
            ",
        )
        .unwrap();
        assert_eq!(Text::with_ascii("foo", "bar"), track.title);
    }

    #[test]
    fn single_simple_artist_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            artist: bar
            ",
        )
        .unwrap();
        assert_eq!(Some(&[Text::new("bar")][..]), track.artists());
    }

    #[test]
    fn single_complex_artist_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            artist:
                text: bar
                ascii: baz
            ",
        )
        .unwrap();
        assert_eq!(Some(&[Text::with_ascii("bar", "baz")][..]), track.artists());
    }

    #[test]
    fn array_in_artist_is_not_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            artist:
                - foo
                - bar
            ",
        );
        assert!(track.is_err());
    }

    #[test]
    fn multi_simple_artists_are_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            artists:
                - bar
                - baz
            ",
        )
        .unwrap();
        assert_eq!(
            Some(&[Text::new("bar"), Text::new("baz")][..]),
            track.artists(),
        );
    }

    #[test]
    fn multi_mixed_artists_are_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            artists:
                - bar
                - text: baz
                  ascii: quux
            ",
        )
        .unwrap();
        assert_eq!(
            Some(&[Text::new("bar"), Text::with_ascii("baz", "quux")][..]),
            track.artists(),
        );
    }

    #[test]
    fn single_artist_in_multi_is_not_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            artists: bar
            ",
        );
        assert!(track.is_err());
    }

    #[test]
    fn year_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            year: 1990
            ",
        )
        .unwrap();
        assert_eq!(Some(1990), track.year);
    }

    #[test]
    fn simple_genre_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            genre: Music
            ",
        )
        .unwrap();
        assert_eq!(Some(&Text::new("Music")), track.genre());
    }

    #[test]
    fn complex_genre_is_parsed_from_yaml() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            genre:
                text: Music
                ascii: Not Music
            ",
        )
        .unwrap();
        assert_eq!(Some(&Text::with_ascii("Music", "Not Music")), track.genre());
    }

    #[test]
    fn simple_comment_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            comment: stuff
            ",
        )
        .unwrap();
        assert_eq!(Some(&Text::new("stuff")), track.comment());
    }

    #[test]
    fn complex_comment_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            comment:
                text: stuff
                ascii: other
            ",
        )
        .unwrap();
        assert_eq!(Some(&Text::with_ascii("stuff", "other")), track.comment());
    }

    #[test]
    fn simple_lyrics_are_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            lyrics: stuff
            ",
        )
        .unwrap();
        assert_eq!(Some(&Text::new("stuff")), track.lyrics());
    }

    #[test]
    fn complex_lyrics_are_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            lyrics:
                text: stuff
                ascii: other
            ",
        )
        .unwrap();
        assert_eq!(Some(&Text::with_ascii("stuff", "other")), track.lyrics());
    }

    #[test]
    fn single_simple_featuring_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            featuring: bar
            ",
        )
        .unwrap();
        assert_eq!(Some(&[Text::new("bar")][..]), track.featuring());
    }

    #[test]
    fn single_complex_featuring_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            featuring:
                text: bar
                ascii: baz
            ",
        )
        .unwrap();
        assert_eq!(
            Some(&[Text::with_ascii("bar", "baz")][..]),
            track.featuring()
        );
    }

    #[test]
    fn multiple_simple_featuring_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            featuring:
              - bar
              - baz
            ",
        )
        .unwrap();
        assert_eq!(
            Some(&[Text::new("bar"), Text::new("baz")][..]),
            track.featuring()
        );
    }

    #[test]
    fn multiple_complex_featuring_is_parsed() {
        let track = serde_yaml::from_str::<Track>(
            "
            title: foo
            featuring:
              - text: bar
                ascii: baz
              - quux
            ",
        )
        .unwrap();
        assert_eq!(
            Some(&[Text::with_ascii("bar", "baz"), Text::new("quux")][..]),
            track.featuring()
        );
    }
}
