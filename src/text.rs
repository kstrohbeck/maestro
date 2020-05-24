//! Functions for handling text that can have both full, ASCII, and file-safe representations.
use std::borrow::Cow;

pub use old_impl::Text;

mod new_impl;

mod old_impl {
    use super::Cow;
    use serde::{de, ser, Deserialize, Serialize};
    use std::{
        fmt,
        ops::{Add, AddAssign},
    };
    use unicode_normalization::UnicodeNormalization;

    /// A piece of text with an overridable ASCII representation.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Text {
        text: String,
        ascii: Option<String>,
    }

    fn str_to_ascii(s: &str) -> Cow<str> {
        if s.is_ascii() {
            return s.into();
        }

        // Decompose unicode characters to handle accents.
        s.nfkd()
            .filter_map(|c| {
                // Map non-ascii characters to their equivalents.
                if c.is_ascii() {
                    Some(c)
                } else if c == '’' || c == '‘' {
                    Some('\'')
                } else {
                    None
                }
            })
            .collect::<String>()
            .into()
    }

    impl Text {
        /// Create a new `Text`.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use songmaster_rs::text::Text;
        /// let text = Text::new("foo");
        /// assert_eq!("foo", text.text());
        /// assert_eq!("foo", text.ascii());
        /// ```
        pub fn new<T>(text: T) -> Text
        where
            T: Into<String>,
        {
            Text {
                text: text.into(),
                ascii: None,
            }
        }

        /// Create a new `Text` with overridden ASCII.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use songmaster_rs::text::Text;
        /// let text = Text::with_ascii("foo", "bar");
        /// assert_eq!("foo", text.text());
        /// assert_eq!("bar", text.ascii());
        /// ```
        pub fn with_ascii<T, U>(text: T, ascii: U) -> Text
        where
            T: Into<String>,
            U: Into<String>,
        {
            Text {
                text: text.into(),
                ascii: Some(ascii.into()),
            }
        }

        /// Get the text value of a `Text`.
        pub fn text(&self) -> &str {
            &self.text
        }

        /// Get the ascii value of a `Text`.
        ///
        /// If the ascii has been overridden, this returns that value. Otherwise, it returns the text
        /// with any non-ASCII characters replaced with '?'.
        pub fn ascii(&self) -> Cow<str> {
            if let Some(asc) = &self.ascii {
                return asc.into();
            }
            str_to_ascii(self.text())
        }

        /// Returns if this `Text` has an overridden `ascii` value.
        pub fn has_ascii(&self) -> bool {
            self.ascii.is_some()
        }

        /// Get a version of the `Text` safe to use in filenames.
        ///
        /// The file safe name is the ASCII content of the `Text`, minus characters that aren't usable
        /// in one or more operating system filenames. These characters are replaced with file safe
        /// variants.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use songmaster_rs::text::Text;
        /// let text = Text::new("foo: <bar>?");
        /// assert_eq!("foo - [bar]", text.file_safe());
        /// ```
        pub fn file_safe(&self) -> Cow<str> {
            let ascii = self.ascii();
            if !ascii.contains(&['<', '>', ':', '"', '/', '|', '~', '\\', '*', '?'][..]) {
                return ascii;
            }
            let mut buf = String::new();
            for c in ascii.chars() {
                match c {
                    '<' => buf.push('['),
                    '>' => buf.push(']'),
                    ':' => buf.push_str(" -"),
                    '"' => buf.push('\''),
                    '/' | '|' | '~' => buf.push('-'),
                    '\\' | '*' => buf.push('_'),
                    '?' => {}
                    _ => buf.push(c),
                }
            }
            buf.into()
        }

        /// Get a version of the `Text` safe to use in filenames that can be alphabetically sorted.
        ///
        /// This is the same as the results of `file_safe`, except that it allows the text to be
        /// sorted alphabetically - articles at the beginning of the text are moved to the end.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use songmaster_rs::text::Text;
        /// let text = Text::new("The Title of Something");
        /// assert_eq!("Title of Something, The", text.sortable_file_safe());
        /// ```
        pub fn sortable_file_safe(&self) -> Cow<str> {
            use lazy_static::lazy_static;
            use regex::Regex;

            lazy_static! {
                static ref RE: Regex =
                    Regex::new(r"^(?i)(?P<article>the|an|a)\s(?P<rest>.*)$").unwrap();
            }
            let file_safe = self.file_safe();
            match RE.captures(&file_safe) {
                None => file_safe,
                Some(caps) => {
                    let article = caps.name("article").unwrap().as_str();
                    let rest = caps.name("rest").unwrap().as_str();
                    format!("{}, {}", rest, article).into()
                }
            }
        }
    }

    impl Default for Text {
        fn default() -> Self {
            Text::new("")
        }
    }

    impl Serialize for Text {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            use serde::ser::SerializeStruct;

            match self.ascii {
                Some(ref ascii) => {
                    let mut state = serializer.serialize_struct("Text", 2)?;
                    state.serialize_field("text", &self.text)?;
                    state.serialize_field("ascii", ascii)?;
                    state.end()
                }
                None => serializer.serialize_str(&self.text),
            }
        }
    }

    impl<'de> Deserialize<'de> for Text {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            struct Visitor;

            impl<'de> de::Visitor<'de> for Visitor {
                type Value = Text;

                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("a text definition")
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Ok(Text::new(value))
                }

                fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
                where
                    M: de::MapAccess<'de>,
                {
                    #[derive(Deserialize)]
                    #[serde(field_identifier, rename_all = "lowercase")]
                    enum Fields {
                        Text,
                        Ascii,
                    }

                    let mut text = None;
                    let mut ascii = None;

                    while let Some(key) = map.next_key()? {
                        match key {
                            Fields::Text => field!(map, text),
                            Fields::Ascii => field!(map, ascii),
                        }
                    }

                    let text = text.ok_or_else(|| de::Error::missing_field("text"))?;
                    Ok(Text { text, ascii })
                }
            }

            deserializer.deserialize_any(Visitor)
        }
    }

    impl Add for Text {
        type Output = Text;

        fn add(self, other: Self) -> Self::Output {
            let ascii = match (self.ascii, other.ascii) {
                (None, None) => None,
                (None, Some(mut ascii)) => {
                    ascii.insert_str(0, &str_to_ascii(&self.text));
                    Some(ascii)
                }
                (Some(mut ascii), None) => {
                    ascii.push_str(&str_to_ascii(&other.text));
                    Some(ascii)
                }
                (Some(mut ascii), Some(other)) => {
                    ascii.push_str(&other);
                    Some(ascii)
                }
            };

            let mut text = self.text;
            text.push_str(&other.text);

            Text { text, ascii }
        }
    }

    impl Add<&Text> for Text {
        type Output = Text;

        fn add(self, other: &Self) -> Self::Output {
            let ascii = match (self.ascii, other.ascii.as_ref()) {
                (None, None) => None,
                (None, Some(other)) => {
                    let mut ascii = str_to_ascii(&self.text).into_owned();
                    ascii.push_str(other);
                    Some(ascii)
                }
                (Some(mut ascii), None) => {
                    ascii.push_str(&str_to_ascii(&other.text));
                    Some(ascii)
                }
                (Some(mut ascii), Some(other)) => {
                    ascii.push_str(&other);
                    Some(ascii)
                }
            };

            let mut text = self.text;
            text.push_str(&other.text);

            Text { text, ascii }
        }
    }

    impl Add<Text> for &Text {
        type Output = Text;

        fn add(self, other: Text) -> Self::Output {
            let ascii = match (self.ascii.as_ref(), other.ascii) {
                (None, None) => None,
                (None, Some(mut ascii)) => {
                    ascii.insert_str(0, &str_to_ascii(&self.text));
                    Some(ascii)
                }
                (Some(ascii), None) => {
                    let mut ascii = ascii.clone();
                    ascii.push_str(&str_to_ascii(&other.text));
                    Some(ascii)
                }
                (Some(ascii), Some(mut other)) => {
                    other.insert_str(0, ascii);
                    Some(other)
                }
            };

            let mut text = other.text;
            text.insert_str(0, &self.text);

            Text { text, ascii }
        }
    }

    impl<'a, 'b> Add<&'b Text> for &'a Text {
        type Output = Text;

        fn add(self, other: &'b Text) -> Self::Output {
            let ascii = match (self.ascii.as_ref(), other.ascii.as_ref()) {
                (None, None) => None,
                (None, Some(other)) => {
                    let mut ascii = str_to_ascii(&self.text).into_owned();
                    ascii.push_str(other);
                    Some(ascii)
                }
                (Some(ascii), None) => {
                    let mut ascii = ascii.clone();
                    ascii.push_str(&str_to_ascii(&other.text));
                    Some(ascii)
                }
                (Some(ascii), Some(other)) => {
                    let mut ascii = ascii.clone();
                    ascii.push_str(&other);
                    Some(ascii)
                }
            };

            let mut text = self.text.clone();
            text.push_str(&other.text);

            Text { text, ascii }
        }
    }

    impl AddAssign for Text {
        fn add_assign(&mut self, other: Self) {
            <Self as AddAssign<&Text>>::add_assign(self, &other);
        }
    }

    impl AddAssign<&Text> for Text {
        fn add_assign(&mut self, other: &Self) {
            if let Some(ref mut ascii) = &mut self.ascii {
                ascii.push_str(&other.ascii());
            } else if let Some(ref ascii) = &other.ascii {
                self.ascii = Some(self.ascii().into_owned() + ascii);
            }

            self.text.push_str(&other.text);
        }
    }

    impl fmt::Display for Text {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.text)
        }
    }

    impl<'a> From<&'a str> for Text {
        fn from(text: &'a str) -> Text {
            Text::new(text)
        }
    }

    impl From<String> for Text {
        fn from(text: String) -> Text {
            Text::new(text)
        }
    }

    impl std::iter::Sum for Text {
        fn sum<I>(iter: I) -> Self
        where
            I: Iterator<Item = Self>,
        {
            iter.fold(Text::new(""), |a, b| a + b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cow, Text};
    use matches::assert_matches;

    mod serde {
        use super::Text;

        mod ser {
            use super::Text;

            #[test]
            fn simple_text_serializes_to_string() {
                use serde_yaml::Value;
                let text = Text::new("foo");
                let yaml = serde_yaml::to_value(&text).unwrap();
                let expected: Value = "foo".into();
                assert_eq!(expected, yaml);
            }
            #[test]
            fn ascii_text_serializes_to_struct() {
                use serde_yaml::Value;
                let text = Text::with_ascii("foo", "bar");
                let yaml = serde_yaml::to_value(&text).unwrap();
                let mapping = match yaml {
                    Value::Mapping(mapping) => mapping,
                    _ => panic!("yaml wasn't mapping"),
                };

                let pairs = mapping.into_iter().collect::<Vec<_>>();
                let expected: Vec<(Value, Value)> = vec![
                    ("text".into(), "foo".into()),
                    ("ascii".into(), "bar".into()),
                ];
                assert_eq!(expected, pairs);
            }
        }

        mod de {
            use super::Text;

            #[test]
            fn simple_yaml_parses_text() {
                let text = serde_yaml::from_str("\"foo\"").unwrap();
                assert_eq!(Text::new("foo"), text);
            }

            #[test]
            fn yaml_with_only_text_parses_text() {
                let text = serde_yaml::from_str("text: foo").unwrap();
                assert_eq!(Text::new("foo"), text);
            }

            #[test]
            fn yaml_with_text_and_ascii_parses_both() {
                let text = serde_yaml::from_str(
                    "
            text: foo
            ascii: bar
            ",
                )
                .unwrap();
                assert_eq!(Text::with_ascii("foo", "bar"), text);
            }

            #[test]
            fn yaml_non_string_or_hash_doesnt_parse() {
                let text = serde_yaml::from_str::<Text>("[]");
                assert!(text.is_err());
            }

            #[test]
            fn yaml_hash_without_text_doesnt_parse() {
                let text = serde_yaml::from_str::<Text>("ascii: bar");
                assert!(text.is_err());
            }
        }

        #[test]
        fn simple_text_is_serde_equal() {
            let text = Text::new("foo");
            let new_text: Text = serde_yaml::to_string(&text)
                .and_then(|s| serde_yaml::from_str(&s))
                .unwrap();
            assert_eq!(text, new_text);
        }
        #[test]
        fn ascii_text_is_serde_equal() {
            let text = Text::with_ascii("foo", "bar");
            let new_text: Text = serde_yaml::to_string(&text)
                .and_then(|s| serde_yaml::from_str(&s))
                .unwrap();
            assert_eq!(text, new_text);
        }
    }

    mod ascii {
        use super::{assert_matches, Cow, Text};

        #[test]
        fn is_same_as_text() {
            let text = Text::new("hello");
            assert_eq!(text.ascii(), "hello");
        }

        #[test]
        fn is_overridden_value() {
            let text = Text::with_ascii("hello", "goodbye");
            assert_eq!(text.ascii(), "goodbye");
        }

        #[test]
        fn translates_punctuation() {
            let text = Text::new("Let’s");
            assert_eq!(text.ascii(), "Let's");
        }

        #[test]
        fn translates_accented_characters() {
            let text = Text::new("héllo");
            assert_eq!(text.ascii(), "hello");
        }

        #[test]
        fn removes_nonascii_characters() {
            let text = Text::new("fire = 🔥");
            assert_eq!(text.ascii(), "fire = ");
        }

        #[test]
        fn is_borrowed_text() {
            let text = Text::new("hello");
            assert_matches!(text.ascii(), Cow::Borrowed(_));
        }

        #[test]
        fn is_borrowed_if_overridden() {
            let text = Text::with_ascii("hello", "goodbye");
            assert_matches!(text.ascii(), Cow::Borrowed(_));
        }

        #[test]
        fn is_owned_for_nonascii_text() {
            let text = Text::new("fire = 🔥");
            assert_matches!(text.ascii(), Cow::Owned(_));
        }
    }

    mod file_safe {
        use super::{assert_matches, Cow, Text};

        #[test]
        fn is_ascii_if_no_unsafe_chars() {
            let text = Text::with_ascii("foo", "bar");
            assert_eq!(text.file_safe(), "bar");
        }

        #[test]
        fn replaces_unsafe_chars() {
            let text = Text::new("foo: <bar>?");
            assert_eq!(text.file_safe(), "foo - [bar]");
        }

        #[test]
        fn is_borrowed_if_text_is_safe() {
            let text = Text::new("foo");
            assert_matches!(text.file_safe(), Cow::Borrowed(_));
        }

        #[test]
        fn is_owned_if_text_isnt_ascii() {
            let text = Text::new("fire = 🔥");
            assert_matches!(text.file_safe(), Cow::Owned(_));
        }

        #[test]
        fn is_owned_if_text_isnt_safe() {
            let text = Text::new("foo?");
            assert_matches!(text.file_safe(), Cow::Owned(_));
        }

        #[test]
        fn borrowed_if_overridden_ascii_is_safe() {
            let text = Text::with_ascii("foo", "bar");
            assert_matches!(text.file_safe(), Cow::Borrowed(_));
        }

        #[test]
        fn is_owned_if_overridden_ascii_isnt_safe() {
            let text = Text::with_ascii("foo", "bar?");
            assert_matches!(text.file_safe(), Cow::Owned(_));
        }
    }

    mod sortable_file_safe {
        use super::{assert_matches, Cow, Text};

        #[test]
        fn is_same_as_file_safe_without_article() {
            let text = Text::with_ascii("foo", "\"bar\"");
            assert_eq!(text.sortable_file_safe(), "'bar'");
        }

        #[test]
        fn moves_article_to_end() {
            let text = Text::with_ascii("foo", "the \"bar\"");
            assert_eq!(text.sortable_file_safe(), "'bar', the");
        }

        #[test]
        fn preserves_casing() {
            let text = Text::new("A Song Title");
            assert_eq!(text.sortable_file_safe(), "Song Title, A");
        }

        #[test]
        fn is_borrowed_with_unmodified_text() {
            let text = Text::new("foo");
            assert_matches!(text.sortable_file_safe(), Cow::Borrowed(_));
        }

        #[test]
        fn is_owned_with_nonascii_text() {
            let text = Text::new("fire = 🔥");
            assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
        }

        #[test]
        fn is_owned_with_non_file_safe_text() {
            let text = Text::new("foo?");
            assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
        }

        #[test]
        fn is_owned_with_modified_text() {
            let text = Text::new("A Song Title");
            assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
        }

        #[test]
        fn is_borrowed_with_unmodified_ascii() {
            let text = Text::with_ascii("foo", "bar");
            assert_matches!(text.sortable_file_safe(), Cow::Borrowed(_));
        }

        #[test]
        fn is_owned_with_non_safe_ascii() {
            let text = Text::with_ascii("foo", "bar?");
            assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
        }

        #[test]
        fn is_owned_with_modified_ascii() {
            let text = Text::with_ascii("foo", "the bar");
            assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
        }
    }

    mod add {
        use super::Text;

        #[test]
        fn texts_add_together() {
            let (a, b) = (Text::new("hello"), Text::new("world"));
            assert_eq!(a + b, Text::new("helloworld"));
            let (a, b) = (Text::new("hello"), Text::new("world"));
            assert_eq!(a + &b, Text::new("helloworld"));
            let (a, b) = (Text::new("hello"), Text::new("world"));
            assert_eq!(&a + b, Text::new("helloworld"));
            let (a, b) = (Text::new("hello"), Text::new("world"));
            assert_eq!(&a + &b, Text::new("helloworld"));
        }

        macro_rules! add_tests {
            ($a:ident, $b:ident, $exp:expr) => {
                let (x, y) = ($a.clone(), $b.clone());
                assert_eq!($exp, x + y);
                let (x, y) = ($a.clone(), $b.clone());
                assert_eq!($exp, x + &y);
                let (x, y) = ($a.clone(), $b.clone());
                assert_eq!($exp, &x + y);
                let (x, y) = ($a.clone(), $b.clone());
                assert_eq!($exp, &x + &y);
            };
        }

        #[test]
        fn first_asciis_add_together() {
            let (a, b) = (Text::with_ascii("hello", "goodbye"), Text::new("world"));
            add_tests!(a, b, Text::with_ascii("helloworld", "goodbyeworld"));
        }

        #[test]
        fn add_second_asciis_add_together() {
            let (a, b) = (Text::new("hello"), Text::with_ascii("world", "universe"));
            add_tests!(a, b, Text::with_ascii("helloworld", "hellouniverse"));
        }

        #[test]
        fn text_adds_to_itself() {
            let mut a = Text::new("hello");
            a += &Text::new("world");
            assert_eq!(a, Text::new("helloworld"));
        }

        #[test]
        fn ascii_adds_to_itself() {
            let mut a = Text::new("hello");
            a += &Text::with_ascii("world", "universe");
            assert_eq!(a, Text::with_ascii("helloworld", "hellouniverse"));
        }

        #[test]
        fn text_adds_to_existing_ascii() {
            let mut a = Text::with_ascii("hello", "goodbye");
            a += &Text::with_ascii("world", "universe");
            assert_eq!(a, Text::with_ascii("helloworld", "goodbyeuniverse"));
        }
    }

    mod sum {
        use super::Text;

        #[test]
        fn texts_sum_together() {
            let texts = vec![Text::new("hello"), Text::new("world")];
            assert_eq!(texts.into_iter().sum::<Text>(), Text::new("helloworld"));
        }

        #[test]
        fn asciis_sum_together() {
            let texts = vec![Text::new("hello"), Text::with_ascii("world", "universe")];
            assert_eq!(
                texts.into_iter().sum::<Text>(),
                Text::with_ascii("helloworld", "hellouniverse")
            );
        }
    }
}
