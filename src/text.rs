use std::borrow::Cow;
use std::ops::Add;
use std::iter::Sum;
use regex::Regex;
use lazy_static::lazy_static;

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    text: String,
    ascii: Option<String>,
}

impl Text {
    pub fn new<T>(text: T) -> Text
    where
        T: Into<String>,
    {
        Text {
            text: text.into(),
            ascii: None,
        }
    }

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
            static ref RE: Regex = Regex::new(r"^(?i)(?P<article>the|an|a)\s(?P<rest>.*)$").unwrap();
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

impl Add<Text> for Text {
    type Output = Text;

    fn add(self, other: Text) -> Self::Output {
        let ascii = if self.ascii.is_none() && other.ascii.is_none() {
            None
        } else {
            Some(self.ascii().into_owned() + &other.ascii())
        };

        let text = self.text + &other.text;

        Text { text, ascii }
    }
}

impl Sum for Text {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item=Self>,
    {
        iter.fold(Text::new(""), |a, b| a + b)
    }
}

#[cfg(test)]
mod tests {
    use super::Text;

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
    fn texts_sum_text_together() {
        let texts = vec![Text::new("hello"), Text::new("world")];
        assert_eq!(texts.into_iter().sum::<Text>(), Text::new("helloworld"));
    }

    #[test]
    fn texts_sum_asciis_together() {
        let texts = vec![Text::new("hello"), Text::with_ascii("world", "universe")];
        assert_eq!(texts.into_iter().sum::<Text>(), Text::with_ascii("helloworld", "hellouniverse"));
    }
}
