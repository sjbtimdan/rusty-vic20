#![allow(dead_code)]

use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
};

pub const SCREEN_RAM_START: u16 = 0x1E00;
pub const SCREEN_LINE_LEN: u16 = 22;

pub fn read_screen_line(bus: &Bus, addr: u16) -> [u8; 22] {
    let mut line = [0u8; 22];
    for (i, b) in line.iter_mut().enumerate() {
        *b = bus.read_byte(addr + i as u16);
    }
    line
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

fn screen_line_to_string(line: &[u8]) -> String {
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
        let actual = read_screen_line(bus, SCREEN_RAM_START + i as u16 * SCREEN_LINE_LEN);
        if actual != *expected_line {
            panic!(
                "Line {} does not match:\n  expected: \"{}\"\n  got:      \"{}\"",
                i + 1,
                screen_line_to_string(expected_line),
                screen_line_to_string(&actual),
            );
        }
    }
}

pub fn count_screen_chars(bus: &Bus, screen_code: u8) -> usize {
    let screen_start = SCREEN_RAM_START;
    let screen_size = 23 * 22;
    (0..screen_size)
        .filter(|&offset| bus.read_byte(screen_start + offset as u16) == screen_code)
        .count()
}

pub fn run_boot() -> (Bus, CPU6502) {
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;
    bus.load_standard_roms_from_data_dir();
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);

    for _ in 0..600_000 {
        bus.step_devices(&mut cpu);
        cpu.step(&mut bus, &instruction_executor);
    }
    (bus, cpu)
}

pub fn run_extra_steps(bus: &mut Bus, cpu: &mut CPU6502, steps: usize) {
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;
    for _ in 0..steps {
        bus.step_devices(cpu);
        cpu.step(bus, &instruction_executor);
    }
}
