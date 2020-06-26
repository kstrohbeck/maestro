use crate::Text;
use std::borrow::Cow;

/// Get the number of base 10 digits in a number.
///
/// # Examples
///
/// ```rust
/// # use songmaster::utils::num_digits;
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
/// # use songmaster::{text::Text, utils::comma_separated};
/// let text = [Text::from("foo"), Text::from(("bar", "baar")), Text::from("baz")];
/// assert_eq!(Cow::Owned::<Text>(Text::from(("foo, bar, baz", "foo, baar, baz"))), comma_separated(&text[..]));
/// ```
pub fn comma_separated(text: &[Text]) -> Cow<Text> {
    if text.len() == 1 {
        Cow::Borrowed(&text[0])
    } else {
        let mut res = Text::from("");
        let sep = Text::from(", ");
        for (i, t) in text.iter().enumerate() {
            if i != 0 {
                res += &sep;
            }
            res += t;
        }
        Cow::Owned(res)
    }
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
}
