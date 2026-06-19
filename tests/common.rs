#![allow(dead_code)]

use rusty_vic20::{
    emulator::{EmulatorRunner, ThreadReceivers, paste::new_paste_queue},
    hardware::{addressable::Addressable, bus::Bus, memory::MemoryExpansion},
    peripherals::{brake::make_brake_channel, keyboard::make_keyboard_channel},
    ui::{audio::AudioProducer, control::SharedPerformanceMetrics, screen::display::SharedVideoState},
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub const UNEXPANDED_SCREEN_RAM_START: u16 = 0x1E00;
pub const SCREEN_LINE_LEN: u16 = 22;

pub fn read_screen_line(bus: &Bus, addr: u16) -> [u8; 22] {
    let mut line = [0u8; 22];
    for (i, b) in line.iter_mut().enumerate() {
        *b = bus.read_byte(addr + i as u16);
    }
    line
}

pub fn assert_screen_line(bus: &Bus, screen_ram_start: u16, row: usize, expected_line: &str) {
    let actual_bytes = read_screen_line(bus, screen_ram_start + row as u16 * SCREEN_LINE_LEN);
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
            0x26 => '&',
            0x2A => '*',
            0x2E => '.',
            0x30..=0x39 => b as char,
            _ => '?',
        })
        .collect()
}

pub fn assert_screen_lines(bus: &Bus, screen_ram_start: u16, expected: &[[u8; 22]]) {
    for (i, expected_line) in expected.iter().enumerate() {
        assert_screen_line(bus, screen_ram_start, i, &screen_line_to_string(expected_line));
    }
}

pub fn count_screen_chars(bus: &Bus, screen_code: u8) -> usize {
    let screen_start = UNEXPANDED_SCREEN_RAM_START;
    let screen_size = 23 * 22;
    (0..screen_size)
        .filter(|&offset| bus.read_byte(screen_start + offset as u16) == screen_code)
        .count()
}

pub fn run_boot() -> EmulatorRunner {
    let mut runner = EmulatorRunner::new(test_receivers(), MemoryExpansion::None, AudioProducer::noop());
    runner.step_multiple(600_000);
    runner
}

pub fn run_boot_with_expansion(expansion: MemoryExpansion) -> EmulatorRunner {
    let mut runner = EmulatorRunner::new(test_receivers(), expansion, AudioProducer::noop());
    let steps = match expansion {
        MemoryExpansion::EightK | MemoryExpansion::SixteenK | MemoryExpansion::ThirtyTwoK => 2_000_000,
        _ => 600_000,
    };
    runner.step_multiple(steps);
    runner
}

fn test_receivers() -> ThreadReceivers {
    ThreadReceivers {
        video: Arc::new(Mutex::new(SharedVideoState::default())),
        perf: Arc::new(Mutex::new(SharedPerformanceMetrics::default())),
        load_queue: Arc::new(Mutex::new(VecDeque::new())),
        paste_queue: new_paste_queue(),
        keyboard_receiver: make_keyboard_channel().1,
        cassette_receiver: std::sync::mpsc::channel().1,
        joystick_receiver: std::sync::mpsc::channel().1,
        direct_loader_receiver: std::sync::mpsc::channel().1,
        brake_receiver: make_brake_channel().1,
        shutdown_receiver: std::sync::mpsc::channel().1,
    }
}

pub fn run_extra_steps(runner: &mut EmulatorRunner, steps: usize) {
    runner.step_multiple(steps);
}

pub fn splash_screen_lines() -> Vec<[u8; 22]> {
    splash_screen_lines_with(3583)
}

pub fn splash_screen_lines_with(bytes_free: u16) -> Vec<[u8; 22]> {
    let bytes_line = format!("{bytes_free} BYTES FREE");
    let padded = format!("{bytes_line:<22}");
    vec![
        screen_code("**** CBM BASIC V2 ****"),
        screen_code("                      "),
        screen_code(&padded),
        screen_code("                      "),
        screen_code("READY.                "),
    ]
}
