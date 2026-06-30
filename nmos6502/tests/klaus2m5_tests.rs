//! Assemble the full Klaus Dormann 6502 functional test suite.
//!
//! This just checks that the assembler can handle the real AS65 source
//! files without errors.  The assembled output is not verified against
//! a golden binary — the functional test itself does that at runtime.

use nmos6502::assembler::assemble;

fn check_assemble(source: &str, label: &str) {
    let result = assemble(source, 0, None);
    match result {
        Ok((bytes, syms)) => {
            eprintln!("{label}: Assembled {} bytes with {} symbols", bytes.len(), syms.len());
        }
        Err(e) => {
            panic!("{label} failed: {e}");
        }
    }
}

#[test]
fn assemble_decimal_test() {
    check_assemble(
        include_str!("../external/Klaus2m5/6502_decimal_test.a65"),
        "decimal_test",
    );
}

#[test]
fn assemble_functional_test() {
    check_assemble(
        include_str!("../external/Klaus2m5/6502_functional_test.a65"),
        "functional_test",
    );
}
