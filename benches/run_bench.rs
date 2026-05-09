#![feature(test)]

extern crate test;

use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
};
use test::Bencher;

fn run_steps(steps: usize) -> (Bus, CPU6502) {
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;
    bus.load_standard_roms_from_data_dir();
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);

    for _ in 0..steps {
        bus.step_devices(&mut cpu);
        cpu.step(&mut bus, &instruction_executor);
    }
    (bus, cpu)
}

#[bench]
fn bench_emulator_run_1m_steps(b: &mut Bencher) {
    b.iter(|| {
        run_steps(1_000_000);
    });
}
