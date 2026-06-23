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
/// Initialises RAM and CPU registers from `case.initial`, steps through
/// `case.cycles.len()` cycles, then compares all register and RAM values
/// against `case.end`.
///
/// `case_index` is the 0-based position in the JSON file array, included
/// in panic messages to help locate failures.
///
/// # Panics
/// Panics with a descriptive message on any register or RAM mismatch.
fn run_harte_test(case_index: usize, case: &TestCase) {
    let label = format!("#{case_index} [{}]", case.name);

    let mut cpu = CPU6502::new();
    let mut mem = Ram::new();

    // Step 1: initialize RAM from the test's initial state.
    for &(addr, val) in &case.initial.ram {
        mem.write_byte(addr, val);
    }

    // Step 2: set CPU registers from the test's initial state.
    cpu.registers.pc = case.initial.pc;
    cpu.registers.sp = case.initial.s;
    cpu.registers.a = case.initial.a;
    cpu.registers.x = case.initial.x;
    cpu.registers.y = case.initial.y;
    cpu.registers.status = case.initial.p;

    // Step 3: run one instruction (first cycle triggers fetch, then keep
    // stepping until end_instruction clears the sequence).
    let cycles_before = cpu.total_cycles;
    cpu.cycle(&mut mem);
    while !cpu.instruction_complete() {
        cpu.cycle(&mut mem);
    }
    let cycles_taken = cpu.total_cycles - cycles_before;
    assert_eq!(
        cycles_taken as usize,
        case.cycles.len(),
        "{label} step 3 cycles: expected {}, took {cycles_taken}",
        case.cycles.len()
    );

    // Step 4: compare final register state.
    assert_eq!(
        cpu.registers.pc, case.end.pc,
        "{label} step 4 register PC: expected ${:04X}, got ${:04X}",
        case.end.pc, cpu.registers.pc
    );
    assert_eq!(
        cpu.registers.sp, case.end.s,
        "{label} step 4 register S: expected ${:02X}, got ${:02X}",
        case.end.s, cpu.registers.sp
    );
    assert_eq!(
        cpu.registers.a, case.end.a,
        "{label} step 4 register A: expected ${:02X}, got ${:02X}",
        case.end.a, cpu.registers.a
    );
    assert_eq!(
        cpu.registers.x, case.end.x,
        "{label} step 4 register X: expected ${:02X}, got ${:02X}",
        case.end.x, cpu.registers.x
    );
    assert_eq!(
        cpu.registers.y, case.end.y,
        "{label} step 4 register Y: expected ${:02X}, got ${:02X}",
        case.end.y, cpu.registers.y
    );
    assert_eq!(
        cpu.registers.status, case.end.p,
        "{label} step 4 register P: expected ${:02X}, got ${:02X}",
        case.end.p, cpu.registers.status
    );

    // Step 5: compare RAM state.
    for &(addr, expected) in &case.end.ram {
        let actual = mem.read_byte(addr);
        assert_eq!(
            actual, expected,
            "{label} step 5 RAM[${:04X}]: expected ${:02X}, got ${:02X}",
            addr, expected, actual
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_lda_a9() {
        let cases = parse_harte_test_file("external/6502/v1/a9.json");
        assert_eq!(cases.len(), 10_000);

        for (i, case) in cases.iter().enumerate() {
            run_harte_test(i, case);
        }
    }
}
