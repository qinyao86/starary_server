const ID_ALPHABET: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

pub fn generate_id(prefix: &str) -> String {
    format!("{prefix}{}", nanoid::nanoid!(12, &ID_ALPHABET))
}

pub fn is_prefixed_id(value: &str, prefix: &str) -> bool {
    let Some(suffix) = value.strip_prefix(prefix) else {
        return false;
    };

    suffix.len() == 12
        && suffix
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}
