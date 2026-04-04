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
            AddressingMode::ZeroPageX => {
                let address = operands[0].wrapping_add(registers.x);
                let read = memory.read_zero_page(address);
                registers.set_accumulator(read);
            }
            AddressingMode::Absolute => {
                let address = (operands[1] as u16) << 8 | operands[0] as u16;
                let read = memory.bytes[address as usize];
                registers.set_accumulator(read);
            }
            AddressingMode::AbsoluteX => {
                let address = ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.x as u16);
                let read = memory.bytes[address as usize];
                registers.set_accumulator(read);
            }
            AddressingMode::AbsoluteY => {
                let address = ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.y as u16);
                let read = memory.bytes[address as usize];
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
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    #[case(0x7F, false, false)]
    fn test_lda_immediate(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        execute_instruction(&mut registers, &mut memory, &LDA_IMMEDIATE, &[value]);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }
}
