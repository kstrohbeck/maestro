//! Functions for handling text that can have both full, ASCII, and file-safe representations.

use crate::utils::{parse_key_from_hash, try_parse_key_from_hash, yaml_into_string, ParseKeyError};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    borrow::Cow,
    fmt,
    ops::{Add, AddAssign},
};
use yaml_rust::Yaml;

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

    /// Parse `Text` from a Yaml source.
    ///
    /// A `Text` may be loaded from one of two kinds of YAML objects. If the YAML is a string, that
    /// string is used as the text, and ASCII is not overridden. If the YAML is a hash, then a
    /// "text" key is expected with a string value, and an "ascii" key is allowed with a string
    /// value that overrides the `Text`'s ASCII value.
    ///
    /// # Examples
    ///
    /// Loading a simple string:
    ///
    /// ```rust
    /// # use songmaster_rs::text::Text;
    /// use yaml_rust::YamlLoader;
    ///
    /// let yaml = YamlLoader::load_from_str("\"foo\"")?.remove(0);
    /// assert_eq!(Ok(Text::new("foo")), Text::from_yaml(yaml));
    /// # Ok::<(), yaml_rust::ScanError>(())
    /// ```
    ///
    /// Loading a string with overridden ASCII:
    ///
    /// ```rust
    /// # use songmaster_rs::text::Text;
    /// use yaml_rust::YamlLoader;
    ///
    /// let yaml = YamlLoader::load_from_str("
    /// text: foo
    /// ascii: bar
    /// ")?
    /// .remove(0);
    /// assert_eq!(Ok(Text::with_ascii("foo", "bar")), Text::from_yaml(yaml));
    /// # Ok::<(), yaml_rust::ScanError>(())
    /// ```
    pub fn from_yaml(yaml: Yaml) -> Result<Text, FromYamlError> {
        match yaml {
            Yaml::String(text) => Ok(Text::new(text)),
            Yaml::Hash(mut hash) => Ok({
                let text = parse_key_from_hash(&mut hash, "text", yaml_into_string).map_err(
                    |e| match e {
                        ParseKeyError::KeyNotFound => FromYamlError::MissingTextKey,
                        ParseKeyError::InvalidValue(v) => FromYamlError::InvalidText(v),
                    },
                )?;

                let ascii = try_parse_key_from_hash(&mut hash, "ascii", yaml_into_string)
                    .map_err(FromYamlError::InvalidAscii)?;

                Text { text, ascii }
            }),
            yaml => Err(FromYamlError::NotStringOrHash(yaml)),
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

/// An error when parsing a `Text` from YAML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FromYamlError {
    /// The hash is missing the "text" key.
    MissingTextKey,

    /// The hash's "text" value is not a string.
    InvalidText(Yaml),

    /// The hash's "ascii" value is not a string.
    InvalidAscii(Yaml),

    /// The text object is not a string or a hash.
    NotStringOrHash(Yaml),
}

impl fmt::Display for FromYamlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FromYamlError::MissingTextKey => write!(f, "missing \"text\" key"),
            FromYamlError::InvalidText(_) => write!(f, "invalid \"text\" value"),
            FromYamlError::InvalidAscii(_) => write!(f, "invalid \"ascii\" value"),
            FromYamlError::NotStringOrHash(_) => write!(f, "text must be string or hash"),
        }
    }
}

impl std::error::Error for FromYamlError {}

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
    use yaml_rust::YamlLoader;

    #[test]
    fn simple_yaml_parses_text() {
        let yaml = YamlLoader::load_from_str("\"foo\"").unwrap().pop().unwrap();
        let text = Text::from_yaml(yaml).unwrap();
        assert_eq!(text, Text::new("foo"));
    }

    #[test]
    fn yaml_with_only_text_parses_text() {
        let yaml = YamlLoader::load_from_str("text: foo")
            .unwrap()
            .pop()
            .unwrap();
        let text = Text::from_yaml(yaml).unwrap();
        assert_eq!(text, Text::new("foo"));
    }

    #[test]
    fn yaml_with_text_and_ascii_parses_both() {
        let yaml = YamlLoader::load_from_str("text: foo\nascii: bar")
            .unwrap()
            .pop()
            .unwrap();
        let text = Text::from_yaml(yaml).unwrap();
        assert_eq!(text, Text::with_ascii("foo", "bar"));
    }

    #[test]
    fn yaml_non_string_or_hash_doesnt_parse() {
        let yaml = YamlLoader::load_from_str("123").unwrap().pop().unwrap();
        let text = Text::from_yaml(yaml);
        assert_matches!(text, Err(FromYamlError::NotStringOrHash(_)));
    }

    #[test]
    fn yaml_hash_without_text_doesnt_parse() {
        let yaml = YamlLoader::load_from_str("ascii: bar")
            .unwrap()
            .pop()
            .unwrap();
        let text = Text::from_yaml(yaml);
        assert_eq!(text, Err(FromYamlError::MissingTextKey));
    }

    #[test]
    fn yaml_hash_with_bad_text_doesnt_parse() {
        let yaml = YamlLoader::load_from_str("text: 123")
            .unwrap()
            .pop()
            .unwrap();
        let text = Text::from_yaml(yaml);
        assert_matches!(text, Err(FromYamlError::InvalidText(_)));
    }

    #[test]
    fn yaml_hash_with_bad_ascii_doesnt_parse() {
        let yaml = YamlLoader::load_from_str("text: foo\nascii: 123")
            .unwrap()
            .pop()
            .unwrap();
        let text = Text::from_yaml(yaml);
        assert_matches!(text, Err(FromYamlError::InvalidAscii(_)));
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
