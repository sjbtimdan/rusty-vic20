use rusty_vic20::cpu::instructions::LDA_IMMEDIATE;
use rusty_vic20::tools::disassembler::{DefaultDisassembler, Disassembler};

fn main() {
    let disassembler = DefaultDisassembler::new(" ".to_string());
    println!("{}", disassembler.disassemble_instruction(&LDA_IMMEDIATE, &[0x45]));
}
