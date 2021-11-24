use crate::Text;
use std::borrow::Cow;

/// Get the number of base 10 digits in a number.
///
/// # Examples
///
/// ```rust
/// # use maestro::utils::num_digits;
/// assert_eq!(2, num_digits(12));
/// assert_eq!(3, num_digits(900));
/// ```
pub fn num_digits(mut number: usize) -> usize {
    let mut count = 0;
    while number != 0 {
        number /= 10;
        count += 1;
    }
    std::cmp::max(count, 1)
}

/// Creates a text that is the given list of text separated by commas.
///
/// # Examples
///
/// ```rust
/// # use std::borrow::Cow;
/// # use maestro::{text::Text, utils::comma_separated};
/// let text = [Text::from("foo"), Text::from(("bar", "baar")), Text::from("baz")];
/// assert_eq!(Cow::Owned::<Text>(Text::from(("foo, bar, baz", "foo, baar, baz"))), comma_separated(&text[..]));
/// ```
pub fn comma_separated(text: &[Text]) -> Cow<Text> {
    use crate::text::{COMMA_SEP, EMPTY_TEXT};

    if text.len() == 1 {
        Cow::Borrowed(&text[0])
    } else {
        let mut res = EMPTY_TEXT;
        for (i, t) in text.iter().enumerate() {
            if i != 0 {
                res += COMMA_SEP;
            }
            res += t;
        }
        Cow::Owned(res)
    }
}

macro_rules! expect_char {
    ($cs:expr, $( $c:literal ),*) => {
        let next = $cs.next()?;
        if $( next != $c )&&* {
            return None;
        }
    }
}

/// Splits an initial article from a string.
///
/// Returns a pair of the article and the rest of the string, or None if the string didn't start
/// with an article.
///
/// Articles are "a", "an", and "the", ignoring case.
///
/// ```rust
/// # use maestro::utils::split_article;
/// assert_eq!(split_article("A Thing"), Some(("A", "Thing")));
/// assert_eq!(split_article("Another Thing"), None);
/// ```
pub fn split_article(s: &str) -> Option<(&str, &str)> {
    let mut cs = s.chars();

    match cs.next()? {
        't' | 'T' => {
            expect_char!(cs, 'h', 'H');
            expect_char!(cs, 'e', 'E');
            expect_char!(cs, ' ');
            unsafe { Some((s.get_unchecked(..3), s.get_unchecked(4..))) }
        }
        'a' | 'A' => {
            let next = cs.next()?;
            if next == ' ' {
                return unsafe { Some((s.get_unchecked(..1), s.get_unchecked(2..))) };
            }
            if next != 'n' && next != 'N' {
                return None;
            }
            expect_char!(cs, ' ');

            unsafe { Some((s.get_unchecked(..2), s.get_unchecked(3..))) }
        }
        _ => None,
    }
}

const FILE_UNSAFE_CHARS: [char; 10] = ['<', '>', ':', '"', '/', '|', '~', '\\', '*', '?'];

/// Checks if a string is file safe.
///
/// ```rust
/// # use maestro::utils::is_file_safe;
/// assert_eq!(is_file_safe("foo: bar"), false);
/// assert_eq!(is_file_safe("foo-bar"), true);
/// ```
pub fn is_file_safe(s: &str) -> bool {
    !s.contains(&FILE_UNSAFE_CHARS[..])
}

/// Returns a file safe version of a string, or `None` if it was already file safe.
///
/// ```rust
/// # use maestro::utils::make_file_safe;
/// assert_eq!(make_file_safe("foo: bar"), Some(String::from("foo - bar")));
/// assert_eq!(make_file_safe("foo-bar"), None);
/// ```
pub fn make_file_safe(s: &str) -> Option<String> {
    let mut prefix = None;
    let mut iter = s.char_indices().peekable();

    while let Some((i, c)) = iter.peek() {
        if FILE_UNSAFE_CHARS.contains(c) {
            prefix = Some(String::from(unsafe { s.get_unchecked(..*i) }));
            break;
        }
        iter.next();
    }

    let mut accum = prefix?;

    // TODO: Can we replace this with a for loop?
    while let Some((_, c)) = iter.next() {
        match c {
            '<' => accum.push('['),
            '>' => accum.push(']'),
            ':' => {
                if iter.peek().map(|(_, c)| *c) == Some(' ') {
                    accum.push_str(" -");
                } else {
                    accum.push('-');
                }
            }
            '"' => accum.push('\''),
            '/' | '|' | '~' => accum.push('-'),
            '\\' | '*' => accum.push('_'),
            '?' => {}
            c => accum.push(c),
        }
    }

    Some(accum)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! num_digits_tests {
        ($( $name:ident($num:expr, $digits:expr); )* ) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(num_digits($num), $digits);
                }
            )*
        }
    }

    num_digits_tests! {
        zero_has_one_digit(0, 1);
        one_has_one_digit(1, 1);
        two_has_one_digit(2, 1);
        nine_has_one_digit(9, 1);
        ten_has_two_digits(10, 2);
        eleven_has_two_digits(11, 2);
        ninety_nine_has_two_digits(99, 2);
        one_hundred_has_three_digits(100, 3);
        one_hundred_and_one_has_three_digits(101, 3);
    }

    #[test]
    fn comma_separated_empty_vec_is_empty() {
        let text = &[];
        assert_eq!(Cow::Owned::<Text>(Text::from("")), comma_separated(text));
    }

    #[test]
    fn comma_separated_single_is_same() {
        let text = &[Text::from(("foo", "bar"))];
        assert_eq!(
            Cow::Borrowed(&Text::from(("foo", "bar"))),
            comma_separated(text)
        );
    }

    #[test]
    fn comma_separated_double_is_correct() {
        let text = &[Text::from(("foo", "bar")), Text::from("baz")];
        assert_eq!(
            Cow::Owned::<Text>(Text::from(("foo, baz", "bar, baz"))),
            comma_separated(text),
        );
    }

    #[test]
    fn comma_separated_triple_is_correct() {
        let text = &[
            Text::from(("foo", "bar")),
            Text::from("baz"),
            Text::from(("quux", "other")),
        ];
        assert_eq!(
            Cow::Owned::<Text>(Text::from(("foo, baz, quux", "bar, baz, other"))),
            comma_separated(text),
        );
    }

    #[test]
    fn split_article_preserves_capitalization() {
        assert_eq!(split_article("THe titLe"), Some(("THe", "titLe")));
    }

    #[test]
    fn split_article_only_removes_first_space() {
        assert_eq!(split_article("the   title"), Some(("the", "  title")));
    }

    #[test]
    fn split_article_doesnt_split_if_no_space() {
        assert_eq!(split_article("the_title"), None);
    }

    mod make_file_safe {
        use super::{is_file_safe, make_file_safe};
        use quickcheck::TestResult;
        use quickcheck_macros::quickcheck;

        #[quickcheck]
        fn returns_none_for_file_safe_string(a: String) -> TestResult {
            if !is_file_safe(&a) {
                TestResult::discard()
            } else {
                TestResult::from_bool(make_file_safe(&a).is_none())
            }
        }

        #[quickcheck]
        fn returns_some_for_non_file_safe_string(a: String) -> TestResult {
            if is_file_safe(&a) {
                TestResult::discard()
            } else {
                TestResult::from_bool(make_file_safe(&a).is_some())
            }
        }

        #[quickcheck]
        fn result_doesnt_contain_non_file_safe_chars(a: String) -> TestResult {
            if let Some(s) = make_file_safe(&a) {
                TestResult::from_bool(is_file_safe(&s))
            } else {
                TestResult::discard()
            }
        }

        #[test]
        fn result_has_spaced_hyphen_instead_of_spaced_colon() {
            assert_eq!(make_file_safe("foo: bar"), Some(String::from("foo - bar")));
        }

        #[test]
        fn result_has_unspaced_hyphen_instead_of_unspaced_colon() {
            assert_eq!(make_file_safe("foo:bar"), Some(String::from("foo-bar")));
        }
    }
}
