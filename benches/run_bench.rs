#![feature(test)]

extern crate test;

use nmos6502::CPU6502;
use rusty_vic20::hardware::bus::Bus;
use test::Bencher;

fn run_steps(steps: usize) -> (Bus, CPU6502) {
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    cpu.reset(&mut bus);

    for _ in 0..steps {
        bus.step_devices(&mut cpu);
        cpu.cycle(&mut bus);
    }
    (bus, cpu)
}

#[bench]
fn bench_emulator_run_1m_steps(b: &mut Bencher) {
    b.iter(|| {
        run_steps(1_000_000);
    });
}
