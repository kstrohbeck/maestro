use crate::utils::make_file_safe;
use serde::{de, ser, Deserialize, Serialize};
use std::borrow::Cow;
use std::ops::{Add, AddAssign};

/// Adds two cows together, reusing allocations if possible.
fn add_cows<'a>(left: Cow<'a, str>, right: Cow<'a, str>) -> String {
    if let Cow::Owned(mut left) = left {
        left.push_str(&right);
        left
    } else if let Cow::Owned(mut right) = right {
        right.insert_str(0, &left);
        right
    } else {
        format!("{}{}", left, right)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The ASCII value of a piece of text.
enum Ascii {
    /// ASCII is same as the regular value.
    Same,

    /// ASCII is not the same as the regular value.
    Different {
        /// The ASCII's value.
        value: Cow<'static, str>,

        /// If the ASCII is overridden or calculated.
        is_overridden: bool,
    },
}

// Helper macro that passes on owned Cow values.
macro_rules! for_value {
    ($ascii:ident, $text:ident) => {
        match $ascii {
            Self::Same => Cow::Borrowed($text),
            Self::Different { value, .. } => value,
        }
    };
    (ref $ascii:ident, $text:ident) => {
        Cow::Borrowed($ascii.for_value($text))
    };
}

impl Ascii {
    /// Returns an ASCII for the value that's overridden.
    fn overridden(value: Cow<'static, str>) -> Self {
        Self::Different {
            value,
            is_overridden: true,
        }
    }

    /// Returns an ASCII for the value that isn't overridden.
    fn calculated(value: String) -> Self {
        Self::Different {
            value: value.into(),
            is_overridden: false,
        }
    }

    /// Return the inner ASCII string if it is different than the value.
    fn inner(&self) -> Option<&str> {
        match self {
            Ascii::Same => None,
            Ascii::Different { value, .. } => Some(value),
        }
    }

    /// Return the ASCII string, or `value` if the ASCII is the same.
    fn for_value<'a>(&'a self, value: &'a str) -> &'a str {
        self.inner().unwrap_or(value)
    }

    /// Returns if the ASCII value is manually overridden.
    fn is_overridden(&self) -> bool {
        match self {
            Self::Same => false,
            Self::Different { is_overridden, .. } => *is_overridden,
        }
    }

    /// Adds an owned Ascii to an owned Ascii.
    fn add_owned_to_owned(self, left: &str, other: Ascii, right: &str) -> Ascii {
        if self == Self::Same && other == Self::Same {
            return Self::Same;
        }

        let is_overridden = self.is_overridden() || other.is_overridden();
        let left = for_value!(self, left);
        let right = for_value!(other, right);
        let value = add_cows(left, right).into();

        Self::Different {
            value,
            is_overridden,
        }
    }

    /// Adds an owned `Ascii` to a borrowed `Ascii`.
    fn add_owned_to_ref(self, left: &str, other: &Ascii, right: &str) -> Ascii {
        if self == Self::Same && other == &Self::Same {
            return Self::Same;
        }

        let is_overridden = self.is_overridden() || other.is_overridden();
        let left = for_value!(self, left);
        let right = for_value!(ref other, right);
        let value = add_cows(left, right).into();

        Self::Different {
            value,
            is_overridden,
        }
    }

    /// Adds a borrowed `Ascii` to an owned `Ascii`.
    fn add_ref_to_owned(&self, left: &str, other: Ascii, right: &str) -> Ascii {
        if self == &Self::Same && other == Self::Same {
            return Self::Same;
        }

        let is_overridden = self.is_overridden() || other.is_overridden();
        let left = for_value!(ref self, left);
        let right = for_value!(other, right);
        let value = add_cows(left, right).into();

        Self::Different {
            value,
            is_overridden,
        }
    }

    /// Adds a borrowed `Ascii` to a borrowed `Ascii`.
    fn add_ref_to_ref(&self, left: &str, other: &Ascii, right: &str) -> Ascii {
        if let (Self::Same, Self::Same) = (self, other) {
            return Self::Same;
        }

        let is_overridden = self.is_overridden() || other.is_overridden();
        let left = for_value!(ref self, left);
        let right = for_value!(ref other, right);
        let value = add_cows(left, right).into();

        Self::Different {
            value,
            is_overridden,
        }
    }
}

/// A piece of text with different representations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Text {
    /// The normal content of the text.
    value: Cow<'static, str>,

    /// The ASCII representation of the text.
    ascii: Ascii,

    /// A version of the text safe to use in filenames, if it's different from the ASCII
    /// version.
    file_safe: Option<String>,
}

/// The empty text. Useful for string concatenation.
pub const EMPTY_TEXT: Text = unsafe { Text::from_str_unchecked("") };

/// A comma separator. Useful for joining a list of `Text`s.
pub const COMMA_SEP: Text = unsafe { Text::from_str_unchecked(", ") };

impl Text {
    /// Create a constant `Text` from a file-safe string constant.
    ///
    /// # Safety
    ///
    /// The passed-in string must be both entirely ascii and also file-safe.
    const unsafe fn from_str_unchecked(value: &'static str) -> Self {
        Self {
            value: Cow::Borrowed(value),
            ascii: Ascii::Same,
            file_safe: None,
        }
    }

    /// Create a new `Text` from regular text and an optionally overridden ASCII value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::new("本", Some("book"));
    /// assert_eq!("本", text.value());
    /// assert_eq!("book", text.ascii());
    /// ```
    pub fn new<T, U>(value: T, ascii: Option<U>) -> Self
    where
        T: Into<Cow<'static, str>>,
        U: Into<Cow<'static, str>>,
    {
        fn calculate_ascii(s: &str) -> Option<String> {
            use unicode_normalization::UnicodeNormalization;

            if s.is_ascii() {
                return None;
            }

            fn char_ascii(c: char) -> Option<char> {
                if c.is_ascii() {
                    Some(c)
                } else if c == '‘' || c == '’' {
                    Some('\'')
                } else {
                    None
                }
            }

            s.nfkd().filter_map(char_ascii).collect::<String>().into()
        }

        let value: Cow<str> = value.into();
        let ascii: Option<Cow<str>> = ascii.map(Into::into);

        let ascii = if let Some(ovr) = ascii {
            let value = calculate_ascii(&ovr).map(Into::into).unwrap_or(ovr);
            Ascii::overridden(value)
        } else if let Some(value) = calculate_ascii(&value) {
            Ascii::calculated(value)
        } else {
            Ascii::Same
        };

        let ascii_for_value = ascii.for_value(&value);
        let file_safe = make_file_safe(ascii_for_value);

        Self {
            value,
            ascii,
            file_safe,
        }
    }

    /// Create a new `Text` from regular text without an override.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from_string("bók");
    /// assert_eq!("bók", text.value());
    /// assert_eq!("bok", text.ascii());
    /// ```
    pub fn from_string<T>(value: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::new::<_, &str>(value, None)
    }

    /// Get the regular value of the text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from("bók?");
    /// assert_eq!("bók?", text.value());
    /// ```
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Get the ASCII representation of the text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from("bók?");
    /// assert_eq!("bok?", text.ascii());
    /// ```
    pub fn ascii(&self) -> &str {
        self.ascii.for_value(self.value())
    }

    /// Get the filename safe representation of the text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from("bók?");
    /// assert_eq!("bok", text.file_safe());
    /// ```
    pub fn file_safe(&self) -> &str {
        self.file_safe.as_deref().unwrap_or_else(|| self.ascii())
    }

    /// Get a sortable filename safe representation of the text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from("the bók");
    /// assert_eq!("bok, the", text.sortable_file_safe());
    /// ```
    pub fn sortable_file_safe(&self) -> Cow<str> {
        use crate::utils::split_article;

        let file_safe = self.file_safe();
        if let Some((article, rest)) = split_article(&file_safe) {
            format!("{}, {}", rest, article).into()
        } else {
            file_safe.into()
        }
    }

    /// Return if the text's ASCII representation has been manually overridden.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from("bók");
    /// assert!(!text.has_overridden_ascii());
    /// ```
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from(("bók", "book"));
    /// assert!(text.has_overridden_ascii());
    /// ```
    pub fn has_overridden_ascii(&self) -> bool {
        self.ascii.is_overridden()
    }
}

impl Default for Text {
    fn default() -> Self {
        // TODO: Implement.
        unimplemented!()
    }
}

impl From<&'static str> for Text {
    fn from(value: &'static str) -> Text {
        Text::from_string(value)
    }
}

impl From<String> for Text {
    fn from(text: String) -> Text {
        Text::from_string(text)
    }
}

impl From<(&'static str, &'static str)> for Text {
    fn from((value, ascii): (&'static str, &'static str)) -> Text {
        Text::new(value, Some(ascii))
    }
}

impl From<(String, &'static str)> for Text {
    fn from((value, ascii): (String, &'static str)) -> Text {
        Text::new(value, Some(ascii))
    }
}

impl From<(&'static str, String)> for Text {
    fn from((value, ascii): (&'static str, String)) -> Text {
        Text::new(value, Some(ascii))
    }
}

impl From<(String, String)> for Text {
    fn from((value, ascii): (String, String)) -> Text {
        Text::new(value, Some(ascii))
    }
}

impl Add<Text> for Text {
    type Output = Text;

    fn add(self, other: Text) -> Self::Output {
        if self == EMPTY_TEXT {
            return other;
        } else if other == EMPTY_TEXT {
            return self;
        }

        let file_safe = match (self.file_safe, other.file_safe) {
            (None, None) => None,
            (Some(mut a), None) => {
                let other_ascii = other.ascii.for_value(&other.value);
                a.push_str(other_ascii);
                Some(a)
            }
            (None, Some(mut b)) => {
                let self_ascii = self.ascii.for_value(&self.value);
                b.insert_str(0, self_ascii);
                Some(b)
            }
            (Some(mut a), Some(b)) => {
                a.push_str(&b);
                Some(a)
            }
        };

        let ascii = self
            .ascii
            .add_owned_to_owned(&self.value, other.ascii, &other.value);

        let value = add_cows(self.value, other.value).into();

        Self {
            value,
            ascii,
            file_safe,
        }
    }
}

impl Add<&Text> for Text {
    type Output = Text;

    fn add(self, other: &Text) -> Self::Output {
        if other == &EMPTY_TEXT {
            return self;
        }

        let file_safe = if let Some(mut a) = self.file_safe {
            a.push_str(other.file_safe());
            Some(a)
        } else if let Some(ref b) = other.file_safe {
            Some(format!("{}{}", self.ascii(), b))
        } else {
            None
        };

        let ascii = self
            .ascii
            .add_owned_to_ref(&self.value, &other.ascii, other.value());

        let value = add_cows(self.value, other.value.clone()).into();

        Self {
            value,
            ascii,
            file_safe,
        }
    }
}

impl Add<Text> for &Text {
    type Output = Text;

    fn add(self, other: Text) -> Self::Output {
        if self == &EMPTY_TEXT {
            return other;
        }

        let file_safe = if let Some(mut b) = other.file_safe {
            b.insert_str(0, self.file_safe());
            Some(b)
        } else if let Some(ref a) = self.file_safe {
            Some(format!("{}{}", a, other.ascii()))
        } else {
            None
        };

        let ascii = self
            .ascii
            .add_ref_to_owned(&self.value, other.ascii, &other.value);

        let value: Cow<str> = add_cows(self.value.clone(), other.value).into();

        Text {
            value,
            ascii,
            file_safe,
        }
    }
}

impl<'a, 'b> Add<&'a Text> for &'b Text {
    type Output = Text;

    fn add(self, other: &'a Text) -> Self::Output {
        let file_safe = if (&self.file_safe, &other.file_safe) == (&None, &None) {
            None
        } else {
            Some(format!("{}{}", self.file_safe(), other.file_safe()))
        };

        let ascii = self
            .ascii
            .add_ref_to_ref(&self.value, &other.ascii, &other.value);

        let value = add_cows(self.value.clone(), other.value.clone()).into();

        Text {
            value,
            ascii,
            file_safe,
        }
    }
}

impl AddAssign<Text> for Text {
    fn add_assign(&mut self, other: Text) {
        // TODO: Actually implement this correctly.
        *self = &*self + other;
    }
}

impl AddAssign<&Text> for Text {
    fn add_assign(&mut self, other: &Text) {
        // TODO: Actually implement this correctly.
        *self = &*self + other;
    }
}

impl Serialize for Text {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct;

        match &self.ascii {
            Ascii::Different {
                value,
                is_overridden: true,
            } => {
                let mut state = serializer.serialize_struct("Text", 2)?;
                state.serialize_field("text", &self.value)?;
                state.serialize_field("ascii", &value)?;
                state.end()
            }
            _ => serializer.serialize_str(&self.value),
        }
    }
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        use std::fmt;
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
                Ok(Text::from(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Text::from(value))
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

                let mut text: Option<String> = None;
                let mut ascii: Option<String> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Fields::Text => field!(map, text),
                        Fields::Ascii => field!(map, ascii),
                    }
                }

                let text = text.ok_or_else(|| de::Error::missing_field("text"))?;
                Ok(Text::new(text, ascii))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

    #[derive(Debug, Clone)]
    struct AsciiString(String);

    impl From<String> for AsciiString {
        fn from(s: String) -> Self {
            Self(s)
        }
    }

    impl Arbitrary for AsciiString {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Vec::<u8>::arbitrary(g)
                .into_iter()
                .map(Into::<char>::into)
                .collect::<String>()
                .into()
        }
    }

    impl From<AsciiString> for String {
        fn from(s: AsciiString) -> Self {
            s.0
        }
    }

    #[derive(Debug, Clone)]
    struct FileSafeString(String);

    impl From<String> for FileSafeString {
        fn from(s: String) -> Self {
            Self(s)
        }
    }

    impl Arbitrary for FileSafeString {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Vec::<u8>::arbitrary(g)
                .into_iter()
                .map(Into::<char>::into)
                .filter(|c| c.is_alphanumeric() || *c == ' ')
                .collect::<String>()
                .into()
        }
    }

    impl From<FileSafeString> for String {
        fn from(s: FileSafeString) -> Self {
            s.0
        }
    }

    impl Arbitrary for Text {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let value = match u8::arbitrary(g) % 3 {
                0 => FileSafeString::arbitrary(g).into(),
                1 => AsciiString::arbitrary(g).into(),
                2 => String::arbitrary(g),
                _ => unreachable!(),
            };

            let ascii: Option<String> = match u8::arbitrary(g) % 3 {
                0 => Some(FileSafeString::arbitrary(g).into()),
                1 => Some(AsciiString::arbitrary(g).into()),
                2 => None,
                _ => unreachable!(),
            };

            Text::new(value, ascii)
        }

        // TODO: Implement a better shrinking strategy.
    }

    mod value {
        use super::*;

        #[quickcheck]
        fn is_the_value_passed_to_from_string(a: String) -> bool {
            Text::from_string(a.clone()).value() == a
        }

        #[quickcheck]
        fn is_the_value_passed_to_new_without_ascii(a: String) -> bool {
            Text::new::<_, &str>(a.clone(), None).value() == a
        }

        #[quickcheck]
        fn is_the_value_passed_to_new_with_ascii(a: String, b: String) -> bool {
            Text::new(a.clone(), Some(b)).value() == a
        }
    }

    mod ascii {
        use super::*;

        #[test]
        fn is_value_with_nonascii_removed_if_calculated() {
            let text = Text::from_string("bók");
            assert_eq!(text.ascii(), "bok");
        }

        #[quickcheck]
        fn is_the_value_passed_to_new_if_overridden(a: String, b: AsciiString) -> TestResult {
            let b: String = b.into();
            TestResult::from_bool(Text::new(a, Some(b.clone())).ascii() == b)
        }

        #[quickcheck]
        fn has_only_ascii_chars(a: Text) -> bool {
            a.ascii().is_ascii()
        }

        #[quickcheck]
        fn is_same_as_value_if_ascii_and_nonoverridden(a: Text) -> TestResult {
            if !a.value().is_ascii() || a.has_overridden_ascii() {
                return TestResult::discard();
            }
            TestResult::from_bool(a.value() == a.ascii())
        }

        #[quickcheck]
        fn is_different_than_value_if_calculated(a: Text) -> TestResult {
            if !matches!(a.ascii, Ascii::Different { is_overridden: false, ..}) {
                return TestResult::discard();
            }
            TestResult::from_bool(a.value() != a.ascii())
        }

        #[quickcheck]
        fn is_different_than_value_if_it_is_nonascii(a: Text) -> TestResult {
            if a.value().is_ascii() {
                return TestResult::discard();
            }
            TestResult::from_bool(a.value() != a.ascii())
        }
    }

    mod file_safe {
        use super::*;

        #[quickcheck]
        fn has_only_ascii_chars(a: Text) -> bool {
            a.file_safe().is_ascii()
        }

        #[quickcheck]
        fn doesnt_contain_non_file_safe_chars(a: Text) -> bool {
            !a.file_safe()
                .contains(&['<', '>', ':', '"', '/', '|', '~', '\\', '*', '?'][..])
        }

        #[quickcheck]
        fn is_same_as_ascii_if_not_set(a: Text) -> TestResult {
            if a.file_safe.is_some() {
                return TestResult::discard();
            }
            TestResult::from_bool(a.ascii() == a.file_safe())
        }

        #[quickcheck]
        fn is_different_than_ascii_if_set(a: Text) -> TestResult {
            if a.file_safe.is_none() {
                return TestResult::discard();
            }
            TestResult::from_bool(a.ascii() != a.file_safe())
        }
    }

    mod add {
        use super::*;

        #[quickcheck]
        fn empty_is_left_identity(a: Text) -> bool {
            Text::from_string("") + &a == a
        }

        #[quickcheck]
        fn empty_is_right_identity(a: Text) -> bool {
            &a + Text::from_string("") == a
        }

        #[quickcheck]
        fn is_associative(a: Text, b: Text, c: Text) -> bool {
            (&a + &b) + &c == a + (b + c)
        }

        #[quickcheck]
        fn ref_plus_owned_is_correct(a: Text, b: Text) -> bool {
            &a + b.clone() == a + b
        }

        #[quickcheck]
        fn owned_plus_ref_is_correct(a: Text, b: Text) -> bool {
            a.clone() + &b == a + b
        }

        #[quickcheck]
        fn ref_plus_ref_is_correct(a: Text, b: Text) -> bool {
            &a + &b == a + b
        }

        #[quickcheck]
        fn values_are_added(a: Text, b: Text) -> bool {
            (&a + &b).value() == format!("{}{}", a.value(), b.value())
        }

        #[quickcheck]
        fn ascii_reprs_are_added(a: Text, b: Text) -> bool {
            (&a + &b).ascii() == format!("{}{}", a.ascii(), b.ascii())
        }

        #[quickcheck]
        fn file_safe_reprs_are_added(a: Text, b: Text) -> bool {
            (&a + &b).file_safe() == format!("{}{}", a.file_safe(), b.file_safe())
        }

        #[quickcheck]
        fn ascii_difference_is_left_absorbing(a: Text, b: Text) -> TestResult {
            if a.value() == a.ascii() {
                return TestResult::discard();
            }
            let res = a + b;
            TestResult::from_bool(res.value() != res.ascii())
        }

        #[quickcheck]
        fn ascii_difference_is_right_absorbing(a: Text, b: Text) -> TestResult {
            if a.value() == a.ascii() {
                return TestResult::discard();
            }
            let res = b + a;
            TestResult::from_bool(res.value() != res.ascii())
        }

        #[quickcheck]
        fn override_is_left_absorbing(a: Text, b: Text) -> TestResult {
            if !a.has_overridden_ascii() {
                return TestResult::discard();
            }
            TestResult::from_bool((a + b).has_overridden_ascii())
        }

        #[quickcheck]
        fn override_is_right_absorbing(a: Text, b: Text) -> TestResult {
            if !a.has_overridden_ascii() {
                return TestResult::discard();
            }
            TestResult::from_bool((b + a).has_overridden_ascii())
        }
    }

    mod serde {
        use super::*;

        #[test]
        fn simple_text_is_serde_equal() {
            let text = Text::from_string("foo");
            let new_text: Text = serde_yaml::to_string(&text)
                .and_then(|s| serde_yaml::from_str(&s))
                .unwrap();
            assert_eq!(text, new_text);
        }

        #[test]
        fn ascii_text_is_serde_equal() {
            let text = Text::new("foo", Some("bar"));
            let new_text: Text = serde_yaml::to_string(&text)
                .and_then(|s| serde_yaml::from_str(&s))
                .unwrap();
            assert_eq!(text, new_text);
        }
    }

    mod ser {
        use super::*;

        #[test]
        fn simple_text_serializes_to_string() {
            use serde_yaml::Value;
            let text = Text::from_string("foo");
            let yaml = serde_yaml::to_value(&text).unwrap();
            let expected: Value = "foo".into();
            assert_eq!(expected, yaml);
        }

        #[test]
        fn overridden_ascii_text_serializes_to_struct() {
            use serde_yaml::Value;
            let text = Text::new("foo", Some("bar"));
            let yaml = serde_yaml::to_value(&text).unwrap();
            let mapping = match yaml {
                Value::Mapping(mapping) => mapping,
                _ => panic!("yaml wasn't a mapping"),
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
        use super::*;

        #[test]
        fn simple_yaml_parses_text() {
            let text = serde_yaml::from_str("\"foo\"").unwrap();
            assert_eq!(Text::from_string("foo"), text);
        }

        #[test]
        fn yaml_with_only_text_parses_text() {
            let text = serde_yaml::from_str("text: foo").unwrap();
            assert_eq!(Text::from_string("foo"), text);
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
            assert_eq!(Text::new("foo", Some("bar")), text);
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
}
