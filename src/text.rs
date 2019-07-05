//! Functions for handling text that can have both full, ASCII, and file-safe representations.

use lazy_static::lazy_static;
use regex::Regex;
use std::{
    borrow::Cow,
    fmt::{self, Display},
    iter::Sum,
    ops::{Add, AddAssign},
};
use yaml_rust::Yaml;

/// A piece of text with an overridable ASCII representation.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    text: String,
    ascii: Option<String>,
}

impl Text {
    /// Create a new `Text`.
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

    /// Load `Text` from a Yaml source.
    ///
    /// # Examples
    ///
    /// Loading a simple string:
    ///
    /// ```rust
    /// # use yaml_rust::YamlLoader;
    /// let yaml = YamlLoader::load_from_str("\"foo\"")?[0];
    /// assert_eq!(Text::new("foo"), Text::from_yaml(yaml)?);
    /// # Ok::<(), std::error::Error>(())
    /// ```
    pub fn from_yaml(yaml: Yaml) -> Result<Text, FromYamlError> {
        match yaml {
            Yaml::String(text) => Ok(Text::new(text)),
            Yaml::Hash(mut hash) => Ok({
                let text = pop!(hash["text"])
                    .ok_or(FromYamlError::MissingTextKey)?
                    .into_string()
                    .ok_or(FromYamlError::InvalidText)?;

                let ascii = pop!(hash["ascii"])
                    .map(|y| y.into_string().ok_or(()))
                    .transpose()
                    .map_err(|_| FromYamlError::InvalidAscii)?;

                Text { text, ascii }
            }),
            _ => Err(FromYamlError::NotStringOrHash),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn ascii(&self) -> Cow<str> {
        match &self.ascii {
            Some(asc) => asc.into(),
            None => {
                let text = self.text();
                if text.is_ascii() {
                    text.into()
                } else {
                    text.chars()
                        .map(|c| if c.is_ascii() { c } else { '?' })
                        .collect::<String>()
                        .into()
                }
            }
        }
    }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FromYamlError {
    MissingTextKey,
    InvalidText,
    InvalidAscii,
    NotStringOrHash,
}

impl Display for FromYamlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid yaml")
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

impl Add for Text {
    type Output = Text;

    fn add(self, other: Self) -> Self::Output {
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

impl Sum for Text {
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
    use yaml_rust::YamlLoader;

    fn is_borrowed(cow: Cow<str>) -> bool {
        match cow {
            Cow::Borrowed(_) => true,
            Cow::Owned(_) => false,
        }
    }

    fn is_owned(cow: Cow<str>) -> bool {
        match cow {
            Cow::Borrowed(_) => false,
            Cow::Owned(_) => true,
        }
    }

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
        assert!(is_borrowed(text.ascii()));
    }

    #[test]
    fn ascii_is_borrowed_if_overridden() {
        let text = Text::with_ascii("hello", "goodbye");
        assert!(is_borrowed(text.ascii()));
    }

    #[test]
    fn ascii_is_owned_for_nonascii_text() {
        let text = Text::new("fire = 🔥");
        assert!(is_owned(text.ascii()));
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
        assert!(is_borrowed(text.file_safe()));
    }

    #[test]
    fn file_safe_is_owned_if_text_isnt_ascii() {
        let text = Text::new("fire = 🔥");
        assert!(is_owned(text.file_safe()));
    }

    #[test]
    fn file_safe_is_owned_if_text_isnt_safe() {
        let text = Text::new("foo?");
        assert!(is_owned(text.file_safe()));
    }

    #[test]
    fn file_safe_is_borrowed_if_overridden_ascii_is_safe() {
        let text = Text::with_ascii("foo", "bar");
        assert!(is_borrowed(text.file_safe()));
    }

    #[test]
    fn file_safe_is_owned_if_overridden_ascii_isnt_safe() {
        let text = Text::with_ascii("foo", "bar?");
        assert!(is_owned(text.file_safe()));
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
        assert!(is_borrowed(text.sortable_file_safe()));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_nonascii_text() {
        let text = Text::new("fire = 🔥");
        assert!(is_owned(text.sortable_file_safe()));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_non_file_safe_text() {
        let text = Text::new("foo?");
        assert!(is_owned(text.sortable_file_safe()));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_modified_text() {
        let text = Text::new("A Song Title");
        assert!(is_owned(text.sortable_file_safe()));
    }

    #[test]
    fn sortable_file_safe_is_borrowed_with_unmodified_ascii() {
        let text = Text::with_ascii("foo", "bar");
        assert!(is_borrowed(text.sortable_file_safe()));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_non_safe_ascii() {
        let text = Text::with_ascii("foo", "bar?");
        assert!(is_owned(text.sortable_file_safe()));
    }

    #[test]
    fn sortable_file_safe_is_owned_with_modified_ascii() {
        let text = Text::with_ascii("foo", "the bar");
        assert!(is_owned(text.sortable_file_safe()));
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
