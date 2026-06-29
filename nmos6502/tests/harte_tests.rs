#[cfg(test)]
mod tests {
    use nmos6502::{memory::Ram, Addressable, CPU6502};
    use rstest::rstest;
    use serde::Deserialize;
    use std::path::PathBuf;

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

    /// Wraps a `Ram` and records bus operations per cycle.
    ///
    /// `last_bus` holds the most recent bus op in the current cycle.
    /// `bus_count` counts total bus ops in the current cycle — used to detect
    /// extra reads/writes from internal operations (e.g. `op_jsr_c6`).
    /// Reset both fields via `reset_for_new_cycle()` before each `cpu.cycle()`.
    struct BusRecorder<'a> {
        inner: &'a mut Ram,
        last_bus: Option<(u16, u8, bool)>,
        bus_count: u32,
    }

    impl<'a> BusRecorder<'a> {
        fn new(inner: &'a mut Ram) -> Self {
            Self {
                inner,
                last_bus: None,
                bus_count: 0,
            }
        }

        fn reset_for_new_cycle(&mut self) {
            self.last_bus = None;
            self.bus_count = 0;
        }
    }

    impl Addressable for BusRecorder<'_> {
        fn read_byte(&mut self, address: u16) -> u8 {
            let val = self.inner.read_byte(address);
            self.bus_count += 1;
            self.last_bus = Some((address, val, false));
            val
        }
        fn write_byte(&mut self, address: u16, value: u8) {
            self.inner.write_byte(address, value);
            self.bus_count += 1;
            self.last_bus = Some((address, value, true));
        }
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

    fn check_bus_cycle(
        errors: &mut Vec<String>,
        label: &str,
        cycle_index: usize,
        recorded: Option<(u16, u8, bool)>,
        expected: &(u16, u8, CycleType),
    ) {
        match recorded {
            Some((addr, val, is_write)) => {
                if addr != expected.0 {
                    errors.push(format!(
                        "{label} cycle {cycle_index}: address: expected ${:04X}, got ${:04X}",
                        expected.0, addr
                    ));
                }
                if val != expected.1 {
                    errors.push(format!(
                        "{label} cycle {cycle_index}: value: expected ${:02X}, got ${:02X}",
                        expected.1, val
                    ));
                }
                if is_write != (expected.2 == CycleType::Write) {
                    errors.push(format!(
                        "{label} cycle {cycle_index}: direction: expected {:?}, got {}",
                        expected.2,
                        if is_write { "write" } else { "read" }
                    ));
                }
            }
            None => {
                errors.push(format!(
                    "{label} cycle {cycle_index}: expected bus op but CPU produced none"
                ));
            }
        }
    }

    fn check_final_state(errors: &mut Vec<String>, label: &str, cpu: &CPU6502, mem: &mut Ram, end: &CpuState) {
        if cpu.registers.pc != end.pc {
            errors.push(format!(
                "{label} PC: expected ${:04X}, got ${:04X}",
                end.pc, cpu.registers.pc
            ));
        }
        if cpu.registers.sp != end.s {
            errors.push(format!(
                "{label} S: expected ${:02X}, got ${:02X}",
                end.s, cpu.registers.sp
            ));
        }
        if cpu.registers.a != end.a {
            errors.push(format!(
                "{label} A: expected ${:02X}, got ${:02X}",
                end.a, cpu.registers.a
            ));
        }
        if cpu.registers.x != end.x {
            errors.push(format!(
                "{label} X: expected ${:02X}, got ${:02X}",
                end.x, cpu.registers.x
            ));
        }
        if cpu.registers.y != end.y {
            errors.push(format!(
                "{label} Y: expected ${:02X}, got ${:02X}",
                end.y, cpu.registers.y
            ));
        }
        if cpu.registers.status != end.p {
            errors.push(format!(
                "{label} P: expected ${:02X}, got ${:02X}",
                end.p, cpu.registers.status
            ));
        }

        for &(addr, expected) in &end.ram {
            let actual = mem.read_byte(addr);
            if actual != expected {
                errors.push(format!(
                    "{label} RAM[${:04X}]: expected ${:02X}, got ${:02X}",
                    addr, expected, actual
                ));
            }
        }
    }

    /// Run a single Harte test case against the nmos6502 CPU.
    ///
    /// When `validate_bus` is `true`, each cycle's bus operation is compared against
    /// the expected trace in `case.cycles`.  Mismatches are reported but do not
    /// affect pass/fail — the final register/RAM check is the source of truth.
    ///
    /// Returns a list of mismatch descriptions. Empty `Vec` means the case passed.
    fn run_harte_test(case_index: usize, case: &TestCase, validate_bus: bool) -> Vec<String> {
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

        let mut recorder = BusRecorder::new(&mut mem);
        for (cycle_index, expected) in case.cycles.iter().enumerate() {
            recorder.reset_for_new_cycle();
            cpu.cycle(&mut recorder);
            if validate_bus {
                if recorder.bus_count != 1 {
                    errors.push(format!(
                        "{} cycle {cycle_index}: expected exactly 1 bus operation, got {}",
                        label, recorder.bus_count,
                    ));
                }
                check_bus_cycle(&mut errors, &label, cycle_index, recorder.last_bus, expected);
            }
        }

        check_final_state(&mut errors, &label, &cpu, &mut mem, &case.end);

        errors
    }

    fn run_single_opcode(path: PathBuf, validate_bus: bool) {
        let cases = parse_harte_test_file(path.to_str().unwrap());
        let total = cases.len();
        let mut failed = 0;
        let mut messages = Vec::new();

        for (i, case) in cases.iter().enumerate() {
            let errors = run_harte_test(i, case, validate_bus);
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

    #[rstest]
    fn run_opcode(#[files("external/6502/v1/[0-9a-f][0-9a-f].json")] path: PathBuf) {
        run_single_opcode(path, true);
    }
}
