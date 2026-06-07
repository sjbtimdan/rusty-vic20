mod common;

use common::screen_code;
use rusty_vic20::{addressable::Addressable, ui::keyboard::key::Key};
use std::collections::HashSet;

fn run_boot_with_keyboard() -> rusty_vic20::runner::EmulatorRunner {
    let mut runner = common::run_boot();
    for _ in 0..100_000 {
        runner.step_keyboard();
        runner.step();
    }
    runner
}

#[test]
fn via1_ca1_enabled_after_boot() {
    let runner = common::run_boot();
    let via1_ier = runner.bus.read_byte(0x911E);
    assert!(
        via1_ier & 0x02 != 0,
        "KERNAL should enable VIA1 CA1 interrupts, got IER={:02X}",
        via1_ier
    );
}

#[test]
fn held_key_repeats_in_kernal() {
    let mut runner = run_boot_with_keyboard();

    common::assert_screen_lines(
        &runner.bus,
        &[
            screen_code("**** CBM BASIC V2 ****"),
            screen_code("                      "),
            screen_code("3583 BYTES FREE       "),
            screen_code("                      "),
            screen_code("READY.                "),
        ],
    );

    runner.keyboard_sender.send(HashSet::from([Key::Single('A')])).ok();

    for _ in 0..500_000 {
        runner.step_keyboard();
        runner.step();
    }

    let screen_a_count = common::count_screen_chars(&runner.bus, 0x01);
    eprintln!("Found {} 'A' characters on screen", screen_a_count);
    assert!(
        screen_a_count > 1,
        "Expected multiple 'A's from key repeat, but found only {}",
        screen_a_count,
    );
}
