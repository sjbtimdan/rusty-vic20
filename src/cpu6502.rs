use crate::memory::Memory;

#[derive(Default)]
pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,     // Stack Pointer
    pub pc: u16,    // Program Counter
    pub status: u8, // Processor Status
}

pub struct CPU6502<'a> {
    pub registers: Registers,
    pub memory: &'a Memory,
}

impl<'a> CPU6502<'a> {
    pub fn new(memory: &'a Memory) -> Self {
        Self {
            registers: Registers::default(),
            memory,
        }
    }
}
