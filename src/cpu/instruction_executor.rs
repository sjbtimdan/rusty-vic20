use crate::{
    cpu::{
        instructions::{AddressingMode, Instruction, InstructionInfo},
        registers::Registers,
    },
    memory::Memory,
};

pub fn execute_instruction(
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: &InstructionInfo,
    operands: &[u8],
) {
    match instruction.instruction {
        Instruction::LDA => match instruction.mode {
            AddressingMode::Immediate => {
                registers.set_accumulator(operands[0]);
            }
            AddressingMode::ZeroPage => {
                let read = memory.read_zero_page(operands[0]);
                registers.set_accumulator(read);
            }
            _ => unimplemented!("Addressing mode {:?} not implemented for LDA", instruction.mode),
        },
        _ => unimplemented!("Instruction {:?} not implemented yet", instruction.instruction),
    }
}

#[cfg(test)]
mod tests {
    use rstest::fixture;
    use rstest::rstest;

    use super::*;
    use crate::cpu::instructions::{AddressingMode, Instruction, InstructionInfo};
    use crate::cpu::registers::{NEGATIVE_FLAG_BITMASK, Registers, ZERO_FLAG_BITMASK};

    const LDA_IMMEDIATE: InstructionInfo = InstructionInfo {
        opcode: 0xA9,
        instruction: Instruction::LDA,
        mode: AddressingMode::Immediate,
        cycles: 2,
    };

    #[fixture]
    fn registers() -> Registers {
        Registers::default()
    }

    #[fixture]
    fn memory() -> Memory {
        Memory::default()
    }

    #[rstest]
    fn test_lda_immediate(mut registers: Registers, mut memory: Memory) {
        execute_instruction(&mut registers, &mut memory, &LDA_IMMEDIATE, &[0x42]);
        assert_eq!(registers.a, 0x42);
        assert!(registers.is_flag_set(ZERO_FLAG_BITMASK)); // zero flag
        assert!(!registers.is_flag_set(NEGATIVE_FLAG_BITMASK)); // negative flag
    }

    #[rstest]
    fn test_lda_immediate_clears_zero_flag(mut registers: Registers, mut memory: Memory) {
        execute_instruction(&mut registers, &mut memory, &LDA_IMMEDIATE, &[0x01]);
        assert!(!registers.is_flag_set(ZERO_FLAG_BITMASK)); // zero flag cleared
    }

    #[rstest]
    fn test_lda_immediate_sets_negative_flag(mut registers: Registers, mut memory: Memory) {
        execute_instruction(&mut registers, &mut memory, &LDA_IMMEDIATE, &[0x80]);
        assert_eq!(registers.a, 0x80);
        assert!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK)); // negative flag
    }

    #[rstest]
    fn test_lda_immediate_clears_negative_flag(mut registers: Registers, mut memory: Memory) {
        execute_instruction(&mut registers, &mut memory, &LDA_IMMEDIATE, &[0x7F]);
        assert!(!registers.is_flag_set(NEGATIVE_FLAG_BITMASK)); // negative flag cleared
    }
}
