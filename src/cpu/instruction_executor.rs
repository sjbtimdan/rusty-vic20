use crate::{
    cpu::{addressing_mode::OperandResolution, instructions::Instruction, registers::Registers},
    memory::Memory,
};

pub fn execute_instruction(
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: Instruction,
    operand_resolution: &impl OperandResolution,
    operands: &[u8],
) {
    match instruction {
        Instruction::LDA => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_accumulator(value);
        }
        Instruction::LDX => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_x(value);
        }
        Instruction::LDY => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
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
            let address = operand_resolution.resolve_address(registers, memory, operands);
            let value = memory.read_byte(address).wrapping_add(1);
            memory.set_byte(address, value);
            registers.update_zero_and_negative(value);
        }
        Instruction::DEC => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            let value = memory.read_byte(address).wrapping_sub(1);
            memory.set_byte(address, value);
            registers.update_zero_and_negative(value);
        }
        Instruction::STA => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            memory.set_byte(address, registers.a);
        }
        Instruction::STX => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            memory.set_byte(address, registers.x);
        }
        Instruction::STY => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            memory.set_byte(address, registers.y);
        }
        Instruction::ORA => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_accumulator(registers.a | value);
        }
        Instruction::AND => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_accumulator(registers.a & value);
        }
        Instruction::EOR => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_accumulator(registers.a ^ value);
        }
        Instruction::JMP => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            registers.pc = address;
        }
        Instruction::CLC => registers.update_carry_flag(false),
        Instruction::CLD => registers.update_decimal_flag(false),
        Instruction::CLI => registers.update_interrupt_flag(false),
        Instruction::CLV => registers.update_overflow_flag(false),
        Instruction::SEC => registers.update_carry_flag(true),
        Instruction::SED => registers.update_decimal_flag(true),
        Instruction::SEI => registers.update_interrupt_flag(true),
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
    use crate::cpu::addressing_mode::OperandResolutionMock;
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
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, [0x42]))
                .returns(0x42),
        );
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::LDA,
            &operand_resolution,
            &[0x42],
        );
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
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(value),
        );
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::LDX,
            &operand_resolution,
            &[value],
        );
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
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(value),
        );
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::LDY,
            &operand_resolution,
            &[value],
        );
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
        let operand_resolution = Unimock::new(());
        registers.set_x(initial);
        execute_instruction(&mut registers, &mut memory, Instruction::INX, &operand_resolution, &[]);
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
        let operand_resolution = Unimock::new(());
        registers.set_y(initial);
        execute_instruction(&mut registers, &mut memory, Instruction::INY, &operand_resolution, &[]);
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
        let operand_resolution = Unimock::new(());
        registers.set_x(initial);
        execute_instruction(&mut registers, &mut memory, Instruction::DEX, &operand_resolution, &[]);
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
        let operand_resolution = Unimock::new(());
        registers.set_y(initial);
        execute_instruction(&mut registers, &mut memory, Instruction::DEY, &operand_resolution, &[]);
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
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(address),
        );
        memory.set_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, Instruction::INC, &operand_resolution, &[]);
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
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(address),
        );
        memory.set_byte(address, initial);
        execute_instruction(&mut registers, &mut memory, Instruction::DEC, &operand_resolution, &[]);
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    fn test_sta(mut registers: Registers, mut memory: Memory) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(0x0200u16),
        );
        registers.a = 0x42;
        execute_instruction(&mut registers, &mut memory, Instruction::STA, &operand_resolution, &[]);
        assert_eq!(memory.read_byte(0x0200), 0x42);
    }

    #[rstest]
    fn test_stx(mut registers: Registers, mut memory: Memory) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(0x0200u16),
        );
        registers.x = 0x42;
        execute_instruction(&mut registers, &mut memory, Instruction::STX, &operand_resolution, &[]);
        assert_eq!(memory.read_byte(0x0200), 0x42);
    }

    #[rstest]
    fn test_sty(mut registers: Registers, mut memory: Memory) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(0x0200u16),
        );
        registers.y = 0x42;
        execute_instruction(&mut registers, &mut memory, Instruction::STY, &operand_resolution, &[]);
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
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(operand),
        );
        registers.set_accumulator(accumulator);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::EOR,
            &operand_resolution,
            &[operand],
        );
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
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(operand),
        );
        registers.set_accumulator(accumulator);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::AND,
            &operand_resolution,
            &[operand],
        );
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
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(operand),
        );
        registers.set_accumulator(accumulator);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ORA,
            &operand_resolution,
            &[operand],
        );
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    fn test_jmp(mut registers: Registers, mut memory: Memory) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(0x1234u16),
        );
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::JMP,
            &operand_resolution,
            &[0x34, 0x12],
        );
        assert_eq!(registers.pc, 0x1234);
    }

    #[rstest]
    fn test_flag_clearing_and_setting(mut registers: Registers, mut memory: Memory) {
        let operand_resolution = Unimock::new(());

        // Start with all target flags set
        registers.status = CARRY_FLAG_BITMASK | DECIMAL_FLAG_BITMASK | INTERRUPT_FLAG_BITMASK | OVERFLOW_FLAG_BITMASK;

        execute_instruction(&mut registers, &mut memory, Instruction::CLC, &operand_resolution, &[]);
        assert!(!registers.is_flag_set(CARRY_FLAG_BITMASK), "CLC should clear carry");

        execute_instruction(&mut registers, &mut memory, Instruction::CLD, &operand_resolution, &[]);
        assert!(!registers.is_flag_set(DECIMAL_FLAG_BITMASK), "CLD should clear decimal");

        execute_instruction(&mut registers, &mut memory, Instruction::CLI, &operand_resolution, &[]);
        assert!(
            !registers.is_flag_set(INTERRUPT_FLAG_BITMASK),
            "CLI should clear interrupt"
        );

        execute_instruction(&mut registers, &mut memory, Instruction::CLV, &operand_resolution, &[]);
        assert!(
            !registers.is_flag_set(OVERFLOW_FLAG_BITMASK),
            "CLV should clear overflow"
        );

        execute_instruction(&mut registers, &mut memory, Instruction::SEC, &operand_resolution, &[]);
        assert!(registers.is_flag_set(CARRY_FLAG_BITMASK), "SEC should set carry");

        execute_instruction(&mut registers, &mut memory, Instruction::SED, &operand_resolution, &[]);
        assert!(registers.is_flag_set(DECIMAL_FLAG_BITMASK), "SED should set decimal");

        execute_instruction(&mut registers, &mut memory, Instruction::SEI, &operand_resolution, &[]);
        assert!(
            registers.is_flag_set(INTERRUPT_FLAG_BITMASK),
            "SEI should set interrupt"
        );
    }
}
