//! Functions for handling text that can have both full, ASCII, and file-safe representations.

use serde::{de, Deserialize};
use std::{
    borrow::Cow,
    fmt,
    ops::{Add, AddAssign},
};

/// A piece of text with an overridable ASCII representation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Text {
    text: String,
    ascii: Option<String>,
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

        let text = self.text();
        if text.is_ascii() {
            return text.into();
        }

        text.chars()
            .map(|c| if c.is_ascii() { c } else { '?' })
            .collect::<String>()
            .into()
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
        <Self as Add<&Text>>::add(self, &other)
    }
}

impl Add<&Text> for Text {
    type Output = Text;

    fn add(self, other: &Self) -> Self::Output {
        #[allow(clippy::suspicious_arithmetic_impl)]
        let ascii = if self.ascii.is_none() && other.ascii.is_none() {
            None
        } else {
            Some(self.ascii().into_owned() + &other.ascii())
        };

        let text = self.text + &other.text;

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

#[cfg(test)]
mod tests {
    use super::*;
    use matches::assert_matches;

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

    #[test]
    fn ascii_is_same_as_text() {
        let text = Text::new("hello");
        assert_eq!(text.ascii(), "hello");
    }

    #[test]
    fn ascii_is_overridden_value() {
        let text = Text::with_ascii("hello", "goodbye");
        assert_eq!(text.ascii(), "goodbye");
    }

    #[test]
    fn ascii_replaces_nonascii_characters() {
        let text = Text::new("fire = 🔥");
        assert_eq!(text.ascii(), "fire = ?");
    }

    #[test]
    fn ascii_is_borrowed_text() {
        let text = Text::new("hello");
        assert_matches!(text.ascii(), Cow::Borrowed(_));
    }

    #[test]
    fn ascii_is_borrowed_if_overridden() {
        let text = Text::with_ascii("hello", "goodbye");
        assert_matches!(text.ascii(), Cow::Borrowed(_));
    }

    #[test]
    fn ascii_is_owned_for_nonascii_text() {
        let text = Text::new("fire = 🔥");
        assert_matches!(text.ascii(), Cow::Owned(_));
    }

    #[test]
    fn file_safe_is_ascii_if_no_unsafe_chars() {
        let text = Text::with_ascii("foo", "bar");
        assert_eq!(text.file_safe(), "bar");
    }

    #[test]
    fn file_safe_replaces_unsafe_chars() {
        let text = Text::new("foo: <bar>?");
        assert_eq!(text.file_safe(), "foo - [bar]");
    }

    #[test]
    fn file_safe_is_borrowed_if_text_is_safe() {
        let text = Text::new("foo");
        assert_matches!(text.file_safe(), Cow::Borrowed(_));
    }

    #[test]
    fn file_safe_is_owned_if_text_isnt_ascii() {
        let text = Text::new("fire = 🔥");
        assert_matches!(text.file_safe(), Cow::Owned(_));
    }

    #[test]
    fn file_safe_is_owned_if_text_isnt_safe() {
        let text = Text::new("foo?");
        assert_matches!(text.file_safe(), Cow::Owned(_));
    }

    #[test]
    fn file_safe_is_borrowed_if_overridden_ascii_is_safe() {
        let text = Text::with_ascii("foo", "bar");
        assert_matches!(text.file_safe(), Cow::Borrowed(_));
    }

    #[test]
    fn file_safe_is_owned_if_overridden_ascii_isnt_safe() {
        let text = Text::with_ascii("foo", "bar?");
        assert_matches!(text.file_safe(), Cow::Owned(_));
    }

    #[test]
    fn sortable_file_safe_is_same_as_file_safe_without_article() {
        let text = Text::with_ascii("foo", "\"bar\"");
        assert_eq!(text.sortable_file_safe(), "'bar'");
    }

    #[test]
    fn sortable_file_safe_moves_article_to_end() {
        let text = Text::with_ascii("foo", "the \"bar\"");
        assert_eq!(text.sortable_file_safe(), "'bar', the");
    }

    #[test]
    fn sortable_file_safe_preserves_casing() {
        let text = Text::new("A Song Title");
        assert_eq!(text.sortable_file_safe(), "Song Title, A");
    }

    #[test]
    fn sortable_file_safe_is_borrowed_with_unmodified_text() {
        let text = Text::new("foo");
        assert_matches!(text.sortable_file_safe(), Cow::Borrowed(_));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_nonascii_text() {
        let text = Text::new("fire = 🔥");
        assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_non_file_safe_text() {
        let text = Text::new("foo?");
        assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_modified_text() {
        let text = Text::new("A Song Title");
        assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
    }

    #[test]
    fn sortable_file_safe_is_borrowed_with_unmodified_ascii() {
        let text = Text::with_ascii("foo", "bar");
        assert_matches!(text.sortable_file_safe(), Cow::Borrowed(_));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_non_safe_ascii() {
        let text = Text::with_ascii("foo", "bar?");
        assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_modified_ascii() {
        let text = Text::with_ascii("foo", "the bar");
        assert_matches!(text.sortable_file_safe(), Cow::Owned(_));
    }

    #[test]
    fn texts_add_text_together() {
        let (a, b) = (Text::new("hello"), Text::new("world"));
        assert_eq!(a + b, Text::new("helloworld"));
    }

    #[test]
    fn texts_add_asciis_together() {
        let (a, b) = (Text::new("hello"), Text::with_ascii("world", "universe"));
        assert_eq!(a + b, Text::with_ascii("helloworld", "hellouniverse"));
    }

    #[test]
    fn text_adds_text_to_itself() {
        let mut a = Text::new("hello");
        a += &Text::new("world");
        assert_eq!(a, Text::new("helloworld"));
    }

    #[test]
    fn text_adds_ascii_to_itself() {
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

    #[test]
    fn texts_sum_text_together() {
        let texts = vec![Text::new("hello"), Text::new("world")];
        assert_eq!(texts.into_iter().sum::<Text>(), Text::new("helloworld"));
    }

    #[test]
    fn texts_sum_asciis_together() {
        let texts = vec![Text::new("hello"), Text::with_ascii("world", "universe")];
        assert_eq!(
            texts.into_iter().sum::<Text>(),
            Text::with_ascii("helloworld", "hellouniverse")
        );
    }
}
