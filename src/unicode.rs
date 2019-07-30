pub fn unicode_to_ascii(c: char) -> Option<char> {
    match c {
        '\u{00a1}' => Some('!'),
        _ => None,
    }
}
