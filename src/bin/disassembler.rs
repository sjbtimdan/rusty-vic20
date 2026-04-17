use rusty_vic20::tools::disassembler::{DefaultDisassembler, disassemble_bytes};
use std::env::args;

// Example: cargo run --bin disassembler ./data/kernal.901486-07.bin E000 FF72 | head -10
fn main() {
    sigpipe::reset(); // so <disassemble> | head -10 will work.
    let filename = args()
        .nth(1)
        .expect("Usage: disassembler filename [base_address=0x0000] [disassemble_start_address=base_address]");
    let base_address = args()
        .nth(2)
        .map(|s| u16::from_str_radix(&s, 16).expect("Base address must be a hex number"))
        .unwrap_or(0x0000);
    let disassemble_start_address = args()
        .nth(3)
        .map(|s| u16::from_str_radix(&s, 16).expect("Disassemble start address must be a hex number"))
        .unwrap_or(base_address);
    let data = std::fs::read(&filename).expect("Failed to read file");
    let disassembler = DefaultDisassembler::new("\t".to_string());
    for line in disassemble_bytes(&data, &disassembler, base_address, disassemble_start_address) {
        println!("{}", line);
    }
}
