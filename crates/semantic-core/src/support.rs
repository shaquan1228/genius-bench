pub(crate) fn word_count(text: &str) -> i64 {
    text.split(|c| matches!(c, '\u{9}'..='\u{d}' | '\u{1c}'..='\u{20}' | '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'))
        .filter(|word| !word.is_empty())
        .count() as i64
}

pub(crate) fn valid_path(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}

pub(crate) fn valid_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

pub(crate) fn unique<'a>(values: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    values.into_iter().all(|item| seen.insert(item))
}
