//! Assemble and run the full Klaus Dormann 6502 functional test suite.
//!
//! The test is assembled, loaded into RAM at address 0, and run until the
//! CPU stalls at a `jmp *` (both `trap` and `success` use `jmp *` with
//! `report = 0`).  Before the `success` trap, the test stores `$F0` into
//! `test_case` ($200).  We pass if that byte is `$F0` after the stall.

use nmos6502::Addressable;

mod common;

/// `test_case` is the first byte after `org data_segment` ($200).
const TEST_CASE_ADDR: u16 = 0x0200;

const MAX_CYCLES: u64 = 2_000_000_000;

#[test]
fn run_functional_test() {
    let source = include_str!("../external/Klaus2m5/6502_functional_test.a65");
    let (bytes, syms) = common::assemble_program(source, 0);
    let start_addr = *syms.get("start").expect("symbol 'start' not found");

    let (mut cpu, mut mem) = common::load_program(&bytes, start_addr);

    let mut last_boundary_pc = 0u16;
    let mut stall_count = 0u8;

    let cycles = common::run_until(
        &mut cpu,
        &mut mem,
        |cpu, _mem| {
            if cpu.is_at_instruction_boundary() {
                let pc = cpu.registers.pc;
                if pc == last_boundary_pc {
                    stall_count += 1;
                    if stall_count >= 3 {
                        return true; // stalled at `jmp *`
                    }
                } else {
                    stall_count = 0;
                    last_boundary_pc = pc;
                }
            }
            false
        },
        MAX_CYCLES,
    );

    let test_case = mem.read_byte(TEST_CASE_ADDR);
    let last_pc = last_boundary_pc;
    if test_case != 0xF0 {
        eprintln!("stalled at PC=${last_pc:04X} test_case=${test_case:02X}");
    }
    assert_eq!(
        test_case, 0xF0,
        "FAILED: stalled at ${last_pc:04X} test_case=${test_case:02X} after {cycles} cycles \
         (expected $F0 = all opcode tests passed)",
    );

    eprintln!(
        "PASSED in {cycles} cycles (~{:.0} Mcycle/s)",
        cycles as f64 / 1_000_000.0
    );
}
