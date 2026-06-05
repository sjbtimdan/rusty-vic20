mod common;

use common::splash_screen_lines;
use rusty_vic20::addressable::Addressable;

const JIFFY_MSB: u16 = 0x00A0;
const JIFFY_MID: u16 = 0x00A1;
const JIFFY_LSB: u16 = 0x00A2;

fn read_jiffy(bus: &rusty_vic20::bus::Bus) -> u32 {
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
fn splash_screen_shows_on_startup() {
    let runner = common::run_boot();
    common::assert_screen_lines(&runner.bus, &splash_screen_lines());
}
