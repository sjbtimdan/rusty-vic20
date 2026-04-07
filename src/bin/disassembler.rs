use rusty_vic20::tools::disassembler::{DefaultDisassembler, disassemble_bytes};
use std::env::args;

fn main() {
    sigpipe::reset(); // so <disassemble> | head -10 will work.
    let filename = args().nth(1).expect("Usage: disassembler <filename>");
    let data = std::fs::read(&filename).expect("Failed to read file");
    let disassembler = DefaultDisassembler::new("\t".to_string());
    for line in disassemble_bytes(&data, &disassembler) {
        println!("{}", line);
    }
}
