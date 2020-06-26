use super::disc::Disc;
use crate::Text;
use serde::{de, ser, Deserialize, Serialize};
use std::{borrow::Cow, fmt, path::Path};

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

    /// Create an album from a folder of MP3s.
    pub fn generate<P: AsRef<Path>>(path: P) -> Album {
        use super::track::Track;
        use std::collections::HashMap;
        use std::fs::File;
        use std::io::{BufReader, Seek};
        use std::path::PathBuf;
        use walkdir::WalkDir;

        struct TrackInfo {
            path: PathBuf,
            tag: Option<id3::Tag>,
            disc_name: Option<String>,
        }

        let path = path.as_ref();
        fn ok_or_print<T, E>(res: Result<T, E>) -> Option<T>
        where
            E: std::fmt::Debug,
        {
            match res {
                Ok(t) => Some(t),
                Err(e) => {
                    println!("{:?}", e);
                    None
                }
            }
        }

        let mut track_infos = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| ok_or_print(e))
            .filter(|d| d.file_type().is_file())
            .filter_map(|d| {
                let path = d.into_path();
                let ext = path.extension()?;
                if ext != "mp3" {
                    return None;
                }

                let tag = id3::Tag::read_from_path(&path).ok();
                /*
                let file = ok_or_print(File::open(&path));
                let tag = file.and_then(|f| {
                    /*
                    let mut rdr = BufReader::new(f);
                    if let Some(tag) = ok_or_print(id3::Tag::read_from(rdr)) {
                        Some(tag)
                    } else {
                        None
                    }
                    */
                    /*
                    ok_or_print(id3::Tag::read_from(&mut rdr))
                        .or_else(|| ok_or_print(id3::v1::Tag::read_from(&mut rdr).map(Into::into)))
                        */
                });
                */

                Some(TrackInfo {
                    path,
                    tag,
                    disc_name: None,
                })
            })
            .collect::<Vec<_>>();

        track_infos.sort_by(|a, b| a.path.cmp(&b.path));

        fn get_most_often<'a, T, F>(track_infos: &'a [TrackInfo], get: F) -> Option<T>
        where
            T: Eq + std::hash::Hash,
            F: Fn(&'a id3::Tag) -> Option<T>,
        {
            let mut occurrences = HashMap::new();
            for t in track_infos
                .iter()
                .filter_map(|t| t.tag.as_ref())
                .filter_map(get)
            {
                *occurrences.entry(t).or_insert(0) += 1;
            }

            let mut value = None;
            let mut occ = 0;
            for (v, o) in occurrences.drain() {
                if o > occ {
                    value = Some(v);
                    occ = o;
                }
            }

            value
        }

        let title = get_most_often(&track_infos, id3::Tag::album).map(|s| s.to_string());
        let artists = vec![get_most_often(&track_infos, id3::Tag::album_artist)
            .or_else(|| get_most_often(&track_infos, id3::Tag::artist))
            .map(|s| s.to_string())
            .unwrap_or_else(|| String::from(""))
            .into()];
        let year = get_most_often(&track_infos, |t| t.date_recorded().map(|d| d.year as usize));
        let genre: Option<Text> =
            get_most_often(&track_infos, id3::Tag::genre).map(|s| Text::from(s.to_string()));

        let mut discs = HashMap::new();
        for info in track_infos.into_iter() {
            let filename = ok_or_print(info.path.strip_prefix(path))
                .and_then(|o| o.to_str())
                .map(|s| s.to_string());
            let title = info
                .tag
                .as_ref()
                .and_then(|t| t.title())
                .or_else(|| info.path.file_stem().and_then(|o| o.to_str()))
                .unwrap_or("");
            let track_artists = info
                .tag
                .as_ref()
                .and_then(|t| t.artist())
                .map(|a| vec![Text::from(a.to_string())]);
            let track_year = info
                .tag
                .as_ref()
                .and_then(|t| t.date_recorded())
                .map(|d| d.year as usize);
            let track_genre = info
                .tag
                .as_ref()
                .and_then(|t| t.genre())
                .map(|s| Text::from(s.to_string()));
            let track = Track::new(title.to_string())
                .with_filename(filename)
                .with_artists(track_artists)
                .with_year(track_year)
                .with_genre(track_genre);

            let disc = info
                .tag
                .as_ref()
                .and_then(|t| t.disc())
                .map(|d| d.to_string())
                .or(info.disc_name)
                .unwrap_or(String::from("Disc 1"));
            discs.entry(disc).or_insert_with(Vec::new).push(track);
        }

        let mut discs = discs.into_iter().collect::<Vec<_>>();
        discs.sort_by(|a, b| a.0.cmp(&b.0));
        let discs = discs
            .into_iter()
            .map(|(_, v)| Disc::from_tracks(v))
            .collect::<Vec<_>>();

        Album::new(title.unwrap_or(String::from("")))
            .with_artists(artists)
            .with_year(year)
            .with_genre(genre)
            .with_discs(discs)
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
        let album = Album::new("foo").with_artists(vec![Text::from(("b", "c"))]);
        assert_eq!(Cow::Borrowed(&Text::from(("b", "c"))), album.artist());
    }

    #[test]
    fn artist_is_comma_separated_if_multiple() {
        let album = Album::new("foo").with_artists(vec![Text::from("a"), Text::from(("b", "c"))]);

        assert_eq!(
            Cow::<Text>::Owned(Text::from(("a, b", "a, c"))),
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
        assert_eq!(Text::from("foo"), album.title);
    }
}
