use crate::{
    cpu::{
        instructions::{Instruction, InstructionInfoOps},
        registers::Registers,
    },
    memory::Memory,
};

pub fn execute_instruction(
    registers: &mut Registers,
    memory: &mut Memory,
    instruction_info: &impl InstructionInfoOps,
    operands: &[u8],
) {
    let instruction = instruction_info.instruction();
    match instruction {
        Instruction::LDA => {
            let value = instruction_info.resolve_load_operand(registers, memory, operands);
            registers.set_accumulator(value);
        }
        Instruction::LDX => {
            let value = instruction_info.resolve_load_operand(registers, memory, operands);
            registers.set_x(value);
        }
        Instruction::LDY => {
            let value = instruction_info.resolve_load_operand(registers, memory, operands);
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
            let address = instruction_info.resolve_store_address(registers, memory, operands);
            let value = memory.read_byte(address).wrapping_add(1);
            memory.set_byte(address, value);
            registers.update_zero_and_negative(value);
        }
        Instruction::DEC => {
            let address = instruction_info.resolve_store_address(registers, memory, operands);
            let value = memory.read_byte(address).wrapping_sub(1);
            memory.set_byte(address, value);
            registers.update_zero_and_negative(value);
        }
        Instruction::STA => {
            let address = instruction_info.resolve_store_address(registers, memory, operands);
            memory.set_byte(address, registers.a);
        }
        Instruction::STX => {
            let address = instruction_info.resolve_store_address(registers, memory, operands);
            memory.set_byte(address, registers.x);
        }
        Instruction::STY => {
            let address = instruction_info.resolve_store_address(registers, memory, operands);
            memory.set_byte(address, registers.y);
        }
        Instruction::ORA => {
            let value = instruction_info.resolve_load_operand(registers, memory, operands);
            registers.set_accumulator(registers.a | value);
        }
        Instruction::AND => {
            let value = instruction_info.resolve_load_operand(registers, memory, operands);
            registers.set_accumulator(registers.a & value);
        }
        Instruction::EOR => {
            let value = instruction_info.resolve_load_operand(registers, memory, operands);
            registers.set_accumulator(registers.a ^ value);
        }
        Instruction::JMP => {
            let address = instruction_info.resolve_store_address(registers, memory, operands);
            registers.pc = address;
        }
        _ => unimplemented!("Instruction {:?} not implemented yet", instruction),
    }
}

#[cfg(test)]
mod tests {
    use rstest::fixture;
    use rstest::rstest;
    use unimock::MockFn;
    use unimock::matching;

    use super::*;
    use crate::cpu::instructions::InstructionInfoOpsMock;
    use crate::cpu::instructions::*;
    use crate::cpu::registers::*;
    use unimock::Unimock;

    #[fixture]
    fn registers() -> Registers {
        Registers::default()
    }

    #[fixture]
    fn memory() -> Memory {
        Memory::default()
    }

    #[rstest]
    fn test_lda(mut registers: Registers, mut memory: Memory) {
        let lda_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::LDA),
            InstructionInfoOpsMock::resolve_load_operand
                .each_call(matching!(_, _, [0x42]))
                .returns(0x42),
        ));
        execute_instruction(&mut registers, &mut memory, &lda_instruction, &[0x42]);
        assert_eq!(registers.a, 0x42);
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_ldx(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let ldx_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::LDX),
            InstructionInfoOpsMock::resolve_load_operand
                .each_call(matching!(_, _, _))
                .returns(value),
        ));
        execute_instruction(&mut registers, &mut memory, &ldx_instruction, &[value]);
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_ldy(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let ldy_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::LDY),
            InstructionInfoOpsMock::resolve_load_operand
                .each_call(matching!(_, _, _))
                .returns(value),
        ));
        execute_instruction(&mut registers, &mut memory, &ldy_instruction, &[value]);
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
        let inx_instruction = Unimock::new(
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::INX),
        );
        registers.set_x(initial);
        execute_instruction(&mut registers, &mut memory, &inx_instruction, &[]);
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
        let iny_instruction = Unimock::new(
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::INY),
        );
        registers.set_y(initial);
        execute_instruction(&mut registers, &mut memory, &iny_instruction, &[]);
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
        let dex_instruction = Unimock::new(
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::DEX),
        );
        registers.set_x(initial);
        execute_instruction(&mut registers, &mut memory, &dex_instruction, &[]);
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
        let dey_instruction = Unimock::new(
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::DEY),
        );
        registers.set_y(initial);
        execute_instruction(&mut registers, &mut memory, &dey_instruction, &[]);
        assert_eq!(registers.y, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x0200u16, 0x41, 0x42, false, false)]
    #[case(0x0200u16, 0xFF, 0x00, true, false)]
    #[case(0x0200u16, 0x7F, 0x80, false, true)]
    fn test_inc(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let inc_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::INC),
            InstructionInfoOpsMock::resolve_store_address
                .each_call(matching!(_, _, _))
                .returns(address),
        ));
        memory.set_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &inc_instruction, &[]);
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x0200u16, 0x42, 0x41, false, false)]
    #[case(0x0200u16, 0x00, 0xFF, false, true)]
    #[case(0x0200u16, 0x01, 0x00, true, false)]
    fn test_dec(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let dec_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::DEC),
            InstructionInfoOpsMock::resolve_store_address
                .each_call(matching!(_, _, _))
                .returns(address),
        ));
        memory.set_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, &dec_instruction, &[]);
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    fn test_sta(mut registers: Registers, mut memory: Memory) {
        let sta_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::STA),
            InstructionInfoOpsMock::resolve_store_address
                .each_call(matching!(_, _, _))
                .returns(0x0200u16),
        ));
        registers.a = 0x42;
        execute_instruction(&mut registers, &mut memory, &sta_instruction, &[]);
        assert_eq!(memory.read_byte(0x0200), 0x42);
    }

    #[rstest]
    fn test_stx(mut registers: Registers, mut memory: Memory) {
        let stx_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::STX),
            InstructionInfoOpsMock::resolve_store_address
                .each_call(matching!(_, _, _))
                .returns(0x0200u16),
        ));
        registers.x = 0x42;
        execute_instruction(&mut registers, &mut memory, &stx_instruction, &[]);
        assert_eq!(memory.read_byte(0x0200), 0x42);
    }

    #[rstest]
    fn test_sty(mut registers: Registers, mut memory: Memory) {
        let sty_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::STY),
            InstructionInfoOpsMock::resolve_store_address
                .each_call(matching!(_, _, _))
                .returns(0x0200u16),
        ));
        registers.y = 0x42;
        execute_instruction(&mut registers, &mut memory, &sty_instruction, &[]);
        assert_eq!(memory.read_byte(0x0200), 0x42);
    }

    #[rstest]
    #[case(0xFF, 0xFF, 0x00, true, false)]
    #[case(0x0F, 0x0F, 0x00, true, false)]
    #[case(0x0F, 0xF0, 0xFF, false, true)]
    #[case(0x42, 0x15, 0x57, false, false)]
    fn test_eor(
        mut registers: Registers,
        mut memory: Memory,
        #[case] accumulator: u8,
        #[case] operand: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let eor_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::EOR),
            InstructionInfoOpsMock::resolve_load_operand
                .each_call(matching!(_, _, _))
                .returns(operand),
        ));
        registers.set_accumulator(accumulator);
        execute_instruction(&mut registers, &mut memory, &eor_instruction, &[operand]);
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x0F, 0xF0, 0x00, true, false)]
    #[case(0xFF, 0xFF, 0xFF, false, true)]
    #[case(0xFF, 0x0F, 0x0F, false, false)]
    fn test_and(
        mut registers: Registers,
        mut memory: Memory,
        #[case] accumulator: u8,
        #[case] operand: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let and_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::AND),
            InstructionInfoOpsMock::resolve_load_operand
                .each_call(matching!(_, _, _))
                .returns(operand),
        ));
        registers.set_accumulator(accumulator);
        execute_instruction(&mut registers, &mut memory, &and_instruction, &[operand]);
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x0F, 0xF0, 0xFF, false, true)]
    #[case(0x00, 0x00, 0x00, true, false)]
    #[case(0x42, 0x15, 0x57, false, false)]
    fn test_ora(
        mut registers: Registers,
        mut memory: Memory,
        #[case] accumulator: u8,
        #[case] operand: u8,
        #[case] expected: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let ora_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::ORA),
            InstructionInfoOpsMock::resolve_load_operand
                .each_call(matching!(_, _, _))
                .returns(operand),
        ));
        registers.set_accumulator(accumulator);
        execute_instruction(&mut registers, &mut memory, &ora_instruction, &[operand]);
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    fn test_jmp(mut registers: Registers, mut memory: Memory) {
        let jmp_instruction = Unimock::new((
            InstructionInfoOpsMock::instruction
                .each_call(matching!())
                .returns(Instruction::JMP),
            InstructionInfoOpsMock::resolve_store_address
                .each_call(matching!(_, _, _))
                .returns(0x1234u16),
        ));
        execute_instruction(&mut registers, &mut memory, &jmp_instruction, &[0x34, 0x12]);
        assert_eq!(registers.pc, 0x1234);
    }
}
