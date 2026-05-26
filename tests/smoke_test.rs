mod common;

use common::screen_code;
use rusty_vic20::addressable::Addressable;

const JIFFY_MSB: u16 = 0x00A0;
const JIFFY_MID: u16 = 0x00A1;
const JIFFY_LSB: u16 = 0x00A2;

fn read_jiffy(bus: &rusty_vic20::bus::Bus) -> u32 {
    (bus.read_byte(JIFFY_MSB) as u32) << 16 | (bus.read_byte(JIFFY_MID) as u32) << 8 | bus.read_byte(JIFFY_LSB) as u32
}

#[test]
fn jiffy_clock_rate_is_correct() {
    let (mut bus, mut cpu) = common::run_boot();

    let latch_lo = bus.read_byte(0x9126) as u16;
    let latch_hi = bus.read_byte(0x9127) as u16;
    let latch = (latch_hi << 8) | latch_lo;
    let period_cycles = latch as u64 + 1;

    let jiffy_start = read_jiffy(&bus);

    let steps: u64 = 1_000_000;
    common::run_extra_steps(&mut bus, &mut cpu, steps as usize);

    let jiffy_end = read_jiffy(&bus);
    let actual_delta = jiffy_end - jiffy_start;
    let expected = (steps / period_cycles) as u32;

    eprintln!("VIA2 Timer1 latch = ${:04X}, period = {} cycles", latch, period_cycles);
    eprintln!(
        "Jiffy start = {}, end = {}, delta = {}",
        jiffy_start, jiffy_end, actual_delta
    );
    eprintln!("Expected ≈ {} jiffies in {} steps", expected, steps);

    let via_ier = bus.read_byte(0x912E);
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
    let (bus, _cpu) = common::run_boot();
    common::assert_screen_lines(&bus, &splash_screen_lines());
}

fn splash_screen_lines() -> Vec<[u8; 22]> {
    vec![
        screen_code("**** CBM BASIC V2 ****"),
        screen_code("                      "),
        screen_code("3583 BYTES FREE       "),
        screen_code("                      "),
        screen_code("READY.                "),
    ]
}
