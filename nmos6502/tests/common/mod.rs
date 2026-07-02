//! Shared helpers for Klaus Dormann test runners.
//!
//! Provides `assemble_program`, `load_program`, and `run_until` so
//! integration tests don't duplicate the boilerplate.

use nmos6502::{memory::Ram, tools::assembler::assemble, Addressable, CPU6502};
use std::collections::HashMap;

/// Assemble a test program, returning `(bytes, symbol_table)`.
pub fn assemble_program(source: &str, origin: u16) -> (Vec<u8>, HashMap<String, u16>) {
    assemble(source, origin, None).expect("assembly failed")
}

/// Load assembled bytes (from `assemble_program` with origin 0) into RAM and
/// set PC to `entry_point`.  Bytes are placed at their absolute addresses
/// (matching the assembly origin — currently always 0 for these helpers).
#[allow(dead_code)]
pub fn load_program(bytes: &[u8], entry_point: u16) -> (CPU6502, Ram) {
    let mut cpu = CPU6502::default();
    let mut mem = Ram::new();
    for (i, &b) in bytes.iter().enumerate() {
        mem.write_byte(i as u16, b);
    }
    cpu.registers.pc = entry_point;
    (cpu, mem)
}

#[allow(dead_code)]
pub fn run_until<F>(cpu: &mut CPU6502, mem: &mut Ram, mut stop: F, max_cycles: u64) -> u64
where
    F: FnMut(&CPU6502, &Ram) -> bool,
{
    let mut cycles: u64 = 0;
    loop {
        cpu.cycle(mem);
        cycles += 1;
        if stop(cpu, mem) {
            return cycles;
        }
        if cycles >= max_cycles {
            panic!("did not meet stop condition after {max_cycles} cycles");
        }
    }
}
