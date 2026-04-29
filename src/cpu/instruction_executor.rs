use crate::{
    addressable::Addressable,
    cpu::{
        addressing_mode::OperandResolution,
        instructions::Instruction,
        interrupt_handler::InterruptHandler,
        registers::{
            BREAK_FLAG_BITMASK, CARRY_FLAG_BITMASK, DECIMAL_FLAG_BITMASK, INTERRUPT_FLAG_BITMASK,
            NEGATIVE_FLAG_BITMASK, OVERFLOW_FLAG_BITMASK, Registers, UNUSED_FLAG_BITMASK, ZERO_FLAG_BITMASK,
        },
    },
};

pub trait InstructionExecutor {
    fn execute_instruction(
        &self,
        registers: &mut Registers,
        memory: &mut dyn Addressable,
        instruction: Instruction,
        operand_resolution: &dyn OperandResolution,
        operands: &[u8],
        interrupt_handler: &dyn InterruptHandler,
    ) -> bool;
}

pub struct DefaultInstructionExecutor;

impl InstructionExecutor for DefaultInstructionExecutor {
    fn execute_instruction(
        &self,
        registers: &mut Registers,
        memory: &mut dyn Addressable,
        instruction: Instruction,
        operand_resolution: &dyn OperandResolution,
        operands: &[u8],
        interrupt_handler: &dyn InterruptHandler,
    ) -> bool {
        execute_instruction(
            registers,
            memory,
            instruction,
            operand_resolution,
            operands,
            interrupt_handler,
        )
    }
}

fn execute_instruction(
    registers: &mut Registers,
    memory: &mut dyn Addressable,
    instruction: Instruction,
    operand_resolution: &dyn OperandResolution,
    operands: &[u8],
    interrupt_handler: &dyn InterruptHandler,
) -> bool {
    match instruction {
        Instruction::ADC => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            adc(registers, value);
        }
        Instruction::AND => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_accumulator(registers.a & value);
        }
        Instruction::ASL => {
            apply_shift(registers, memory, operand_resolution, operands, |v| {
                (v << 1, v & 0x80 != 0)
            });
        }
        Instruction::BCC => branch_if(registers, operands, !registers.is_flag_set(CARRY_FLAG_BITMASK)),
        Instruction::BCS => branch_if(registers, operands, registers.is_flag_set(CARRY_FLAG_BITMASK)),
        Instruction::BEQ => branch_if(registers, operands, registers.is_flag_set(ZERO_FLAG_BITMASK)),
        Instruction::BRK => {
            // Push return address (PC+2, skipping the signature byte)
            stack_push_u16(registers, memory, registers.pc.wrapping_add(2));
            // Push status with B and unused flags set (not stored in live status)
            let status_to_push = registers.status | BREAK_FLAG_BITMASK | UNUSED_FLAG_BITMASK;
            stack_push(registers, memory, status_to_push);
            // Set interrupt disable flag
            registers.set_flag(INTERRUPT_FLAG_BITMASK, true);
            // Dispatch to IRQ/BRK vector
            interrupt_handler.handle_interrupt(memory);
        }
        Instruction::BIT => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_flag(ZERO_FLAG_BITMASK, registers.a & value == 0);
            registers.set_flag(OVERFLOW_FLAG_BITMASK, value & 0x40 != 0);
            registers.set_flag(NEGATIVE_FLAG_BITMASK, value & 0x80 != 0);
        }
        Instruction::BMI => branch_if(registers, operands, registers.is_flag_set(NEGATIVE_FLAG_BITMASK)),
        Instruction::BNE => branch_if(registers, operands, !registers.is_flag_set(ZERO_FLAG_BITMASK)),
        Instruction::BPL => branch_if(registers, operands, !registers.is_flag_set(NEGATIVE_FLAG_BITMASK)),
        Instruction::BVC => branch_if(registers, operands, !registers.is_flag_set(OVERFLOW_FLAG_BITMASK)),
        Instruction::BVS => branch_if(registers, operands, registers.is_flag_set(OVERFLOW_FLAG_BITMASK)),
        Instruction::CLC => registers.update_carry_flag(false),
        Instruction::CLD => registers.update_decimal_flag(false),
        Instruction::CLI => registers.update_interrupt_flag(false),
        Instruction::CLV => registers.update_overflow_flag(false),
        Instruction::CMP => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            compare(registers, registers.a, value);
        }
        Instruction::CPX => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            compare(registers, registers.x, value);
        }
        Instruction::CPY => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            compare(registers, registers.y, value);
        }
        Instruction::DEC => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            let value = memory.read_byte(address).wrapping_sub(1);
            memory.write_byte(address, value);
            registers.update_zero_and_negative(value);
        }
        Instruction::DEX => {
            registers.set_x(registers.x.wrapping_sub(1));
        }
        Instruction::DEY => {
            registers.set_y(registers.y.wrapping_sub(1));
        }
        Instruction::EOR => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_accumulator(registers.a ^ value);
        }
        Instruction::INC => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            let value = memory.read_byte(address).wrapping_add(1);
            memory.write_byte(address, value);
            registers.update_zero_and_negative(value);
        }
        Instruction::INX => {
            registers.set_x(registers.x.wrapping_add(1));
        }
        Instruction::INY => {
            registers.set_y(registers.y.wrapping_add(1));
        }
        Instruction::JMP => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            registers.pc = address;
            return false;
        }
        Instruction::JSR => {
            let target = operand_resolution.resolve_address(registers, memory, operands);
            stack_push_u16(registers, memory, registers.pc.wrapping_add(2));
            registers.pc = target;
            return false;
        }
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
        Instruction::LSR => {
            apply_shift(registers, memory, operand_resolution, operands, |v| {
                (v >> 1, v & 0x01 != 0)
            });
        }
        Instruction::NOP => {}
        Instruction::ORA => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            registers.set_accumulator(registers.a | value);
        }
        Instruction::PHA => {
            stack_push(registers, memory, registers.a);
        }
        Instruction::PHP => {
            // Bits 4 (B) and 5 (unused) are always set when pushing via PHP
            stack_push(
                registers,
                memory,
                registers.status | UNUSED_FLAG_BITMASK | BREAK_FLAG_BITMASK,
            );
        }
        Instruction::PLA => {
            let value = stack_pull(registers, memory);
            registers.set_accumulator(value);
        }
        Instruction::PLP => {
            let value = stack_pull(registers, memory);
            // Bit 5 (unused) is always 1; bit 4 (B) is not a real CPU flag
            registers.status = (value | UNUSED_FLAG_BITMASK) & !BREAK_FLAG_BITMASK;
        }
        Instruction::ROL => {
            let old_carry = registers.is_flag_set(CARRY_FLAG_BITMASK) as u8;
            apply_shift(registers, memory, operand_resolution, operands, |v| {
                ((v << 1) | old_carry, v & 0x80 != 0)
            });
        }
        Instruction::ROR => {
            let old_carry = registers.is_flag_set(CARRY_FLAG_BITMASK) as u8;
            apply_shift(registers, memory, operand_resolution, operands, |v| {
                ((v >> 1) | (old_carry << 7), v & 0x01 != 0)
            });
        }
        Instruction::RTI => {
            let status = stack_pull(registers, memory);
            registers.status = (status | UNUSED_FLAG_BITMASK) & !BREAK_FLAG_BITMASK;
            let lo = stack_pull(registers, memory);
            let hi = stack_pull(registers, memory);
            registers.pc = (hi as u16) << 8 | lo as u16;
            return false;
        }
        Instruction::RTS => {
            let lo = stack_pull(registers, memory);
            let hi = stack_pull(registers, memory);
            registers.pc = ((hi as u16) << 8 | lo as u16).wrapping_add(1);
            return false;
        }
        Instruction::SBC => {
            let value = operand_resolution.resolve_value(registers, memory, operands);
            sbc(registers, value);
        }
        Instruction::SEC => registers.update_carry_flag(true),
        Instruction::SED => registers.update_decimal_flag(true),
        Instruction::SEI => registers.update_interrupt_flag(true),
        Instruction::STA => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            memory.write_byte(address, registers.a);
        }
        Instruction::STX => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            memory.write_byte(address, registers.x);
        }
        Instruction::STY => {
            let address = operand_resolution.resolve_address(registers, memory, operands);
            memory.write_byte(address, registers.y);
        }
        Instruction::TAX => registers.set_x(registers.a),
        Instruction::TAY => registers.set_y(registers.a),
        Instruction::TSX => registers.set_x(registers.sp),
        Instruction::TXA => registers.set_accumulator(registers.x),
        Instruction::TXS => registers.sp = registers.x,
        Instruction::TYA => registers.set_accumulator(registers.y),
        _ => unimplemented!("Instruction {:?} not implemented yet", instruction),
    }
    true
}

fn adc(registers: &mut Registers, value: u8) {
    let carry_in = registers.is_flag_set(CARRY_FLAG_BITMASK) as u8;
    if registers.is_flag_set(DECIMAL_FLAG_BITMASK) {
        // NMOS 6502 quirk: N, V and Z are set from the binary result.
        let bin_result = (registers.a as u16) + (value as u16) + (carry_in as u16);
        let bin_byte = bin_result as u8;
        let overflow = (!(registers.a ^ value) & (registers.a ^ bin_byte) & 0x80) != 0;
        registers.update_overflow_flag(overflow);
        registers.update_zero_and_negative(bin_byte);

        let lo = (registers.a & 0x0F) + (value & 0x0F) + carry_in;
        let lo_carry = if lo >= 10 { 1u8 } else { 0u8 };
        let lo = if lo >= 10 { lo + 6 } else { lo };

        let hi = (registers.a >> 4) + (value >> 4) + lo_carry;
        let carry_out = hi >= 10;
        let hi = if hi >= 10 { hi + 6 } else { hi };

        registers.update_carry_flag(carry_out);
        registers.a = ((hi & 0x0F) << 4) | (lo & 0x0F);
    } else {
        let result = (registers.a as u16) + (value as u16) + (carry_in as u16);
        let result_byte = result as u8;
        let overflow = (!(registers.a ^ value) & (registers.a ^ result_byte) & 0x80) != 0;
        registers.update_overflow_flag(overflow);
        registers.update_carry_flag(result > 0xFF);
        registers.set_accumulator(result_byte);
    }
}

fn sbc(registers: &mut Registers, value: u8) {
    let carry_in = registers.is_flag_set(CARRY_FLAG_BITMASK) as u8;
    if registers.is_flag_set(DECIMAL_FLAG_BITMASK) {
        // NMOS 6502 quirk: N, V and Z are set from the binary result (using !value).
        let effective = !value;
        let bin_result = (registers.a as u16) + (effective as u16) + (carry_in as u16);
        let bin_byte = bin_result as u8;
        let overflow = (!(registers.a ^ effective) & (registers.a ^ bin_byte) & 0x80) != 0;
        registers.update_overflow_flag(overflow);
        registers.update_zero_and_negative(bin_byte);

        let borrow = 1 - carry_in;
        let lo = (registers.a & 0x0F) as i16 - (value & 0x0F) as i16 - borrow as i16;
        let lo_borrow = if lo < 0 { 1i16 } else { 0i16 };
        let lo = if lo < 0 { lo - 6 } else { lo };

        let hi = (registers.a >> 4) as i16 - (value >> 4) as i16 - lo_borrow;
        let carry_out = hi >= 0;
        let hi = if hi < 0 { hi - 6 } else { hi };

        registers.update_carry_flag(carry_out);
        registers.a = ((hi as u8 & 0x0F) << 4) | (lo as u8 & 0x0F);
    } else {
        // Binary SBC: A - operand - borrow = A + !operand + C
        let effective = !value;
        let result = (registers.a as u16) + (effective as u16) + (carry_in as u16);
        let result_byte = result as u8;
        let overflow = (!(registers.a ^ effective) & (registers.a ^ result_byte) & 0x80) != 0;
        registers.update_overflow_flag(overflow);
        registers.update_carry_flag(result > 0xFF);
        registers.set_accumulator(result_byte);
    }
}

fn branch_if(registers: &mut Registers, operands: &[u8], condition: bool) {
    if condition {
        let offset = operands[0] as i8;
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

fn compare(registers: &mut Registers, reg: u8, value: u8) {
    let result = reg.wrapping_sub(value);
    registers.update_carry_flag(reg >= value);
    registers.update_zero_and_negative(result);
}

fn stack_push(registers: &mut Registers, memory: &mut dyn Addressable, value: u8) {
    memory.write_byte(0x0100 + registers.sp as u16, value);
    registers.sp = registers.sp.wrapping_sub(1);
}

fn stack_push_u16(registers: &mut Registers, memory: &mut dyn Addressable, value: u16) {
    stack_push(registers, memory, (value >> 8) as u8);
    stack_push(registers, memory, value as u8);
}

fn stack_pull(registers: &mut Registers, memory: &mut dyn Addressable) -> u8 {
    registers.sp = registers.sp.wrapping_add(1);
    memory.read_byte(0x0100 + registers.sp as u16)
}

fn apply_shift(
    registers: &mut Registers,
    memory: &mut dyn Addressable,
    operand_resolution: &dyn OperandResolution,
    operands: &[u8],
    compute: impl Fn(u8) -> (u8, bool),
) {
    if operand_resolution.is_accumulator() {
        let value = registers.a;
        let (result, new_carry) = compute(value);
        registers.update_carry_flag(new_carry);
        registers.set_accumulator(result);
    } else {
        let address = operand_resolution.resolve_address(registers, memory, operands);
        let value = memory.read_byte(address);
        let (result, new_carry) = compute(value);
        memory.write_byte(address, result);
        registers.update_carry_flag(new_carry);
        registers.update_zero_and_negative(result);
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
    use crate::cpu::interrupt_handler::{InterruptHandlerMock, NoOpInterruptHandler};
    use crate::cpu::registers::*;
    use crate::memory::Memory;
    use unimock::Unimock;

    #[fixture]
    fn registers() -> Registers {
        Registers::default()
    }

    #[fixture]
    fn memory() -> Memory {
        crate::memory::default()
    }

    // adc_binary: (a, operand, carry_in, expected, carry, overflow, zero, negative)
    #[rstest]
    #[case(0x01, 0x01, false, 0x02, false, false, false, false)] // basic
    #[case(0xFF, 0x01, false, 0x00, true, false, true, false)] // carry out
    #[case(0x01, 0x01, true, 0x03, false, false, false, false)] // carry in
    #[case(0x50, 0x50, false, 0xA0, false, true, false, true)] // pos+pos overflow
    #[case(0xD0, 0x90, false, 0x60, true, true, false, false)] // neg+neg overflow
    #[case(0x50, 0x10, false, 0x60, false, false, false, false)] // no overflow
    fn test_adc_binary(
        mut registers: Registers,
        mut memory: Memory,
        #[case] a: u8,
        #[case] operand: u8,
        #[case] carry_in: bool,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] overflow: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(operand),
        );
        registers.a = a;
        registers.update_carry_flag(carry_in);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ADC,
            &operand_resolution,
            &[operand],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(OVERFLOW_FLAG_BITMASK), overflow, "overflow flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    // adc_decimal: (a, operand, carry_in, expected_bcd, carry)
    #[rstest]
    #[case(0x15, 0x27, false, 0x42, false)] // 15+27=42
    #[case(0x99, 0x01, false, 0x00, true)] // 99+01=100, carry out
    #[case(0x19, 0x19, true, 0x39, false)] // 19+19+1=39
    #[case(0x09, 0x01, false, 0x10, false)] // low nibble BCD adjust
    #[case(0x58, 0x46, false, 0x04, true)] // 58+46=104, carry out
    fn test_adc_decimal(
        mut registers: Registers,
        mut memory: Memory,
        #[case] a: u8,
        #[case] operand: u8,
        #[case] carry_in: bool,
        #[case] expected_bcd: u8,
        #[case] carry: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(operand),
        );
        registers.a = a;
        registers.update_carry_flag(carry_in);
        registers.update_decimal_flag(true);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ADC,
            &operand_resolution,
            &[operand],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, expected_bcd);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
    }

    // sbc_binary: (a, operand, carry_in, expected, carry, overflow, zero, negative)
    // carry_in=1 means no borrow; carry_in=0 means borrow
    #[rstest]
    #[case(0x50, 0x30, true, 0x20, true, false, false, false)] // 80-48 = 32
    #[case(0x50, 0x30, false, 0x1F, true, false, false, false)] // 80-48-1 = 31
    #[case(0x50, 0xB0, true, 0xA0, false, true, false, true)] // +80-(-80)=+160 overflow
    #[case(0x00, 0x01, true, 0xFF, false, false, false, true)] // 0-1 borrow
    #[case(0x80, 0x01, true, 0x7F, true, true, false, false)] // -128-1 = -129 overflow
    fn test_sbc_binary(
        mut registers: Registers,
        mut memory: Memory,
        #[case] a: u8,
        #[case] operand: u8,
        #[case] carry_in: bool,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] overflow: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(operand),
        );
        registers.a = a;
        registers.update_carry_flag(carry_in);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::SBC,
            &operand_resolution,
            &[operand],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(OVERFLOW_FLAG_BITMASK), overflow, "overflow flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    // sbc_decimal: (a, operand, carry_in, expected_bcd, carry)
    #[rstest]
    #[case(0x42, 0x27, true, 0x15, true)] // 42-27=15
    #[case(0x40, 0x41, true, 0x99, false)] // 40-41=99 with borrow out
    #[case(0x20, 0x10, false, 0x09, true)] // 20-10-1=9
    fn test_sbc_decimal(
        mut registers: Registers,
        mut memory: Memory,
        #[case] a: u8,
        #[case] operand: u8,
        #[case] carry_in: bool,
        #[case] expected_bcd: u8,
        #[case] carry: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(operand),
        );
        registers.a = a;
        registers.update_carry_flag(carry_in);
        registers.update_decimal_flag(true);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::SBC,
            &operand_resolution,
            &[operand],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, expected_bcd);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
    }

    #[rstest]
    fn test_nop(mut registers: Registers, mut memory: Memory) {
        let operand_resolution = Unimock::new(());
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::NOP,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
    }

    #[rstest]
    #[case(0x0F, 0xC0, true, true, true)]
    #[case(0x40, 0x40, false, true, false)]
    #[case(0x80, 0x80, false, false, true)]
    #[case(0xFF, 0x00, true, false, false)]
    fn test_bit(
        mut registers: Registers,
        mut memory: Memory,
        #[case] a: u8,
        #[case] operand: u8,
        #[case] zero: bool,
        #[case] overflow: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_value
                .each_call(matching!(_, _, _))
                .returns(operand),
        );
        registers.a = a;
        registers.status = ZERO_FLAG_BITMASK | OVERFLOW_FLAG_BITMASK | NEGATIVE_FLAG_BITMASK;

        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::BIT,
            &operand_resolution,
            &[operand],
            &NoOpInterruptHandler,
        );

        assert_eq!(registers.a, a, "BIT should not modify the accumulator");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(OVERFLOW_FLAG_BITMASK), overflow, "overflow flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    // transfer: (instruction, src_value, expected_dest, zero, negative)
    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_tax(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(());
        registers.a = value;
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::TAX,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_tay(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(());
        registers.a = value;
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::TAY,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.y, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_txa(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(());
        registers.x = value;
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::TXA,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_tya(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(());
        registers.y = value;
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::TYA,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_tsx(
        mut registers: Registers,
        mut memory: Memory,
        #[case] value: u8,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(());
        registers.sp = value;
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::TSX,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    fn test_txs(mut registers: Registers, mut memory: Memory) {
        let operand_resolution = Unimock::new(());
        registers.x = 0x42;
        registers.status = 0x00;
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::TXS,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.sp, 0x42);
        assert_eq!(registers.status, 0x00, "TXS must not change flags");
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
            &NoOpInterruptHandler,
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
            &NoOpInterruptHandler,
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
            &NoOpInterruptHandler,
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
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::INX,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::INY,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::DEX,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::DEY,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
        memory.write_byte(address, initial);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::INC,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
        memory.write_byte(address, initial);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::DEC,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::STA,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::STX,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::STY,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
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
            &NoOpInterruptHandler,
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
            &NoOpInterruptHandler,
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
            &NoOpInterruptHandler,
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
        let increment_pc = execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::JMP,
            &operand_resolution,
            &[0x34, 0x12],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.pc, 0x1234);
        assert!(!increment_pc, "JSR should not indicate PC increment");
    }

    #[rstest]
    fn test_jsr(mut registers: Registers, mut memory: Memory) {
        registers.pc = 0x1200;
        registers.sp = 0xFF;
        let operand_resolution = Unimock::new(
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(0xC000u16),
        );
        let increment_pc = execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::JSR,
            &operand_resolution,
            &[0x00, 0xC0],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.pc, 0xC000, "pc should jump to target");
        assert_eq!(registers.sp, 0xFD, "sp should decrease by 2");
        assert_eq!(memory.read_byte(0x01FF), 0x12, "high byte of return address on stack");
        assert_eq!(memory.read_byte(0x01FE), 0x02, "low byte of return address on stack");
        assert!(!increment_pc, "JSR should not indicate PC increment");
    }

    #[rstest]
    fn test_rts(mut registers: Registers, mut memory: Memory) {
        // Push return address 0x1202 (as JSR would), then RTS should land at 0x1203
        registers.sp = 0xFF;
        stack_push_u16(&mut registers, &mut memory, 0x1202);
        let operand_resolution = Unimock::new(());
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::RTS,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.pc, 0x1203, "RTS should restore PC to return address + 1");
        assert_eq!(registers.sp, 0xFF, "SP should be restored to original value");
    }

    #[rstest]
    fn test_rti(mut registers: Registers, mut memory: Memory) {
        // Simulate what BRK pushes: PC then status (with B and unused set)
        registers.sp = 0xFF;
        let saved_pc: u16 = 0xC000;
        let saved_status = CARRY_FLAG_BITMASK | ZERO_FLAG_BITMASK | BREAK_FLAG_BITMASK | UNUSED_FLAG_BITMASK;
        stack_push_u16(&mut registers, &mut memory, saved_pc);
        stack_push(&mut registers, &mut memory, saved_status);
        let operand_resolution = Unimock::new(());
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::RTI,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.pc, saved_pc, "RTI should restore PC exactly (no +1)");
        assert_eq!(registers.sp, 0xFF, "SP should be restored to original value");
        assert!(
            registers.is_flag_set(CARRY_FLAG_BITMASK),
            "carry flag should be restored"
        );
        assert!(registers.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be restored");
        assert!(
            registers.is_flag_set(UNUSED_FLAG_BITMASK),
            "unused flag should always be set"
        );
        assert!(
            !registers.is_flag_set(BREAK_FLAG_BITMASK),
            "B flag should be cleared (not a real CPU flag)"
        );
    }

    #[rstest]
    fn test_brk(mut registers: Registers, mut memory: Memory) {
        registers.pc = 0x1200;
        registers.sp = 0xFF;
        registers.status = CARRY_FLAG_BITMASK | ZERO_FLAG_BITMASK; // some initial status
        let operand_resolution = Unimock::new(());
        let interrupt_handler = Unimock::new(
            InterruptHandlerMock::handle_interrupt
                .each_call(matching!(_))
                .returns(()),
        );
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::BRK,
            &operand_resolution,
            &[],
            &interrupt_handler,
        );
        // Return address PC+2 on stack (high byte first)
        assert_eq!(memory.read_byte(0x01FF), 0x12, "high byte of return address");
        assert_eq!(memory.read_byte(0x01FE), 0x02, "low byte of return address");
        // Status pushed with B (0x10) and unused (0x20) set
        assert_eq!(
            memory.read_byte(0x01FD),
            CARRY_FLAG_BITMASK | ZERO_FLAG_BITMASK | BREAK_FLAG_BITMASK | UNUSED_FLAG_BITMASK,
            "pushed status must have B and unused flags set"
        );
        assert_eq!(registers.sp, 0xFC, "SP should decrease by 3");
        assert!(
            registers.is_flag_set(INTERRUPT_FLAG_BITMASK),
            "I flag should be set after BRK"
        );
    }

    #[rstest]
    fn test_flag_clearing_and_setting(mut registers: Registers, mut memory: Memory) {
        let operand_resolution = Unimock::new(());

        // Start with all target flags set
        registers.status = CARRY_FLAG_BITMASK | DECIMAL_FLAG_BITMASK | INTERRUPT_FLAG_BITMASK | OVERFLOW_FLAG_BITMASK;

        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::CLC,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert!(!registers.is_flag_set(CARRY_FLAG_BITMASK), "CLC should clear carry");

        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::CLD,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert!(!registers.is_flag_set(DECIMAL_FLAG_BITMASK), "CLD should clear decimal");

        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::CLI,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert!(
            !registers.is_flag_set(INTERRUPT_FLAG_BITMASK),
            "CLI should clear interrupt"
        );

        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::CLV,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert!(
            !registers.is_flag_set(OVERFLOW_FLAG_BITMASK),
            "CLV should clear overflow"
        );

        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::SEC,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert!(registers.is_flag_set(CARRY_FLAG_BITMASK), "SEC should set carry");

        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::SED,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert!(registers.is_flag_set(DECIMAL_FLAG_BITMASK), "SED should set decimal");

        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::SEI,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert!(
            registers.is_flag_set(INTERRUPT_FLAG_BITMASK),
            "SEI should set interrupt"
        );
    }

    #[rstest]
    #[case(0x80, 0x00, true, true, false)]
    #[case(0x01, 0x02, false, false, false)]
    #[case(0x40, 0x80, false, false, true)]
    fn test_asl_accumulator(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::is_accumulator
                .each_call(matching!())
                .returns(true),
        );
        registers.a = initial;
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ASL,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x01, 0x00, true, true, false)]
    #[case(0x02, 0x01, false, false, false)]
    #[case(0x80, 0x40, false, false, false)]
    #[case(0x00, 0x00, false, true, false)]
    fn test_lsr_accumulator(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::is_accumulator
                .each_call(matching!())
                .returns(true),
        );
        registers.a = initial;
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::LSR,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x1234u16, 0x01, 0x00, true, true, false)]
    #[case(0x1234u16, 0x02, 0x01, false, false, false)]
    #[case(0x1234u16, 0x80, 0x40, false, false, false)]
    #[case(0x1234u16, 0x00, 0x00, false, true, false)]
    fn test_lsr_memory(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.write_byte(address, initial);
        let operand_resolution = Unimock::new((
            OperandResolutionMock::is_accumulator
                .each_call(matching!())
                .returns(false),
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(address),
        ));
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::LSR,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x80, false, 0x00, true, true, false)]
    #[case(0x80, true, 0x01, true, false, false)]
    #[case(0x01, false, 0x02, false, false, false)]
    #[case(0x40, false, 0x80, false, false, true)]
    fn test_rol_accumulator(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] carry_in: bool,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::is_accumulator
                .each_call(matching!())
                .returns(true),
        );
        registers.a = initial;
        registers.update_carry_flag(carry_in);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ROL,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x0200u16, 0x80, false, 0x00, true, true, false)]
    #[case(0x0200u16, 0x80, true, 0x01, true, false, false)]
    #[case(0x0200u16, 0x01, false, 0x02, false, false, false)]
    #[case(0x0200u16, 0x40, false, 0x80, false, false, true)]
    fn test_rol_memory(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] carry_in: bool,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.write_byte(address, initial);
        registers.update_carry_flag(carry_in);
        let operand_resolution = Unimock::new((
            OperandResolutionMock::is_accumulator
                .each_call(matching!())
                .returns(false),
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(address),
        ));
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ROL,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x01, false, 0x00, true, true, false)]
    #[case(0x01, true, 0x80, true, false, true)]
    #[case(0x02, false, 0x01, false, false, false)]
    #[case(0x00, true, 0x80, false, false, true)]
    fn test_ror_accumulator(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] carry_in: bool,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        let operand_resolution = Unimock::new(
            OperandResolutionMock::is_accumulator
                .each_call(matching!())
                .returns(true),
        );
        registers.a = initial;
        registers.update_carry_flag(carry_in);
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ROR,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.a, expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    #[case(0x0200u16, 0x01, false, 0x00, true, true, false)]
    #[case(0x0200u16, 0x01, true, 0x80, true, false, true)]
    #[case(0x0200u16, 0x02, false, 0x01, false, false, false)]
    #[case(0x0200u16, 0x00, true, 0x80, false, false, true)]
    fn test_ror_memory(
        mut registers: Registers,
        mut memory: Memory,
        #[case] address: u16,
        #[case] initial: u8,
        #[case] carry_in: bool,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.write_byte(address, initial);
        registers.update_carry_flag(carry_in);
        let operand_resolution = Unimock::new((
            OperandResolutionMock::is_accumulator
                .each_call(matching!())
                .returns(false),
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(address),
        ));
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ROR,
            &operand_resolution,
            &[],
            &NoOpInterruptHandler,
        );
        assert_eq!(memory.read_byte(address), expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    // (instruction, flag_bitmask, flag_value_for_branch_taken)
    #[rstest]
    #[case(Instruction::BPL, NEGATIVE_FLAG_BITMASK, false)]
    #[case(Instruction::BMI, NEGATIVE_FLAG_BITMASK, true)]
    #[case(Instruction::BVC, OVERFLOW_FLAG_BITMASK, false)]
    #[case(Instruction::BVS, OVERFLOW_FLAG_BITMASK, true)]
    #[case(Instruction::BCC, CARRY_FLAG_BITMASK, false)]
    #[case(Instruction::BCS, CARRY_FLAG_BITMASK, true)]
    #[case(Instruction::BNE, ZERO_FLAG_BITMASK, false)]
    #[case(Instruction::BEQ, ZERO_FLAG_BITMASK, true)]
    fn test_branch_taken_forward(
        mut registers: Registers,
        mut memory: Memory,
        #[case] instruction: Instruction,
        #[case] flag: u8,
        #[case] flag_set: bool,
    ) {
        registers.pc = 0x0200;
        registers.set_flag(flag, flag_set);
        let operand_resolution = Unimock::new(());
        execute_instruction(
            &mut registers,
            &mut memory,
            instruction,
            &operand_resolution,
            &[0x10],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.pc, 0x0210);
    }

    #[rstest]
    #[case(Instruction::BPL, NEGATIVE_FLAG_BITMASK, false)]
    #[case(Instruction::BMI, NEGATIVE_FLAG_BITMASK, true)]
    #[case(Instruction::BVC, OVERFLOW_FLAG_BITMASK, false)]
    #[case(Instruction::BVS, OVERFLOW_FLAG_BITMASK, true)]
    #[case(Instruction::BCC, CARRY_FLAG_BITMASK, false)]
    #[case(Instruction::BCS, CARRY_FLAG_BITMASK, true)]
    #[case(Instruction::BNE, ZERO_FLAG_BITMASK, false)]
    #[case(Instruction::BEQ, ZERO_FLAG_BITMASK, true)]
    fn test_branch_taken_backward(
        mut registers: Registers,
        mut memory: Memory,
        #[case] instruction: Instruction,
        #[case] flag: u8,
        #[case] flag_set: bool,
    ) {
        registers.pc = 0x0210;
        registers.set_flag(flag, flag_set);
        let operand_resolution = Unimock::new(());
        // 0xF0 = -16i8
        execute_instruction(
            &mut registers,
            &mut memory,
            instruction,
            &operand_resolution,
            &[0xF0],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.pc, 0x0200);
    }

    #[rstest]
    #[case(Instruction::BPL, NEGATIVE_FLAG_BITMASK, true)]
    #[case(Instruction::BMI, NEGATIVE_FLAG_BITMASK, false)]
    #[case(Instruction::BVC, OVERFLOW_FLAG_BITMASK, true)]
    #[case(Instruction::BVS, OVERFLOW_FLAG_BITMASK, false)]
    #[case(Instruction::BCC, CARRY_FLAG_BITMASK, true)]
    #[case(Instruction::BCS, CARRY_FLAG_BITMASK, false)]
    #[case(Instruction::BNE, ZERO_FLAG_BITMASK, true)]
    #[case(Instruction::BEQ, ZERO_FLAG_BITMASK, false)]
    fn test_branch_not_taken(
        mut registers: Registers,
        mut memory: Memory,
        #[case] instruction: Instruction,
        #[case] flag: u8,
        #[case] flag_set: bool,
    ) {
        registers.pc = 0x0200;
        registers.set_flag(flag, flag_set);
        let operand_resolution = Unimock::new(());
        execute_instruction(
            &mut registers,
            &mut memory,
            instruction,
            &operand_resolution,
            &[0x10],
            &NoOpInterruptHandler,
        );
        assert_eq!(registers.pc, 0x0200);
    }

    #[rstest]
    #[case(0x80, 0x00, true, true, false)]
    #[case(0x01, 0x02, false, false, false)]
    #[case(0x40, 0x80, false, false, true)]
    fn test_asl_memory(
        mut registers: Registers,
        mut memory: Memory,
        #[case] initial: u8,
        #[case] expected: u8,
        #[case] carry: bool,
        #[case] zero: bool,
        #[case] negative: bool,
    ) {
        memory.write_byte(0x1234, initial);
        let operand_resolution = Unimock::new((
            OperandResolutionMock::is_accumulator
                .each_call(matching!())
                .returns(false),
            OperandResolutionMock::resolve_address
                .each_call(matching!(_, _, _))
                .returns(0x1234u16),
        ));
        execute_instruction(
            &mut registers,
            &mut memory,
            Instruction::ASL,
            &operand_resolution,
            &[0x34, 0x12],
            &NoOpInterruptHandler,
        );
        assert_eq!(memory.read_byte(0x1234), expected);
        assert_eq!(registers.is_flag_set(CARRY_FLAG_BITMASK), carry, "carry flag");
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }
}
