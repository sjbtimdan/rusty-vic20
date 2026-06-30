//! Shared helpers for Klaus Dormann test runners.
//!
//! Provides `assemble_program`, `load_program`, and `run_until` so
//! integration tests don't duplicate the boilerplate.

use std::collections::HashMap;
use nmos6502::{assembler::assemble, memory::Ram, Addressable, CPU6502};

/// Assemble a test program, returning `(bytes, symbol_table)`.
pub fn assemble_program(source: &str, origin: u16) -> (Vec<u8>, HashMap<String, u16>) {
    assemble(source, origin, None).expect("assembly failed")
}

/// Load bytes into RAM and initialise the CPU at the given start address.
pub fn load_program(bytes: &[u8], start_addr: u16) -> (CPU6502, Ram) {
    let mut cpu = CPU6502::new();
    let mut mem = Ram::new();
    for (i, &b) in bytes.iter().enumerate() {
        mem.write_byte(i as u16, b);
    }
    cpu.registers.pc = start_addr;
    (cpu, mem)
}

/// Run the CPU until `stop` returns `true`.
///
/// `stop` is called AFTER every call to `cpu.cycle()`.  The closure
/// receives immutable references to both the CPU and memory so it can
/// inspect registers or peek at RAM.
///
/// # Panics
///
/// Panics if the `max_cycles` budget is exhausted before `stop` fires.
pub fn run_until<F>(
    cpu: &mut CPU6502,
    mem: &mut Ram,
    mut stop: F,
    max_cycles: u64,
) -> u64
where
    F: FnMut(&CPU6502, &Ram) -> bool,
{
    let mut cycles: u64 = 0;
    loop {
        if stop(cpu, mem) {
            return cycles;
        }
        cpu.cycle(mem);
        cycles += 1;
        if cycles >= max_cycles {
            panic!("did not meet stop condition after {max_cycles} cycles");
        }
    }
}
