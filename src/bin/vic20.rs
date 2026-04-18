use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, interrupt_handler::DefaultInterruptHandler},
};
use std::env;
use std::thread;
use std::time::Duration;

const DEFAULT_TICK_MICROS: u64 = 1;

fn parse_tick_duration() -> Duration {
    let tick_micros = env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TICK_MICROS);

    Duration::from_micros(tick_micros)
}

fn main() {
    env_logger::init();
    let tick_duration = parse_tick_duration();
    println!(
        "Using tick duration of {:?} ({} microseconds)",
        tick_duration,
        tick_duration.as_micros()
    );
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let interrupt_handler = DefaultInterruptHandler;
    bus.load_standard_roms_from_data_dir();
    bus.add_watchpoint_at(0x0288);
    bus.add_watchpoint_at(0x0289);
    cpu.add_breakpoint_address(0xFDD2);
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);
    loop {
        cpu.step(&mut bus, &interrupt_handler);
        bus.step_devices();
        if !tick_duration.is_zero() {
            thread::sleep(tick_duration);
        }
    }
}
