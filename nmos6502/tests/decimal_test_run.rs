//! Assemble and run the Klaus Dormann decimal test against the emulated 6502.
//!
//! The test is assembled, loaded into RAM at address 0, and run until the CPU
//! reaches DONE on an instruction boundary (all pending micro-ops complete).
//! ERROR ($0B) is then checked for 0 (pass) or non-zero (fail).

use nmos6502::{assembler::assemble, memory::Ram, registers::ZERO, Addressable, CPU6502};

#[test]
fn trace_decimal_test_failure() {
    let raw = include_str!("../external/Klaus2m5/6502_decimal_test.a65");

    let (bytes, syms) = assemble(raw, 0, None).expect("decimal test assembly failed");

    let test_addr = *syms.get("test").expect("symbol table must contain 'test'");
    let done_addr = *syms.get("done").expect("symbol table must contain 'done'");
    let error_addr = 0x0B;

    // Find BNE DONE instructions
    let mut fail_addrs = Vec::new();
    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] == 0xD0 {
            let offset = bytes[i + 1] as i8;
            let target = (i as u16 + 2).wrapping_add(offset as u16);
            if target == done_addr {
                fail_addrs.push(i as u16);
            }
        }
    }

    eprintln!(
        "test=${test_addr:04X} done=${done_addr:04X} BNE_DONE_at=[{}]  bytes={}",
        fail_addrs
            .iter()
            .map(|a| format!("${a:04X}"))
            .collect::<Vec<_>>()
            .join(", "),
        bytes.len()
    );

    let mut cpu = CPU6502::new();
    let mut mem = Ram::new();
    for (i, &b) in bytes.iter().enumerate() {
        mem.write_byte(i as u16, b);
    }
    cpu.registers.pc = test_addr;

    let max_cycles: u64 = 100_000_000;
    let mut cycles: u64 = 0;
    let mut fail_info = String::new();

    while !(cpu.registers.pc == done_addr && cpu.is_at_instruction_boundary()) {
        if fail_addrs.contains(&cpu.registers.pc) && !cpu.registers.is_flag_set(ZERO) {
            let n1 = mem.read_byte(0x00);
            let n2 = mem.read_byte(0x01);
            let da = mem.read_byte(0x04);
            let ar = mem.read_byte(0x06);
            let dnvzc = mem.read_byte(0x05);
            let cf = mem.read_byte(0x0A);
            let is_sbc = fail_addrs.len() > 1 && cpu.registers.pc == fail_addrs[1];
            fail_info = format!(
                "FAIL at cycle {cycles} ${:04X}: {} N1=${:02X} N2=${:02X} Y=${:02X} \
                 DA=${:02X} AR=${:02X} DNVZC=${:02X} CF=${:02X}",
                cpu.registers.pc,
                if is_sbc { "SBC" } else { "ADC" },
                n1,
                n2,
                cpu.registers.y,
                da,
                ar,
                dnvzc,
                cf,
            );
        }
        cpu.cycle(&mut mem);
        cycles += 1;
        if cycles >= max_cycles {
            panic!("did not reach DONE after {max_cycles} cycles");
        }
    }

    let error = mem.read_byte(error_addr);
    if error != 0 {
        panic!(
            "{}\nERROR = {error} (expected 0)",
            if fail_info.is_empty() {
                "no comparison mismatch captured (BNE never taken)".to_string()
            } else {
                fail_info
            }
        );
    }

    eprintln!(
        "decimal test PASSED in {cycles} cycles (~{:.0} Mcycle/s)",
        cycles as f64 / 1_000_000.0
    );
}
