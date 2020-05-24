use std::borrow::Cow;
use std::ops::Add;

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

impl Text {
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
            Ascii::Different {
                value: calculate_ascii(&ovr)
                    .map(Into::into)
                    .unwrap_or_else(|| ovr.into()),
                is_overridden: true,
            }
        } else {
            match calculate_ascii(&value) {
                Some(value) => Ascii::Different {
                    value: value.into(),
                    is_overridden: false,
                },
                None => Ascii::Same,
            }
        };

        let ascii_for_value = ascii.for_value(&value);
        let file_safe =
            if ascii_for_value.contains(&['<', '>', ':', '"', '/', '|', '~', '\\', '*', '?'][..]) {
                let mut buf = String::with_capacity(ascii_for_value.len());
                for c in ascii_for_value.chars() {
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
                Some(buf)
            } else {
                None
            };

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
    /// let text = Text::from_string("bók?");
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
    /// let text = Text::from_string("bók?");
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
    /// let text = Text::from_string("bók?");
    /// assert_eq!("bok", text.ascii());
    /// ```
    pub fn file_safe(&self) -> &str {
        self.file_safe
            .as_ref()
            .map(String::as_str)
            .unwrap_or(self.ascii())
    }

    /// Get a sortable filename safe representation of the text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from_string("the bók");
    /// assert_eq!("bok, the", text.sortable_file_safe());
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
            None => file_safe.into(),
            Some(caps) => {
                let article = caps.name("article").unwrap().as_str();
                let rest = caps.name("rest").unwrap().as_str();
                format!("{}, {}", rest, article).into()
            }
        }
    }

    /// Return if the text's ASCII representation has been manually overridden.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::from_string("bók");
    /// assert!(!text.has_overridden_ascii());
    /// ```
    ///
    /// ```rust
    /// # use songmaster::Text;
    /// let text = Text::new("bók", Some("book"));
    /// assert!(text.has_overridden_ascii());
    /// ```
    pub fn has_overridden_ascii(&self) -> bool {
        self.ascii.is_overridden()
    }
}

impl Default for Text {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Add<Text> for Text {
    type Output = Text;

    fn add(self, other: Text) -> Self::Output {
        if self == Text::from_string("") {
            return other;
        } else if other == Text::from_string("") {
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
        if other == &Text::from_string("") {
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
        if self == &Text::from_string("") {
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

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

    impl Arbitrary for Text {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            fn arbitrary_filtered_ascii_string<F, G: Gen>(g: &mut G, f: F) -> String
            where
                F: Fn(&char) -> bool,
            {
                Vec::<u8>::arbitrary(g)
                    .into_iter()
                    .map(Into::<char>::into)
                    .filter(f)
                    .collect()
            }

            /// Generate a string containing only alphanumeric & space ASCII characters.
            fn arbitrary_file_safe_string<G: Gen>(g: &mut G) -> String {
                arbitrary_filtered_ascii_string(g, |c| c.is_alphanumeric() || *c == ' ')
            }

            /// Generate a string containing only ASCII characters.
            fn arbitrary_ascii_string<G: Gen>(g: &mut G) -> String {
                arbitrary_filtered_ascii_string(g, |_| true)
            }

            let value = match u8::arbitrary(g) % 3 {
                0 => arbitrary_file_safe_string(g),
                1 => arbitrary_ascii_string(g),
                2 => String::arbitrary(g),
                _ => unreachable!(),
            };

            let ascii = match u8::arbitrary(g) % 3 {
                0 => Some(arbitrary_file_safe_string(g)),
                1 => Some(arbitrary_ascii_string(g)),
                2 => None,
                _ => unreachable!(),
            };

            Text::new(value, ascii)
        }

        // TODO: Implement a better shrinking strategy.
    }

    #[quickcheck]
    fn ascii_is_different_than_value_if_calculated(a: Text) -> TestResult {
        if !matches!(a.ascii, Ascii::Different { is_overridden: false, ..}) {
            return TestResult::discard();
        }
        TestResult::from_bool(a.value() != a.ascii())
    }

    #[quickcheck]
    fn file_safe_is_different_than_ascii_if_set(a: Text) -> TestResult {
        if a.file_safe.is_none() {
            return TestResult::discard();
        }
        TestResult::from_bool(a.ascii() != a.file_safe())
    }

    #[quickcheck]
    fn ascii_has_only_ascii_chars(a: Text) -> bool {
        a.ascii().is_ascii()
    }

    #[quickcheck]
    fn file_safe_has_only_ascii_chars(a: Text) -> bool {
        a.file_safe().is_ascii()
    }

    #[quickcheck]
    fn ascii_is_different_than_value_if_it_is_nonascii(a: Text) -> TestResult {
        if a.value().is_ascii() {
            return TestResult::discard();
        }
        TestResult::from_bool(a.value() != a.ascii())
    }

    #[quickcheck]
    fn ascii_is_same_as_value_if_ascii_and_nonoverridden(a: Text) -> TestResult {
        if !a.value().is_ascii() || a.has_overridden_ascii() {
            return TestResult::discard();
        }
        TestResult::from_bool(a.value() == a.ascii())
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
        fn addition_is_associative(a: Text, b: Text, c: Text) -> bool {
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
            if !(a.value() != a.ascii()) {
                return TestResult::discard();
            }
            let res = a + b;
            TestResult::from_bool(res.value() != res.ascii())
        }

        #[quickcheck]
        fn ascii_difference_is_right_absorbing(a: Text, b: Text) -> TestResult {
            if !(a.value() != a.ascii()) {
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
}
