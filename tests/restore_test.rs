mod common;

use common::screen_code;
use rusty_vic20::{
    addressable::Addressable,
    cpu::instruction_executor,
    keyboard::{Keyboard, make_keyboard_channel},
    ui::keyboard::key::Key,
};
use std::collections::HashSet;

fn run_boot_with_keyboard() -> (rusty_vic20::bus::Bus, rusty_vic20::cpu::cpu6502::CPU6502) {
    let (_tx, rx) = make_keyboard_channel();
    let mut keyboard = Keyboard::new(rx, None);
    let (mut bus, mut cpu) = common::run_boot();
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;

    for _ in 0..100_000 {
        keyboard.inject_paste_into_buffer(&mut bus);
        if let Some(port_a) = keyboard.step(bus.via2.port_b()) {
            bus.via2.set_port_a(port_a);
        } else {
            bus.via2.set_port_a(0xFF);
        }
        bus.via1.set_ca1_pin(false);
        bus.step_devices(&mut cpu);
        cpu.step(&mut bus, &instruction_executor);
    }
    (bus, cpu)
}

#[test]
fn via1_ca1_enabled_after_boot() {
    let (bus, _cpu) = common::run_boot();
    let via1_ier = bus.read_byte(0x911E);
    assert!(
        via1_ier & 0x02 != 0,
        "KERNAL should enable VIA1 CA1 interrupts, got IER={:02X}",
        via1_ier
    );
}

#[test]
fn restore_stop_triggers_warm_start() {
    let (mut bus, mut cpu) = run_boot_with_keyboard();
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;

    common::assert_screen_lines(
        &bus,
        &[
            screen_code("**** CBM BASIC V2 ****"),
            screen_code("                      "),
            screen_code("3583 BYTES FREE       "),
            screen_code("                      "),
            screen_code("READY.                "),
        ],
    );

    let (tx, rx) = make_keyboard_channel();
    tx.send(HashSet::from([Key::RunStop, Key::Restore])).ok();
    let mut keyboard = Keyboard::new(rx, None);

    for _ in 0..300_000 {
        keyboard.inject_paste_into_buffer(&mut bus);
        if let Some(port_a) = keyboard.step(bus.via2.port_b()) {
            bus.via2.set_port_a(port_a);
        } else {
            bus.via2.set_port_a(0xFF);
        }

        let restore_nmi = keyboard.is_restore_pressed() && (bus.via2.port_b() & 0x80 == 0);
        bus.via1.set_ca1_pin(restore_nmi);

        bus.step_devices(&mut cpu);
        if restore_nmi {
            cpu.nmi_latch.set_level(true);
        }
        cpu.step(&mut bus, &instruction_executor);
    }

    common::assert_screen_lines(
        &bus,
        &[
            screen_code("                      "),
            screen_code("READY.                "),
        ],
    );
}

#[test]
fn held_key_repeats_in_kernal() {
    let (mut bus, mut cpu) = run_boot_with_keyboard();
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;

    common::assert_screen_lines(
        &bus,
        &[
            screen_code("**** CBM BASIC V2 ****"),
            screen_code("                      "),
            screen_code("3583 BYTES FREE       "),
            screen_code("                      "),
            screen_code("READY.                "),
        ],
    );

    let (tx, rx) = make_keyboard_channel();
    tx.send(HashSet::from([Key::Single('A')])).ok();
    let mut keyboard = Keyboard::new(rx, None);

    for _ in 0..500_000 {
        keyboard.inject_paste_into_buffer(&mut bus);
        if let Some(port_a) = keyboard.step(bus.via2.port_b()) {
            bus.via2.set_port_a(port_a);
        } else {
            bus.via2.set_port_a(0xFF);
        }

        bus.via1.set_ca1_pin(false);
        bus.step_devices(&mut cpu);
        cpu.step(&mut bus, &instruction_executor);
    }

    let screen_a_count = common::count_screen_chars(&bus, 0x01);
    eprintln!("Found {} 'A' characters on screen", screen_a_count);
    assert!(
        screen_a_count > 1,
        "Expected multiple 'A's from key repeat, but found only {}",
        screen_a_count,
    );
}
