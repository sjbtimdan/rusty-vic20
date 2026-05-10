use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::{self, Receiver, SyncSender},
};

use crate::ui::keyboard::Key;

pub fn make_keyboard_channel() -> (SyncSender<HashSet<Key>>, Receiver<HashSet<Key>>) {
    mpsc::sync_channel(2)
}

pub struct Keyboard {
    cache: HashSet<Key>,
    receiver: Receiver<HashSet<Key>>,
    keyboard_map: HashMap<(Key, u8), u8>,
}

fn keyboard_map() -> HashMap<(Key, u8), u8> {
    let mut map = HashMap::new();
    let base_mappings = vec![
        // Column c0 (port_b = 0xFE)
        (Key::Single('1'), 0xFE, 0xFE),
        (Key::Single('3'), 0xFE, 0xFD),
        (Key::Single('5'), 0xFE, 0xFB),
        (Key::Single('7'), 0xFE, 0xF7),
        (Key::Single('9'), 0xFE, 0xEF),
        (Key::Single('+'), 0xFE, 0xDF),
        (Key::Single('£'), 0xFE, 0xBF),
        (Key::InsDel, 0xFE, 0x7F),
        // Column c1 (port_b = 0xFD)
        (Key::Left, 0xFD, 0xFE),
        (Key::Single('W'), 0xFD, 0xFD),
        (Key::Single('R'), 0xFD, 0xFB),
        (Key::Single('Y'), 0xFD, 0xF7),
        (Key::Single('I'), 0xFD, 0xEF),
        (Key::Single('P'), 0xFD, 0xDF),
        (Key::Single('*'), 0xFD, 0xBF),
        (Key::Return, 0xFD, 0x7F),
        // Column c2 (port_b = 0xFB)
        (Key::Ctrl, 0xFB, 0xFE),
        (Key::Single('A'), 0xFB, 0xFD),
        (Key::Single('D'), 0xFB, 0xFB),
        (Key::Single('G'), 0xFB, 0xF7),
        (Key::Single('J'), 0xFB, 0xEF),
        (Key::Single('L'), 0xFB, 0xDF),
        (Key::Single(';'), 0xFB, 0xBF),
        (Key::CrsrLR, 0xFB, 0x7F),
        // Column c3 (port_b = 0xF7)
        (Key::RunStop, 0xF7, 0xFE),
        (Key::Shift, 0xF7, 0xFD),
        (Key::Single('X'), 0xF7, 0xFB),
        (Key::Single('V'), 0xF7, 0xF7),
        (Key::Single('N'), 0xF7, 0xEF),
        (Key::Single(','), 0xF7, 0xDF),
        (Key::Single('/'), 0xF7, 0xBF),
        (Key::CrsrUD, 0xF7, 0x7F),
        // Column c4 (port_b = 0xEF)
        (Key::Single(' '), 0xEF, 0xFE),
        (Key::Single('Z'), 0xEF, 0xFD),
        (Key::Single('C'), 0xEF, 0xFB),
        (Key::Single('B'), 0xEF, 0xF7),
        (Key::Single('M'), 0xEF, 0xEF),
        (Key::Single('.'), 0xEF, 0xDF),
        (Key::Shift, 0xEF, 0xBF),
        (Key::F1F2, 0xEF, 0x7F),
        // Column c5 (port_b = 0xDF)
        (Key::Cbm, 0xDF, 0xFE),
        (Key::Single('S'), 0xDF, 0xFD),
        (Key::Single('F'), 0xDF, 0xFB),
        (Key::Single('H'), 0xDF, 0xF7),
        (Key::Single('K'), 0xDF, 0xEF),
        (Key::Single(':'), 0xDF, 0xDF),
        (Key::Single('='), 0xDF, 0xBF),
        (Key::F3F4, 0xDF, 0x7F),
        // Column c6 (port_b = 0xBF)
        (Key::Single('Q'), 0xBF, 0xFE),
        (Key::Single('E'), 0xBF, 0xFD),
        (Key::Single('T'), 0xBF, 0xFB),
        (Key::Single('U'), 0xBF, 0xF7),
        (Key::Single('O'), 0xBF, 0xEF),
        (Key::Single('@'), 0xBF, 0xDF),
        (Key::Up, 0xBF, 0xBF),
        (Key::F5F6, 0xBF, 0x7F),
        // Column c7 (port_b = 0x7F)
        (Key::Single('2'), 0x7F, 0xFE),
        (Key::Single('4'), 0x7F, 0xFD),
        (Key::Single('6'), 0x7F, 0xFB),
        (Key::Single('8'), 0x7F, 0xF7),
        (Key::Single('0'), 0x7F, 0xEF),
        (Key::Single('-'), 0x7F, 0xDF),
        (Key::ClrHome, 0x7F, 0xBF),
        (Key::F7F8, 0x7F, 0x7F),
    ];
    for (key, port_b, value) in base_mappings {
        map.insert((key, port_b), value);
        map.insert((key, 0x00), value);
    }
    map
}

impl Keyboard {
    pub fn new(receiver: Receiver<HashSet<Key>>) -> Self {
        Self {
            cache: HashSet::new(),
            receiver,
            keyboard_map: keyboard_map(),
        }
    }

    // 0x9120: column drive (port b, input)
    // 0x9121: row (port a, output)
    #[must_use]
    pub fn step(&mut self, port_b: u8) -> Option<u8> {
        if let Ok(keys) = self.receiver.try_recv() {
            self.cache = keys;
        }
        if !self.cache.is_empty() {
            let result = self
                .cache
                .iter()
                .filter_map(|&k| self.keyboard_map.get(&(k, port_b)).copied())
                .fold(0xFFu8, |acc, val| acc & val);
            if result == 0xFF { None } else { Some(result) }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::collections::HashSet;

    #[fixture]
    fn keyboard() -> Keyboard {
        let (_tx, rx) = make_keyboard_channel();
        Keyboard::new(rx)
    }

    fn keyboard_with_keys(keys: HashSet<Key>) -> Keyboard {
        let (tx, rx) = make_keyboard_channel();
        tx.send(keys).unwrap();
        Keyboard::new(rx)
    }

    #[rstest]
    fn step_returns_none_when_no_key_pressed(mut keyboard: Keyboard) {
        assert_eq!(keyboard.step(0x00), None);
    }

    #[rstest]
    fn step_returns_some_when_key_1_pressed() {
        let mut keyboard = keyboard_with_keys(HashSet::from([Key::Single('1')]));
        assert_eq!(keyboard.step(0xFE), Some(0xFE));
    }

    #[rstest]
    fn step_returns_some_when_return_pressed() {
        let mut keyboard = keyboard_with_keys(HashSet::from([Key::Return]));
        assert_eq!(keyboard.step(0xFD), Some(0x7F));
    }

    #[rstest]
    fn step_returns_none_for_wrong_column() {
        let mut keyboard = keyboard_with_keys(HashSet::from([Key::Single('1')]));
        assert_eq!(keyboard.step(0xFD), None);
    }

    #[rstest]
    fn step_combines_two_keys_same_column_different_rows() {
        let mut keyboard = keyboard_with_keys(HashSet::from([Key::Single('1'), Key::Single('3')]));
        assert_eq!(keyboard.step(0xFE), Some(0xFC));
    }

    #[rstest]
    fn step_returns_key_in_driven_column_with_two_keys_different_columns() {
        let mut keyboard = keyboard_with_keys(HashSet::from([Key::Single('1'), Key::Single('2')]));
        assert_eq!(keyboard.step(0xFE), Some(0xFE));
        assert_eq!(keyboard.step(0x7F), Some(0xFE));
    }
}
