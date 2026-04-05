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
                let read = memory.read_zero_page_byte(operands[0]);
                registers.set_accumulator(read);
            }
            AddressingMode::ZeroPageX => {
                let address = operands[0].wrapping_add(registers.x);
                let read = memory.read_zero_page_byte(address);
                registers.set_accumulator(read);
            }
            AddressingMode::Absolute => {
                let address = (operands[1] as u16) << 8 | operands[0] as u16;
                let read = memory.read_byte(address);
                registers.set_accumulator(read);
            }
            AddressingMode::AbsoluteX => {
                let address = ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.x as u16);
                let read = memory.read_byte(address);
                registers.set_accumulator(read);
            }
            AddressingMode::AbsoluteY => {
                let address = ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.y as u16);
                let read = memory.read_byte(address);
                registers.set_accumulator(read);
            }
            AddressingMode::IndexedIndirect => {
                let ptr = operands[0].wrapping_add(registers.x);
                let address = memory.read_zero_page_word(ptr);
                let read = memory.read_byte(address);
                registers.set_accumulator(read);
            }
            AddressingMode::IndirectIndexed => {
                let base = memory.read_zero_page_word(operands[0]);
                let address = base.wrapping_add(registers.y as u16);
                let read = memory.read_byte(address);
                registers.set_accumulator(read);
            }
            _ => unimplemented!("Addressing mode {:?} not implemented for LDA", instruction.mode),
        },
        Instruction::LDX => match instruction.mode {
            AddressingMode::Immediate => {
                registers.set_x(operands[0]);
            }
            AddressingMode::ZeroPage => {
                let read = memory.read_zero_page_byte(operands[0]);
                registers.set_x(read);
            }
            AddressingMode::ZeroPageY => {
                let address = operands[0].wrapping_add(registers.y);
                let read = memory.read_zero_page_byte(address);
                registers.set_x(read);
            }
            AddressingMode::Absolute => {
                let address = (operands[1] as u16) << 8 | operands[0] as u16;
                let read = memory.read_byte(address);
                registers.set_x(read);
            }
            AddressingMode::AbsoluteY => {
                let address = ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.y as u16);
                let read = memory.read_byte(address);
                registers.set_x(read);
            }
            _ => unimplemented!("Addressing mode {:?} not implemented for LDX", instruction.mode),
        },
        Instruction::LDY => match instruction.mode {
            AddressingMode::Immediate => {
                registers.set_y(operands[0]);
            }
            AddressingMode::ZeroPage => {
                let read = memory.read_zero_page_byte(operands[0]);
                registers.set_y(read);
            }
            AddressingMode::ZeroPageX => {
                let address = operands[0].wrapping_add(registers.x);
                let read = memory.read_zero_page_byte(address);
                registers.set_y(read);
            }
            AddressingMode::Absolute => {
                let address = (operands[1] as u16) << 8 | operands[0] as u16;
                let read = memory.read_byte(address);
                registers.set_y(read);
            }
            AddressingMode::AbsoluteX => {
                let address = ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.x as u16);
                let read = memory.read_byte(address);
                registers.set_y(read);
            }
            _ => unimplemented!("Addressing mode {:?} not implemented for LDY", instruction.mode),
        },
        _ => unimplemented!("Instruction {:?} not implemented yet", instruction.instruction),
    }
}

#[cfg(test)]
mod tests {
    use rstest::fixture;
    use rstest::rstest;

    use super::*;
    use crate::cpu::instructions::{
        LDA_ABSOLUTE, LDA_ABSOLUTE_X, LDA_ABSOLUTE_Y, LDA_IMMEDIATE, LDA_INDEXED_INDIRECT, LDA_INDIRECT_INDEXED,
        LDA_ZERO_PAGE, LDA_ZERO_PAGE_X, LDX_ABSOLUTE, LDX_ABSOLUTE_Y, LDX_IMMEDIATE, LDX_ZERO_PAGE, LDX_ZERO_PAGE_Y,
        LDY_ABSOLUTE, LDY_ABSOLUTE_X, LDY_IMMEDIATE, LDY_ZERO_PAGE, LDY_ZERO_PAGE_X,
    };
    use crate::cpu::registers::{NEGATIVE_FLAG_BITMASK, Registers, ZERO_FLAG_BITMASK};

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

    #[rstest]
    #[case(0x10, 0x42, false, false)]
    #[case(0x20, 0x00, true, false)]
    #[case(0x30, 0x80, false, true)]
    fn test_lda_zero_page(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u8,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_zero_page_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDA_ZERO_PAGE, &[address]);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x05, 0x15, 0x42, false, false)]
    #[case(0x20, 0x03, 0x23, 0x00, true, false)]
    #[case(0x30, 0x02, 0x32, 0x80, false, true)]
    fn test_lda_zero_page_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] x: u8,
        #[case] address: u8,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.x = x;
        memory.set_zero_page_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDA_ZERO_PAGE_X, &[base]);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x02, 0x00, 0x0002, 0x42, false, false)]
    #[case(0x03, 0x00, 0x0003, 0x00, true, false)]
    #[case(0x04, 0x00, 0x0004, 0x80, false, true)]
    fn test_lda_absolute(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDA_ABSOLUTE, &[lo, hi]);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x42, false, false)]
    #[case(0x00, 0x03, 0x02, 0x0302, 0x00, true, false)]
    #[case(0x00, 0x04, 0x01, 0x0401, 0x80, false, true)]
    fn test_lda_absolute_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] x: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.set_x(x);
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDA_ABSOLUTE_X, &[lo, hi]);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x42, false, false)]
    #[case(0x00, 0x03, 0x02, 0x0302, 0x00, true, false)]
    #[case(0x00, 0x04, 0x01, 0x0401, 0x80, false, true)]
    fn test_lda_absolute_y(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] y: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.set_y(y);
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDA_ABSOLUTE_Y, &[lo, hi]);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x03, 0x13, 0x0005, 0x42, false, false)]
    #[case(0x20, 0x01, 0x21, 0x0006, 0x00, true, false)]
    #[case(0x30, 0x02, 0x32, 0x0007, 0x80, false, true)]
    fn test_lda_indexed_indirect(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] x: u8,
        #[case] ptr: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.x = x;
        memory.set_zero_page_word(ptr, address);
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDA_INDEXED_INDIRECT, &[base]);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x20, 0x03, 0xC003, 0x42, false, false)]
    #[case(0x20, 0x05, 0xC005, 0x00, true, false)]
    #[case(0x20, 0x01, 0xC001, 0x80, false, true)]
    fn test_lda_indirect_indexed(
        mut registers: Registers,
        mut memory: Memory,
        #[case] zp_addr: u8,
        #[case] y: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.y = y;
        let base = address.wrapping_sub(y as u16);
        memory.set_zero_page_word(zp_addr, base);
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDA_INDIRECT_INDEXED, &[zp_addr]);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_ldx_immediate(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        execute_instruction(&mut registers, &mut memory, &LDX_IMMEDIATE, &[value]);
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x42, false, false)]
    #[case(0x20, 0x00, true, false)]
    #[case(0x30, 0x80, false, true)]
    fn test_ldx_zero_page(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u8,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_zero_page_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDX_ZERO_PAGE, &[address]);
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x05, 0x15, 0x42, false, false)]
    #[case(0x20, 0x03, 0x23, 0x00, true, false)]
    #[case(0x30, 0x02, 0x32, 0x80, false, true)]
    fn test_ldx_zero_page_y(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] y: u8,
        #[case] address: u8,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.y = y;
        memory.set_zero_page_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDX_ZERO_PAGE_Y, &[base]);
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x02, 0x00, 0x0002, 0x42, false, false)]
    #[case(0x03, 0x00, 0x0003, 0x00, true, false)]
    #[case(0x04, 0x00, 0x0004, 0x80, false, true)]
    fn test_ldx_absolute(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDX_ABSOLUTE, &[lo, hi]);
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x42, false, false)]
    #[case(0x00, 0x03, 0x02, 0x0302, 0x00, true, false)]
    #[case(0x00, 0x04, 0x01, 0x0401, 0x80, false, true)]
    fn test_ldx_absolute_y(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] y: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.set_y(y);
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDX_ABSOLUTE_Y, &[lo, hi]);
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_ldy_immediate(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        execute_instruction(&mut registers, &mut memory, &LDY_IMMEDIATE, &[value]);
        assert_eq!(registers.y, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x42, false, false)]
    #[case(0x20, 0x00, true, false)]
    #[case(0x30, 0x80, false, true)]
    fn test_ldy_zero_page(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u8,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_zero_page_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDY_ZERO_PAGE, &[address]);
        assert_eq!(registers.y, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x05, 0x15, 0x42, false, false)]
    #[case(0x20, 0x03, 0x23, 0x00, true, false)]
    #[case(0x30, 0x02, 0x32, 0x80, false, true)]
    fn test_ldy_zero_page_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] x: u8,
        #[case] address: u8,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.x = x;
        memory.set_zero_page_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDY_ZERO_PAGE_X, &[base]);
        assert_eq!(registers.y, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x02, 0x00, 0x0002, 0x42, false, false)]
    #[case(0x03, 0x00, 0x0003, 0x00, true, false)]
    #[case(0x04, 0x00, 0x0004, 0x80, false, true)]
    fn test_ldy_absolute(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDY_ABSOLUTE, &[lo, hi]);
        assert_eq!(registers.y, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x42, false, false)]
    #[case(0x00, 0x03, 0x02, 0x0302, 0x00, true, false)]
    #[case(0x00, 0x04, 0x01, 0x0401, 0x80, false, true)]
    fn test_ldy_absolute_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] x: u8,
        #[case] address: u16,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.set_x(x);
        memory.set_byte(address, value);
        execute_instruction(&mut registers, &mut memory, &LDY_ABSOLUTE_X, &[lo, hi]);
        assert_eq!(registers.y, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }
}
