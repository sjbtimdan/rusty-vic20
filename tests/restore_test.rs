mod common;

use common::{UNEXPANDED_SCREEN_RAM_START, screen_code};
use rusty_vic20::{addressable::Addressable, ui::keyboard::key::Key};
use std::{collections::HashSet, sync::mpsc::SyncSender};

fn run_boot_with_keyboard() -> (rusty_vic20::runner::EmulatorRunner, SyncSender<HashSet<Key>>) {
    let (keyboard_sender, keyboard_receiver) = rusty_vic20::peripherals::keyboard::make_keyboard_channel();
    let mut runner = rusty_vic20::runner::EmulatorRunner::from_receiver(
        keyboard_receiver,
        rusty_vic20::paste::new_paste_queue(),
        rusty_vic20::memory::MemoryExpansion::None,
        rusty_vic20::peripherals::brake::make_brake_channel().1,
        rusty_vic20::ui::audio::AudioProducer::noop(),
    );
    runner.step_multiple(600_000);
    for _ in 0..100_000 {
        runner.step_keyboard();
        runner.step();
    }
    (runner, keyboard_sender)
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
    let (mut runner, keyboard_sender) = run_boot_with_keyboard();

    common::assert_screen_lines(
        &runner.bus,
        UNEXPANDED_SCREEN_RAM_START,
        &[
            screen_code("**** CBM BASIC V2 ****"),
            screen_code("                      "),
            screen_code("3583 BYTES FREE       "),
            screen_code("                      "),
            screen_code("READY.                "),
        ],
    );

    keyboard_sender.send(HashSet::from([Key::Single('A')])).ok();

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
