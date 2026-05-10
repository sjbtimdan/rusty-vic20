use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub type PasteQueue = Arc<Mutex<VecDeque<u8>>>;

pub fn new_paste_queue() -> PasteQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub const KBD_BUFFER_COUNT: u16 = 0x00C6;
pub const KBD_BUFFER_START: u16 = 0x0277;
pub const KBD_BUFFER_SIZE: u8 = 10;

pub fn char_to_petscii(c: char) -> Option<u8> {
    match c {
        '\n' | '\r' => Some(0x0D),
        ' ' => Some(0x20),
        '!' => Some(0x21),
        '"' => Some(0x22),
        '#' => Some(0x23),
        '$' => Some(0x24),
        '%' => Some(0x25),
        '&' => Some(0x26),
        '\'' => Some(0x27),
        '(' => Some(0x28),
        ')' => Some(0x29),
        '*' => Some(0x2A),
        '+' => Some(0x2B),
        ',' => Some(0x2C),
        '-' => Some(0x2D),
        '.' => Some(0x2E),
        '/' => Some(0x2F),
        '0'..='9' => Some(c as u8),
        ':' => Some(0x3A),
        ';' => Some(0x3B),
        '<' => Some(0x3C),
        '=' => Some(0x3D),
        '>' => Some(0x3E),
        '?' => Some(0x3F),
        '@' => Some(0x40),
        'A'..='Z' => Some(c as u8),
        '[' => Some(0x5B),
        ']' => Some(0x5D),
        'a'..='z' => Some((c as u8) - 32),
        '£' => Some(0x5C),
        '↑' | '^' => Some(0x5E),
        '←' | '_' => Some(0x5F),
        '─' | '`' => Some(0x60),
        '♥' => Some(0x53),
        _ => None,
    }
}

pub fn text_to_petscii(text: &str) -> Vec<u8> {
    text.chars().filter_map(char_to_petscii).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase_letters_map_directly() {
        assert_eq!(char_to_petscii('A'), Some(0x41));
        assert_eq!(char_to_petscii('Z'), Some(0x5A));
    }

    #[test]
    fn lowercase_letters_map_to_uppercase() {
        assert_eq!(char_to_petscii('a'), Some(0x41));
        assert_eq!(char_to_petscii('z'), Some(0x5A));
    }

    #[test]
    fn digits_map_directly() {
        assert_eq!(char_to_petscii('0'), Some(0x30));
        assert_eq!(char_to_petscii('9'), Some(0x39));
    }

    #[test]
    fn space_maps_correctly() {
        assert_eq!(char_to_petscii(' '), Some(0x20));
    }

    #[test]
    fn newline_maps_to_return() {
        assert_eq!(char_to_petscii('\n'), Some(0x0D));
        assert_eq!(char_to_petscii('\r'), Some(0x0D));
    }

    #[test]
    fn common_symbols_map() {
        assert_eq!(char_to_petscii('!'), Some(0x21));
        assert_eq!(char_to_petscii('"'), Some(0x22));
        assert_eq!(char_to_petscii('#'), Some(0x23));
        assert_eq!(char_to_petscii('$'), Some(0x24));
        assert_eq!(char_to_petscii('%'), Some(0x25));
        assert_eq!(char_to_petscii('&'), Some(0x26));
        assert_eq!(char_to_petscii('\''), Some(0x27));
        assert_eq!(char_to_petscii('('), Some(0x28));
        assert_eq!(char_to_petscii(')'), Some(0x29));
        assert_eq!(char_to_petscii('*'), Some(0x2A));
        assert_eq!(char_to_petscii('+'), Some(0x2B));
        assert_eq!(char_to_petscii(','), Some(0x2C));
        assert_eq!(char_to_petscii('-'), Some(0x2D));
        assert_eq!(char_to_petscii('.'), Some(0x2E));
        assert_eq!(char_to_petscii('/'), Some(0x2F));
        assert_eq!(char_to_petscii(':'), Some(0x3A));
        assert_eq!(char_to_petscii(';'), Some(0x3B));
        assert_eq!(char_to_petscii('<'), Some(0x3C));
        assert_eq!(char_to_petscii('='), Some(0x3D));
        assert_eq!(char_to_petscii('>'), Some(0x3E));
        assert_eq!(char_to_petscii('?'), Some(0x3F));
        assert_eq!(char_to_petscii('@'), Some(0x40));
        assert_eq!(char_to_petscii('['), Some(0x5B));
        assert_eq!(char_to_petscii(']'), Some(0x5D));
    }

    #[test]
    fn special_unicode_maps() {
        assert_eq!(char_to_petscii('♥'), Some(0x53));
        assert_eq!(char_to_petscii('^'), Some(0x5E));
        assert_eq!(char_to_petscii('_'), Some(0x5F));
        assert_eq!(char_to_petscii('`'), Some(0x60));
        assert_eq!(char_to_petscii('£'), Some(0x5C));
    }

    #[test]
    fn unknown_chars_return_none() {
        assert_eq!(char_to_petscii('\0'), None);
        assert_eq!(char_to_petscii('±'), None);
        assert_eq!(char_to_petscii('~'), None);
    }

    #[test]
    fn text_to_petscii_converts_string() {
        let result = text_to_petscii("HELLO WORLD!");
        assert_eq!(
            result,
            vec![0x48, 0x45, 0x4C, 0x4C, 0x4F, 0x20, 0x57, 0x4F, 0x52, 0x4C, 0x44, 0x21]
        );
    }

    #[test]
    fn text_to_petscii_skips_unknown_chars() {
        let result = text_to_petscii("HI~\nTHERE");
        assert_eq!(result, vec![0x48, 0x49, 0x0D, 0x54, 0x48, 0x45, 0x52, 0x45]);
    }
}
