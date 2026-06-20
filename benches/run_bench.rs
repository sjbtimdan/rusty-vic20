#![feature(test)]

extern crate test;

use rusty_vic20::{
    cpu::cpu6502::CPU6502,
    hardware::{addressable::Addressable, bus::Bus},
};
use test::Bencher;

fn run_steps(steps: usize) -> (Bus, CPU6502) {
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    bus.memory.load_standard_roms_from_data_dir();
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);

    for _ in 0..steps {
        bus.step_devices(&mut cpu);
        cpu.step(&mut bus);
    }
    (bus, cpu)
}

#[bench]
fn bench_emulator_run_1m_steps(b: &mut Bencher) {
    b.iter(|| {
        run_steps(1_000_000);
    });
}
