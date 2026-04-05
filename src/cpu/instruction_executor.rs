use crate::{
    cpu::{
        instructions::{Instruction, InstructionInfo},
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
        Instruction::LDA => {
            let value = instruction.mode.resolve_load_operand(registers, memory, operands);
            registers.set_accumulator(value);
        }
        Instruction::LDX => {
            let value = instruction.mode.resolve_load_operand(registers, memory, operands);
            registers.set_x(value);
        }
        Instruction::LDY => {
            let value = instruction.mode.resolve_load_operand(registers, memory, operands);
            registers.set_y(value);
        }
        Instruction::DEX => {
            registers.set_x(registers.x.wrapping_sub(1));
        }
        Instruction::DEY => {
            registers.set_y(registers.y.wrapping_sub(1));
        }
        Instruction::INX => {
            registers.set_x(registers.x.wrapping_add(1));
        }
        Instruction::INY => {
            registers.set_y(registers.y.wrapping_add(1));
        }
        Instruction::INC => {
            let address = instruction.mode.resolve_store_address(registers, memory, operands);
            let value = memory.read_byte(address).wrapping_add(1);
            memory.set_byte(address, value);
            registers.update_zero_and_negative(value);
        }
        Instruction::DEC => {
            let address = instruction.mode.resolve_store_address(registers, memory, operands);
            let value = memory.read_byte(address).wrapping_sub(1);
            memory.set_byte(address, value);
            registers.update_zero_and_negative(value);
        }
        Instruction::STA => {
            let address = instruction.mode.resolve_store_address(registers, memory, operands);
            memory.set_byte(address, registers.a);
        }
        Instruction::STX => {
            let address = instruction.mode.resolve_store_address(registers, memory, operands);
            memory.set_byte(address, registers.x);
        }
        Instruction::STY => {
            let address = instruction.mode.resolve_store_address(registers, memory, operands);
            memory.set_byte(address, registers.y);
        }
        _ => unimplemented!("Instruction {:?} not implemented yet", instruction.instruction),
    }
}

#[cfg(test)]
mod tests {
    use rstest::fixture;
    use rstest::rstest;

    use super::*;
    use crate::cpu::instructions::*;
    use crate::cpu::registers::*;

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
        registers.set_x(x);
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
        registers.set_x(x);
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
        registers.set_y(y);
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
        registers.set_y(y);
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
        registers.set_x(x);
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

    #[rstest]
    #[case(0x01, 0x02, false, false)]
    #[case(0xFF, 0x00, true, false)]
    #[case(0x7F, 0x80, false, true)]
    fn test_inx(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.set_x(initial);
        execute_instruction(&mut registers, &mut memory, &INX_IMPLIED, &[]);
        assert_eq!(registers.x, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x01, 0x02, false, false)]
    #[case(0xFF, 0x00, true, false)]
    #[case(0x7F, 0x80, false, true)]
    fn test_iny(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.set_y(initial);
        execute_instruction(&mut registers, &mut memory, &INY_IMPLIED, &[]);
        assert_eq!(registers.y, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x02, 0x01, false, false)]
    #[case(0x01, 0x00, true, false)]
    #[case(0x00, 0xFF, false, true)]
    fn test_dex(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.set_x(initial);
        execute_instruction(&mut registers, &mut memory, &DEX_IMPLIED, &[]);
        assert_eq!(registers.x, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x02, 0x01, false, false)]
    #[case(0x01, 0x00, true, false)]
    #[case(0x00, 0xFF, false, true)]
    fn test_dey(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.set_y(initial);
        execute_instruction(&mut registers, &mut memory, &DEY_IMPLIED, &[]);
        assert_eq!(registers.y, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x41, 0x42, false, false)]
    #[case(0x10, 0xFF, 0x00, true, false)]
    #[case(0x10, 0x7F, 0x80, false, true)]
    fn test_inc_zero_page(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u8,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_zero_page_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &INC_ZERO_PAGE, &[address]);
        assert_eq!(memory.read_zero_page_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x03, 0x13, 0x41, 0x42, false, false)]
    #[case(0x10, 0x03, 0x13, 0xFF, 0x00, true, false)]
    #[case(0x10, 0x03, 0x13, 0x7F, 0x80, false, true)]
    fn test_inc_zero_page_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] x: u8,
        #[case] address: u8,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.x = x;
        memory.set_zero_page_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &INC_ZERO_PAGE_X, &[base]);
        assert_eq!(memory.read_zero_page_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x00, 0x02, 0x0200, 0x41, 0x42, false, false)]
    #[case(0x00, 0x02, 0x0200, 0xFF, 0x00, true, false)]
    #[case(0x00, 0x02, 0x0200, 0x7F, 0x80, false, true)]
    fn test_inc_absolute(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &INC_ABSOLUTE, &[lo, hi]);
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x41, 0x42, false, false)]
    #[case(0x00, 0x02, 0x04, 0x0204, 0xFF, 0x00, true, false)]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x7F, 0x80, false, true)]
    fn test_inc_absolute_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] x: u8,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.x = x;
        memory.set_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &INC_ABSOLUTE_X, &[lo, hi]);
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x42, 0x41, false, false)]
    #[case(0x10, 0x00, 0xFF, false, true)]
    #[case(0x10, 0x01, 0x00, true, false)]
    fn test_dec_zero_page(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u8,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_zero_page_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &DEC_ZERO_PAGE, &[address]);
        assert_eq!(memory.read_zero_page_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x03, 0x13, 0x42, 0x41, false, false)]
    #[case(0x10, 0x03, 0x13, 0x00, 0xFF, false, true)]
    #[case(0x10, 0x03, 0x13, 0x01, 0x00, true, false)]
    fn test_dec_zero_page_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] x: u8,
        #[case] address: u8,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.x = x;
        memory.set_zero_page_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &DEC_ZERO_PAGE_X, &[base]);
        assert_eq!(memory.read_zero_page_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x00, 0x02, 0x0200, 0x42, 0x41, false, false)]
    #[case(0x00, 0x02, 0x0200, 0x00, 0xFF, false, true)]
    #[case(0x00, 0x02, 0x0200, 0x01, 0x00, true, false)]
    fn test_dec_absolute(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.set_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &DEC_ABSOLUTE, &[lo, hi]);
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x42, 0x41, false, false)]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x00, 0xFF, false, true)]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x01, 0x00, true, false)]
    fn test_dec_absolute_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] x: u8,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        registers.x = x;
        memory.set_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &DEC_ABSOLUTE_X, &[lo, hi]);
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x10, 0x42)]
    fn test_sta_zero_page(mut registers: Registers, mut memory: Memory, #[case] address: u8, #[case] value: u8) {
        registers.a = value;
        execute_instruction(&mut registers, &mut memory, &STA_ZERO_PAGE, &[address]);
        assert_eq!(memory.read_zero_page_byte(address), value);
    }

    #[rstest]
    #[case(0x10, 0x03, 0x13, 0x42)]
    fn test_sta_zero_page_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] x: u8,
        #[case] address: u8,
        #[case] value: u8,
    ) {
        registers.a = value;
        registers.x = x;
        execute_instruction(&mut registers, &mut memory, &STA_ZERO_PAGE_X, &[base]);
        assert_eq!(memory.read_zero_page_byte(address), value);
    }

    #[rstest]
    #[case(0x00, 0x02, 0x0200, 0x42)]
    fn test_sta_absolute(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] address: u16,
        #[case] value: u8,
    ) {
        registers.a = value;
        execute_instruction(&mut registers, &mut memory, &STA_ABSOLUTE, &[lo, hi]);
        assert_eq!(memory.read_byte(address), value);
    }

    #[rstest]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x42)]
    fn test_sta_absolute_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] x: u8,
        #[case] address: u16,
        #[case] value: u8,
    ) {
        registers.a = value;
        registers.x = x;
        execute_instruction(&mut registers, &mut memory, &STA_ABSOLUTE_X, &[lo, hi]);
        assert_eq!(memory.read_byte(address), value);
    }

    #[rstest]
    #[case(0x00, 0x02, 0x04, 0x0204, 0x42)]
    fn test_sta_absolute_y(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] y: u8,
        #[case] address: u16,
        #[case] value: u8,
    ) {
        registers.a = value;
        registers.y = y;
        execute_instruction(&mut registers, &mut memory, &STA_ABSOLUTE_Y, &[lo, hi]);
        assert_eq!(memory.read_byte(address), value);
    }

    #[rstest]
    #[case(0x10, 0x03, 0x13, 0x0200, 0x42)]
    fn test_sta_indexed_indirect(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] x: u8,
        #[case] ptr: u8,
        #[case] address: u16,
        #[case] value: u8,
    ) {
        registers.a = value;
        registers.x = x;
        memory.set_zero_page_word(ptr, address);
        execute_instruction(&mut registers, &mut memory, &STA_INDEXED_INDIRECT, &[base]);
        assert_eq!(memory.read_byte(address), value);
    }

    #[rstest]
    #[case(0x20, 0x03, 0xC003, 0x42)]
    fn test_sta_indirect_indexed(
        mut registers: Registers,
        mut memory: Memory,
        #[case] zp_addr: u8,
        #[case] y: u8,
        #[case] address: u16,
        #[case] value: u8,
    ) {
        registers.a = value;
        registers.y = y;
        let base = address.wrapping_sub(y as u16);
        memory.set_zero_page_word(zp_addr, base);
        execute_instruction(&mut registers, &mut memory, &STA_INDIRECT_INDEXED, &[zp_addr]);
        assert_eq!(memory.read_byte(address), value);
    }

    #[rstest]
    #[case(0x10, 0x42)]
    fn test_stx_zero_page(mut registers: Registers, mut memory: Memory, #[case] address: u8, #[case] value: u8) {
        registers.x = value;
        execute_instruction(&mut registers, &mut memory, &STX_ZERO_PAGE, &[address]);
        assert_eq!(memory.read_zero_page_byte(address), value);
    }

    #[rstest]
    #[case(0x10, 0x03, 0x13, 0x42)]
    fn test_stx_zero_page_y(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] y: u8,
        #[case] address: u8,
        #[case] value: u8,
    ) {
        registers.x = value;
        registers.y = y;
        execute_instruction(&mut registers, &mut memory, &STX_ZERO_PAGE_Y, &[base]);
        assert_eq!(memory.read_zero_page_byte(address), value);
    }

    #[rstest]
    #[case(0x00, 0x02, 0x0200, 0x42)]
    fn test_stx_absolute(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] address: u16,
        #[case] value: u8,
    ) {
        registers.x = value;
        execute_instruction(&mut registers, &mut memory, &STX_ABSOLUTE, &[lo, hi]);
        assert_eq!(memory.read_byte(address), value);
    }

    #[rstest]
    #[case(0x10, 0x42)]
    fn test_sty_zero_page(mut registers: Registers, mut memory: Memory, #[case] address: u8, #[case] value: u8) {
        registers.y = value;
        execute_instruction(&mut registers, &mut memory, &STY_ZERO_PAGE, &[address]);
        assert_eq!(memory.read_zero_page_byte(address), value);
    }

    #[rstest]
    #[case(0x10, 0x03, 0x13, 0x42)]
    fn test_sty_zero_page_x(
        mut registers: Registers,
        mut memory: Memory,
        #[case] base: u8,
        #[case] x: u8,
        #[case] address: u8,
        #[case] value: u8,
    ) {
        registers.y = value;
        registers.x = x;
        execute_instruction(&mut registers, &mut memory, &STY_ZERO_PAGE_X, &[base]);
        assert_eq!(memory.read_zero_page_byte(address), value);
    }

    #[rstest]
    #[case(0x00, 0x02, 0x0200, 0x42)]
    fn test_sty_absolute(
        mut registers: Registers,
        mut memory: Memory,
        #[case] lo: u8,
        #[case] hi: u8,
        #[case] address: u16,
        #[case] value: u8,
    ) {
        registers.y = value;
        execute_instruction(&mut registers, &mut memory, &STY_ABSOLUTE, &[lo, hi]);
        assert_eq!(memory.read_byte(address), value);
    }
}
