use nmos6502::{memory::Ram, Addressable, CPU6502};
use serde::Deserialize;

/// Bus cycle type: a read or write operation.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum CycleType {
    Read,
    Write,
}

/// Processor state snapshot (registers + relevant RAM).
#[derive(Debug, Deserialize)]
struct CpuState {
    pc: u16,
    s: u8,
    a: u8,
    x: u8,
    y: u8,
    /// Processor status flags (P register).
    p: u8,
    /// RAM contents as `[[address, value], ...]`.
    ram: Vec<(u16, u8)>,
}

/// A single Tom Harte 6502 JSON test case.
#[derive(Debug, Deserialize)]
struct TestCase {
    /// Ignorable name string.
    name: String,
    /// Initial processor/memory state.
    initial: CpuState,
    /// Expected final state after one instruction.
    #[serde(rename = "final")]
    end: CpuState,
    /// Expected bus operations per cycle: `[address, value, "read"|"write"]`.
    cycles: Vec<(u16, u8, CycleType)>,
}

/// Parse a Tom Harte 6502 JSON test file into a vector of test cases.
fn parse_harte_test_file(path: &str) -> Vec<TestCase> {
    let content = std::fs::read_to_string(path).expect("Failed to read test file");
    let cases: Vec<_> = serde_json::from_str(&content).expect("Failed to parse JSON test data");
    assert!(!cases.is_empty(), "Test file must contain at least one test case");
    cases
}

/// Run a single Harte test case against the nmos6502 CPU.
///
/// Returns a list of mismatch descriptions. Empty `Vec` means the case passed.
fn run_harte_test(case_index: usize, case: &TestCase) -> Vec<String> {
    let mut errors = Vec::new();
    let label = format!("#{case_index} [{}]", case.name);

    let mut cpu = CPU6502::new();
    let mut mem = Ram::new();

    for &(addr, val) in &case.initial.ram {
        mem.write_byte(addr, val);
    }

    cpu.registers.pc = case.initial.pc;
    cpu.registers.sp = case.initial.s;
    cpu.registers.a = case.initial.a;
    cpu.registers.x = case.initial.x;
    cpu.registers.y = case.initial.y;
    cpu.registers.status = case.initial.p;

    for _ in 0..case.cycles.len() {
        cpu.cycle(&mut mem);
    }

    if cpu.registers.pc != case.end.pc {
        errors.push(format!(
            "{label} PC: expected ${:04X}, got ${:04X}",
            case.end.pc, cpu.registers.pc
        ));
    }
    if cpu.registers.sp != case.end.s {
        errors.push(format!(
            "{label} S: expected ${:02X}, got ${:02X}",
            case.end.s, cpu.registers.sp
        ));
    }
    if cpu.registers.a != case.end.a {
        errors.push(format!(
            "{label} A: expected ${:02X}, got ${:02X}",
            case.end.a, cpu.registers.a
        ));
    }
    if cpu.registers.x != case.end.x {
        errors.push(format!(
            "{label} X: expected ${:02X}, got ${:02X}",
            case.end.x, cpu.registers.x
        ));
    }
    if cpu.registers.y != case.end.y {
        errors.push(format!(
            "{label} Y: expected ${:02X}, got ${:02X}",
            case.end.y, cpu.registers.y
        ));
    }
    if cpu.registers.status != case.end.p {
        errors.push(format!(
            "{label} P: expected ${:02X}, got ${:02X}",
            case.end.p, cpu.registers.status
        ));
    }

    for &(addr, expected) in &case.end.ram {
        let actual = mem.read_byte(addr);
        if actual != expected {
            errors.push(format!(
                "{label} RAM[${:04X}]: expected ${:02X}, got ${:02X}",
                addr, expected, actual
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::path::PathBuf;

    #[rstest]
    fn run_opcode(#[files("external/6502/v1/[0-9a-f][0-9a-f].json")] path: PathBuf) {
        run_single_opcode(path);
    }

    #[test]
    fn test_one() {
        let path = PathBuf::from("external/6502/v1/07.json");
        run_single_opcode(path);
    }

    fn run_single_opcode(path: PathBuf) {
        let cases = parse_harte_test_file(path.to_str().unwrap());
        let total = cases.len();
        let mut failed = 0;
        let mut messages = Vec::new();

        for (i, case) in cases.iter().enumerate() {
            let errors = run_harte_test(i, case);
            if !errors.is_empty() {
                failed += 1;
                messages.extend(errors);
            }
        }

        if failed > 0 {
            let passed = total - failed;
            let file = path.file_stem().unwrap().to_str().unwrap();
            let summary = format!(
                "{file}.json: {passed} passed, {failed} failed\n{}",
                messages
                    .iter()
                    .take(20)
                    .map(|m| format!("  {m}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            let remaining = messages.len().saturating_sub(20);
            let tail = if remaining > 0 {
                format!("\n  ... and {remaining} more mismatches")
            } else {
                String::new()
            };
            panic!("{summary}{tail}");
        }
    }
}
