mod common;

use common::{UNEXPANDED_SCREEN_RAM_START, screen_code};
use rusty_vic20::{
    emulator::{ThreadReceivers, paste::new_paste_queue},
    hardware::{addressable::Addressable, memory::MemoryExpansion},
    peripherals::{brake::make_brake_channel, keyboard::make_keyboard_channel},
    ui::{
        audio::AudioProducer,
        control::SharedPerformanceMetrics,
        keyboard::key::Key,
        screen::display::SharedVideoState,
    },
};
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex, mpsc::SyncSender},
};

fn run_boot_with_keyboard() -> (rusty_vic20::emulator::EmulatorRunner, SyncSender<HashSet<Key>>) {
    let (keyboard_sender, keyboard_receiver) = make_keyboard_channel();
    let receivers = ThreadReceivers {
        video: Arc::new(Mutex::new(SharedVideoState::default())),
        perf: Arc::new(Mutex::new(SharedPerformanceMetrics::default())),
        load_queue: Arc::new(Mutex::new(VecDeque::new())),
        paste_queue: new_paste_queue(),
        keyboard_receiver,
        cassette_receiver: std::sync::mpsc::channel().1,
        joystick_receiver: std::sync::mpsc::channel().1,
        direct_loader_receiver: std::sync::mpsc::channel().1,
        brake_receiver: make_brake_channel().1,
        shutdown_receiver: std::sync::mpsc::channel().1,
    };
    let mut runner =
        rusty_vic20::emulator::EmulatorRunner::new(receivers, MemoryExpansion::None, AudioProducer::noop());
    runner.step_multiple(600_000);
    for _ in 0..100_000 {
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
