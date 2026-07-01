//! Assemble and run the Klaus Dormann 6502 interrupt test suite.
//!
//! The test exercises IRQ, NMI, and BRK using a feedback register at `$BFFC`.
//! Writes to that port drive the CPU's IRQ (bit 0) and NMI (bit 1) lines.
//! The test halts at `jmp *` on both success and failure; we distinguish
//! them by checking the `I_src` byte at `$203` — zero means all expected
//! interrupts were received.

use nmos6502::{memory::Ram, Addressable, CPU6502};

mod common;

// Configuration matching the test's own defines
const I_PORT: u16 = 0xBFFC;
const IRQ_BIT: u8 = 0;
const NMI_BIT: u8 = 1;

// Data segment and I_src location
const DATA_SEGMENT: u16 = 0x0200;
const I_SRC_ADDR: u16 = DATA_SEGMENT + 3; // I_src at data_segment + 3

const MAX_CYCLES: u64 = 200_000_000;

/// Bus that captures writes to the feedback port and returns its value on read.
struct InterruptBus {
    ram: Ram,
    feedback: u8,
}

impl Addressable for InterruptBus {
    fn read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            I_PORT => self.feedback,
            _ => self.ram.read_byte(addr),
        }
    }
    fn write_byte(&mut self, addr: u16, val: u8) {
        if addr == I_PORT {
            self.feedback = val & 0x7f; // I_filter = $7f
        }
        self.ram.write_byte(addr, val);
    }
}

#[test]
fn run_interrupt_test() {
    let source = include_str!("../external/Klaus2m5/6502_interrupt_test.a65");
    let (bytes, syms) = common::assemble_program(source, 0);
    let start_addr = *syms.get("start").expect("symbol 'start' not found");

    let mut cpu = CPU6502::new();
    let mut bus = InterruptBus {
        ram: Ram::new(),
        feedback: 0,
    };

    // Load program bytes into RAM
    for (i, &b) in bytes.iter().enumerate() {
        bus.ram.write_byte(i as u16, b);
    }
    cpu.registers.pc = start_addr;

    let mut last_boundary_pc = 0u16;
    let mut stall_count = 0u8;
    let mut cycles = 0u64;

    loop {
        // Map feedback register bits to CPU interrupt lines.
        // Open-collector convention: bit=1 → line LOW (asserted).
        cpu.irq_line_low = (bus.feedback & (1 << IRQ_BIT)) != 0;

        // NMI is falling-edge triggered: bit=1 → LOW, bit=0 → HIGH.
        cpu.nmi_latch.set_level((bus.feedback & (1 << NMI_BIT)) == 0);

        cpu.cycle(&mut bus);
        cycles += 1;

        // Stall detection (3 consecutive same PC at instruction boundary)
        if cpu.is_at_instruction_boundary() {
            let pc = cpu.registers.pc;
            if pc == last_boundary_pc {
                stall_count += 1;
                if stall_count >= 3 {
                    break;
                }
            } else {
                stall_count = 0;
                last_boundary_pc = pc;
            }
        }

        if cycles >= MAX_CYCLES {
            panic!(
                "interrupt test did not complete after {MAX_CYCLES} cycles, \
                 last PC=${:04X} I_src=${:02X}",
                last_boundary_pc,
                bus.ram.read_byte(I_SRC_ADDR)
            );
        }
    }

    // On stall, check I_src — 0 means all expected interrupts received.
    let i_src = bus.ram.read_byte(I_SRC_ADDR);
    assert_eq!(
        i_src, 0,
        "interrupt test FAILED at PC=${last_boundary_pc:04X}: I_src=${i_src:02X} \
         after {cycles} cycles (expected $00 = all interrupts received)",
    );

    eprintln!(
        "interrupt test PASSED in {cycles} cycles (~{:.0} Mcycle/s)",
        cycles as f64 / 1_000_000.0
    );
}
