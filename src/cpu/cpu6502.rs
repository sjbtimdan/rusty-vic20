use crate::{cpu::registers::Registers, memory::Memory};

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
