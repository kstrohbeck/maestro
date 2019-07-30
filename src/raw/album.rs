use super::disc::Disc;
use crate::Text;
use serde::{de, ser, Deserialize, Serialize};
use std::{borrow::Cow, fmt};

#[derive(Debug)]
pub struct Album {
    pub title: Text,
    pub artists: Vec<Text>,
    pub year: Option<usize>,
    pub genre: Option<Text>,
    pub discs: Vec<Disc>,
}

impl Album {
    /// Create a new album with only essential information.
    pub fn new<T>(title: T) -> Album
    where
        T: Into<Text>,
    {
        Album {
            title: title.into(),
            artists: Vec::new(),
            year: None,
            genre: None,
            discs: Vec::new(),
        }
    }

    pub fn artist(&self) -> Cow<Text> {
        crate::utils::comma_separated(&self.artists)
    }

    pub fn genre(&self) -> Option<&Text> {
        self.genre.as_ref()
    }

    pub fn num_discs(&self) -> usize {
        self.discs.len()
    }

    pub fn with_title<T: Into<Text>>(mut self, title: T) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_artists<T: Into<Vec<Text>>>(mut self, artists: T) -> Self {
        self.artists = artists.into();
        self
    }

    pub fn with_year<T: Into<Option<usize>>>(mut self, year: T) -> Self {
        self.year = year.into();
        self
    }

    pub fn with_genre<T: Into<Option<Text>>>(mut self, genre: T) -> Self {
        self.genre = genre.into();
        self
    }

    pub fn with_discs<T: Into<Vec<Disc>>>(mut self, discs: T) -> Self {
        self.discs = discs.into();
        self
    }
}

impl Serialize for Album {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct;

        let num_fields = [self.year.is_some(), self.genre.is_some()]
            .iter()
            .copied()
            .filter(|x| *x)
            .count()
            + 3;

        let mut state = serializer.serialize_struct("Album", num_fields)?;

        state.serialize_field("title", &self.title)?;

        if self.artists.len() == 1 {
            state.serialize_field("artist", &self.artists[0])?;
        } else {
            state.serialize_field("artists", &self.artists)?;
        }

        ser_field!(state, "year", self.year);
        ser_field!(state, "genre", self.genre());

        if self.discs.len() == 1 {
            state.serialize_field("tracks", &self.discs[0])?;
        } else {
            state.serialize_field("discs", &self.discs)?;
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for Album {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Fields {
            Title,
            Artists,
            Artist,
            Year,
            Genre,
            Discs,
            Tracks,
            #[serde(other)]
            Other,
        }

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Album;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a track definition")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut title = None;
                let mut artists = None;
                let mut year = None;
                let mut genre = None;
                let mut discs = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Fields::Title => field!(map, title),
                        Fields::Artists => field!(map, artists),
                        Fields::Artist => field!(artists { vec![map.next_value()?] }),
                        Fields::Year => field!(map, year),
                        Fields::Genre => field!(map, genre),
                        Fields::Discs => field!(map, discs),
                        Fields::Tracks => field!(discs { vec![map.next_value()?] }),
                        Fields::Other => {}
                    }
                }

                let title = title.ok_or_else(|| de::Error::missing_field("title"))?;
                let artists = artists.ok_or_else(|| de::Error::missing_field("artists"))?;
                let discs = discs.ok_or_else(|| de::Error::missing_field("discs"))?;

                Ok(Album {
                    title,
                    artists,
                    year,
                    genre,
                    discs,
                })
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artist_is_only_artist_in_list() {
        let album = Album::new("foo").with_artists(vec![Text::with_ascii("b", "c")]);
        assert_eq!(Cow::Borrowed(&Text::with_ascii("b", "c")), album.artist());
    }

    #[test]
    fn artist_is_comma_separated_if_multiple() {
        let album =
            Album::new("foo").with_artists(vec![Text::new("a"), Text::with_ascii("b", "c")]);

        assert_eq!(
            Cow::<Text>::Owned(Text::with_ascii("a, b", "a, c")),
            album.artist()
        );
    }

    #[test]
    fn title_is_parsed() {
        let album = serde_yaml::from_str::<Album>(
            "
            title: foo
            artist: bar
            tracks:
                - a
                - b
            ",
        )
        .unwrap();
        assert_eq!(Text::new("foo"), album.title);
    }
}
