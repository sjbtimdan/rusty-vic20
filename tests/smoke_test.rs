mod common;

use common::splash_screen_lines;
use rusty_vic20::hardware::{addressable::Addressable, memory::MemoryExpansion};

const JIFFY_MSB: u16 = 0x00A0;
const JIFFY_MID: u16 = 0x00A1;
const JIFFY_LSB: u16 = 0x00A2;

fn read_jiffy(bus: &rusty_vic20::hardware::bus::Bus) -> u32 {
    (bus.read_byte(JIFFY_MSB) as u32) << 16 | (bus.read_byte(JIFFY_MID) as u32) << 8 | bus.read_byte(JIFFY_LSB) as u32
}

#[test]
fn jiffy_clock_rate_is_correct() {
    let mut runner = common::run_boot();

    let latch = runner.bus.read_word(0x9126);
    let period_cycles = latch as u64 + 1;

    let jiffy_start = read_jiffy(&runner.bus);

    let steps: u64 = 1_000_000;
    common::run_extra_steps(&mut runner, steps as usize);

    let jiffy_end = read_jiffy(&runner.bus);
    let actual_delta = jiffy_end - jiffy_start;
    let expected = (steps / period_cycles) as u32;

    eprintln!("VIA2 Timer1 latch = ${:04X}, period = {} cycles", latch, period_cycles);
    eprintln!(
        "Jiffy start = {}, end = {}, delta = {}",
        jiffy_start, jiffy_end, actual_delta
    );
    eprintln!("Expected ≈ {} jiffies in {} steps", expected, steps);

    let via_ier = runner.bus.read_byte(0x912E);
    eprintln!("VIA2 IER = {:02X}", via_ier);

    assert!(
        via_ier & 0x40 != 0,
        "KERNAL should enable Timer 1 interrupts (bit 6 of IER)"
    );

    let tolerance = (expected / 20).max(1);
    assert!(
        actual_delta >= expected - tolerance && actual_delta <= expected + tolerance,
        "Jiffy rate wrong: got {} ticks in {} cycles, expected ~{} (period={} cycles)",
        actual_delta,
        steps,
        expected,
        period_cycles,
    );
}

#[test]
fn splash_screen_shows_3583_bytes_on_startup() {
    let runner = common::run_boot();
    common::assert_screen_lines(&runner.bus, 0x1E00, &splash_screen_lines());
}

#[test]
fn splash_screen_shows_6655_bytes_with_3k_expansion() {
    let runner = common::run_boot_with_expansion(MemoryExpansion::ThreeK);
    let expected = common::splash_screen_lines_with(6655);
    common::assert_screen_lines(&runner.bus, 0x1E00, &expected);
}

#[test]
fn debug_8k_expansion() {
    let mut runner8k = common::run_boot_with_expansion(MemoryExpansion::EightK);
    runner8k.step_multiple(800_000);
    let runner_none = common::run_boot();
    let runner3k = common::run_boot_with_expansion(MemoryExpansion::ThreeK);

    for (label, runner) in [("None", &runner_none), ("ThreeK", &runner3k), ("EightK", &runner8k)] {
        eprintln!(
            "{}: PC=${:04X} SP={:02X} VIC9002={:02X} VIC9005={:02X} topmem=${:04X} basic=${:04X} screen_start=${:04X}",
            label,
            runner.cpu.registers.pc,
            runner.cpu.registers.sp,
            runner.bus.read_byte(0x9002),
            runner.bus.read_byte(0x9005),
            runner.bus.read_word(0x0283),
            runner.bus.read_word(0x002B),
            runner.bus.screen_ram_start(),
        );
    }
}

#[test]
fn splash_screen_shows_11775_bytes_with_8k_expansion() {
    let runner = common::run_boot_with_expansion(MemoryExpansion::EightK);
    let screen_start = runner.bus.screen_ram_start();
    assert_ne!(
        screen_start, 0x1E00,
        "8K expansion should relocate screen away from 0x1E00"
    );
    let expected = common::splash_screen_lines_with(11775);
    common::assert_screen_lines(&runner.bus, screen_start, &expected);
}
