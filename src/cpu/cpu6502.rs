use crate::{
    cpu::{
        instructions::{AddressingMode, Instruction, InstructionInfo},
        registers::Registers,
    },
    memory::Memory,
};

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

    pub fn execute_instruction(&mut self, instruction: &InstructionInfo, operands: &[u8]) {
        match instruction.instruction {
            Instruction::LDA => match instruction.mode {
                AddressingMode::Immediate => {
                    self.registers.set_accoumulator(operands[0]);
                }
                _ => unimplemented!(
                    "Addressing mode {:?} not implemented for LDA",
                    instruction.mode
                ),
            },
            _ => unimplemented!(
                "Instruction {:?} not implemented yet",
                instruction.instruction
            ),
        }
    }
}
