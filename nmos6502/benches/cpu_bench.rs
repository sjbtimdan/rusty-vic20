#![feature(test)]
extern crate test;

use nmos6502::{Addressable, CPU6502};
use test::Bencher;

struct Ram([u8; 65536]);
impl Ram {
    fn new() -> Self {
        Self([0; 65536])
    }
}
impl Addressable for Ram {
    fn read_byte(&mut self, addr: u16) -> u8 {
        self.0[addr as usize]
    }
    fn write_byte(&mut self, addr: u16, v: u8) {
        self.0[addr as usize] = v;
    }
}

fn bench_steps(steps: u64, b: &mut Bencher) {
    let mut cpu = CPU6502::new();
    let mut mem = Ram::new();
    mem.write_byte(0x0200, 0xE8); // INX
    mem.write_byte(0x0201, 0xD0); // BNE
    mem.write_byte(0x0202, 0xFC); // -4 → $0200
    cpu.registers.pc = 0x0200;

    b.iter(|| {
        for _ in 0..steps {
            cpu.cycle(&mut mem);
        }
    });
}

#[bench]
fn bench_1m_cycles(b: &mut Bencher) {
    bench_steps(1_000_000, b);
}
#[bench]
fn bench_10m_cycles(b: &mut Bencher) {
    bench_steps(10_000_000, b);
}
