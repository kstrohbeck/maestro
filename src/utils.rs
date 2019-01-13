use crate::text::Text;
use itertools::Itertools;
use std::cmp::max;

pub fn num_digits(mut number: usize) -> usize {
    let mut count = 0;
    while number != 0 {
        number /= 10;
        count += 1;
    }
    max(count, 1)
}

pub fn comma_separated(text: &[Text]) -> Text {
    text.iter().cloned().intersperse(Text::new(", ")).sum()
}

#[cfg(test)]
mod tests {
    use super::{comma_separated, num_digits, Text};

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
        assert_eq!(comma_separated(text), Text::new(""));
    }

    #[test]
    fn comma_separated_single_is_same() {
        let text = &[Text::with_ascii("foo", "bar")];
        assert_eq!(comma_separated(text), Text::with_ascii("foo", "bar"));
    }

    #[test]
    fn comma_separated_double_is_correct() {
        let text = &[Text::with_ascii("foo", "bar"), Text::new("baz")];
        assert_eq!(
            comma_separated(text),
            Text::with_ascii("foo, baz", "bar, baz")
        );
    }

    #[test]
    fn comma_separated_triple_is_correct() {
        let text = &[
            Text::with_ascii("foo", "bar"),
            Text::new("baz"),
            Text::with_ascii("quux", "other"),
        ];
        assert_eq!(
            comma_separated(text),
            Text::with_ascii("foo, baz, quux", "bar, baz, other")
        );
    }
}
