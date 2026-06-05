#![allow(dead_code)]

use rusty_vic20::{addressable::Addressable, bus::Bus, runner::EmulatorRunner};

pub const SCREEN_RAM_START: u16 = 0x1E00;
pub const SCREEN_LINE_LEN: u16 = 22;

pub fn read_screen_line(bus: &Bus, addr: u16) -> [u8; 22] {
    let mut line = [0u8; 22];
    for (i, b) in line.iter_mut().enumerate() {
        *b = bus.read_byte(addr + i as u16);
    }
    line
}

pub fn assert_screen_line(bus: &Bus, row: usize, expected_line: &str) {
    let actual_bytes = read_screen_line(bus, SCREEN_RAM_START + row as u16 * SCREEN_LINE_LEN);
    let actual = screen_line_to_string(&actual_bytes);
    if actual != expected_line {
        panic!(
            "Line {} does not match:\n  expected: \"{}\"\n  got:      \"{}\"",
            row + 1,
            actual,
            expected_line
        );
    }
}

pub fn screen_code(s: &str) -> [u8; 22] {
    let mut buf = [0x20u8; 22];
    for (i, ch) in s.chars().take(22).enumerate() {
        buf[i] = match ch {
            'A'..='Z' => ch as u8 - b'A' + 0x01,
            'a'..='z' => ch as u8 - b'a' + 0x01,
            '0'..='9' => ch as u8,
            '*' => 0x2A,
            ' ' => 0x20,
            '.' => 0x2E,
            _ => ch as u8,
        };
    }
    buf
}

pub fn screen_line_to_string(line: &[u8]) -> String {
    line.iter()
        .map(|&b| match b {
            0x00 => '@',
            0x01..=0x1A => (b - 0x01 + b'A') as char,
            0x20 => ' ',
            0x2A => '*',
            0x2E => '.',
            0x30..=0x39 => b as char,
            _ => '?',
        })
        .collect()
}

pub fn assert_screen_lines(bus: &Bus, expected: &[[u8; 22]]) {
    for (i, expected_line) in expected.iter().enumerate() {
        assert_screen_line(bus, i, &screen_line_to_string(expected_line));
    }
}

pub fn count_screen_chars(bus: &Bus, screen_code: u8) -> usize {
    let screen_start = SCREEN_RAM_START;
    let screen_size = 23 * 22;
    (0..screen_size)
        .filter(|&offset| bus.read_byte(screen_start + offset as u16) == screen_code)
        .count()
}

pub fn run_boot() -> EmulatorRunner {
    let mut runner = EmulatorRunner::default();
    runner.step_multiple(600_000);
    runner
}

pub fn run_extra_steps(runner: &mut EmulatorRunner, steps: usize) {
    runner.step_multiple(steps);
}

pub fn splash_screen_lines() -> Vec<[u8; 22]> {
    vec![
        screen_code("**** CBM BASIC V2 ****"),
        screen_code("                      "),
        screen_code("3583 BYTES FREE       "),
        screen_code("                      "),
        screen_code("READY.                "),
    ]
}
