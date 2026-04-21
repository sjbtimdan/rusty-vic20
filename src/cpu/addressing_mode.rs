use crate::{addressable::Addressable, cpu::registers::Registers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
}

#[cfg_attr(test, unimock::unimock(api=OperandResolutionMock))]
pub trait OperandResolution {
    fn is_accumulator(&self) -> bool;
    fn resolve_value(&self, registers: &Registers, memory: &dyn Addressable, operands: &[u8]) -> u8;
    fn resolve_address(&self, registers: &Registers, memory: &dyn Addressable, operands: &[u8]) -> u16;
    fn crosses_page_boundary(&self, registers: &Registers, memory: &dyn Addressable, operands: &[u8]) -> bool;
}

impl AddressingMode {
    pub fn operand_count(&self) -> usize {
        match self {
            AddressingMode::Implied | AddressingMode::Accumulator => 0,
            AddressingMode::Immediate
            | AddressingMode::ZeroPage
            | AddressingMode::ZeroPageX
            | AddressingMode::ZeroPageY
            | AddressingMode::Relative
            | AddressingMode::IndexedIndirect
            | AddressingMode::IndirectIndexed => 1,
            AddressingMode::Absolute
            | AddressingMode::AbsoluteX
            | AddressingMode::AbsoluteY
            | AddressingMode::Indirect => 2,
        }
    }
}

impl OperandResolution for AddressingMode {
    fn is_accumulator(&self) -> bool {
        matches!(self, AddressingMode::Accumulator)
    }

    fn resolve_value(&self, registers: &Registers, memory: &dyn Addressable, operands: &[u8]) -> u8 {
        match self {
            AddressingMode::Immediate => operands[0],
            AddressingMode::ZeroPage => memory.read_zero_page_byte(operands[0]),
            AddressingMode::ZeroPageX => memory.read_zero_page_byte(operands[0].wrapping_add(registers.x)),
            AddressingMode::ZeroPageY => memory.read_zero_page_byte(operands[0].wrapping_add(registers.y)),
            AddressingMode::Absolute => {
                let address = (operands[1] as u16) << 8 | operands[0] as u16;
                memory.read_byte(address)
            }
            AddressingMode::AbsoluteX => {
                let address = ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.x as u16);
                memory.read_byte(address)
            }
            AddressingMode::AbsoluteY => {
                let address = ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.y as u16);
                memory.read_byte(address)
            }
            AddressingMode::IndexedIndirect => {
                let ptr = operands[0].wrapping_add(registers.x);
                let address = memory.read_zero_page_word(ptr);
                memory.read_byte(address)
            }
            AddressingMode::IndirectIndexed => {
                let base = memory.read_zero_page_word(operands[0]);
                let address = base.wrapping_add(registers.y as u16);
                memory.read_byte(address)
            }
            _ => unimplemented!("Addressing mode {:?} not implemented for load", self),
        }
    }

    fn resolve_address(&self, registers: &Registers, memory: &dyn Addressable, operands: &[u8]) -> u16 {
        match self {
            AddressingMode::ZeroPage => operands[0] as u16,
            AddressingMode::ZeroPageX => operands[0].wrapping_add(registers.x) as u16,
            AddressingMode::ZeroPageY => operands[0].wrapping_add(registers.y) as u16,
            AddressingMode::Absolute => (operands[1] as u16) << 8 | operands[0] as u16,
            AddressingMode::AbsoluteX => {
                ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.x as u16)
            }
            AddressingMode::AbsoluteY => {
                ((operands[1] as u16) << 8 | operands[0] as u16).wrapping_add(registers.y as u16)
            }
            AddressingMode::IndexedIndirect => {
                let ptr = operands[0].wrapping_add(registers.x);
                memory.read_zero_page_word(ptr)
            }
            AddressingMode::IndirectIndexed => {
                let base = memory.read_zero_page_word(operands[0]);
                base.wrapping_add(registers.y as u16)
            }
            AddressingMode::Indirect => {
                let ptr = (operands[1] as u16) << 8 | operands[0] as u16;
                memory.read_word(ptr)
            }
            _ => unimplemented!("Addressing mode {:?} not implemented for store", self),
        }
    }

    fn crosses_page_boundary(&self, registers: &Registers, memory: &dyn Addressable, operands: &[u8]) -> bool {
        match self {
            AddressingMode::AbsoluteX => {
                let base = (operands[1] as u16) << 8 | operands[0] as u16;
                let address = base.wrapping_add(registers.x as u16);
                (base & 0xFF00) != (address & 0xFF00)
            }
            AddressingMode::AbsoluteY => {
                let base = (operands[1] as u16) << 8 | operands[0] as u16;
                let address = base.wrapping_add(registers.y as u16);
                (base & 0xFF00) != (address & 0xFF00)
            }
            AddressingMode::IndexedIndirect => false,
            AddressingMode::IndirectIndexed => {
                let base = memory.read_zero_page_word(operands[0]);
                let address = base.wrapping_add(registers.y as u16);
                (base & 0xFF00) != (address & 0xFF00)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::fixture;
    use rstest::rstest;

    use super::*;
    use crate::{cpu::registers::Registers, memory::*};

    #[fixture]
    fn registers() -> Registers {
        Registers::default()
    }

    #[fixture]
    fn memory() -> Memory {
        crate::memory::default()
    }

    // resolve_load_operand

    #[rstest]
    fn test_load_immediate(registers: Registers, memory: Memory) {
        assert_eq!(
            AddressingMode::Immediate.resolve_value(&registers, &memory, &[0x42]),
            0x42
        );
    }

    #[rstest]
    fn test_load_zero_page(registers: Registers, mut memory: Memory) {
        memory.write_zero_page_byte(0x10, 0x42);
        assert_eq!(
            AddressingMode::ZeroPage.resolve_value(&registers, &memory, &[0x10]),
            0x42
        );
    }

    #[rstest]
    fn test_load_zero_page_x(mut registers: Registers, mut memory: Memory) {
        registers.set_x(0x05);
        memory.write_zero_page_byte(0x15, 0x42);
        assert_eq!(
            AddressingMode::ZeroPageX.resolve_value(&registers, &memory, &[0x10]),
            0x42
        );
    }

    #[rstest]
    fn test_load_zero_page_y(mut registers: Registers, mut memory: Memory) {
        registers.set_y(0x05);
        memory.write_zero_page_byte(0x15, 0x42);
        assert_eq!(
            AddressingMode::ZeroPageY.resolve_value(&registers, &memory, &[0x10]),
            0x42
        );
    }

    #[rstest]
    fn test_load_absolute(registers: Registers, mut memory: Memory) {
        memory.write_byte(0x0200, 0x42);
        assert_eq!(
            AddressingMode::Absolute.resolve_value(&registers, &memory, &[0x00, 0x02]),
            0x42
        );
    }

    #[rstest]
    fn test_load_absolute_x(mut registers: Registers, mut memory: Memory) {
        registers.set_x(0x04);
        memory.write_byte(0x0204, 0x42);
        assert_eq!(
            AddressingMode::AbsoluteX.resolve_value(&registers, &memory, &[0x00, 0x02]),
            0x42
        );
    }

    #[rstest]
    fn test_load_absolute_y(mut registers: Registers, mut memory: Memory) {
        registers.set_y(0x04);
        memory.write_byte(0x0204, 0x42);
        assert_eq!(
            AddressingMode::AbsoluteY.resolve_value(&registers, &memory, &[0x00, 0x02]),
            0x42
        );
    }

    #[rstest]
    fn test_load_indexed_indirect(mut registers: Registers, mut memory: Memory) {
        registers.set_x(0x03);
        memory.write_zero_page_word(0x13, 0x0200);
        memory.write_byte(0x0200, 0x42);
        assert_eq!(
            AddressingMode::IndexedIndirect.resolve_value(&registers, &memory, &[0x10]),
            0x42
        );
    }

    #[rstest]
    fn test_load_indirect_indexed(mut registers: Registers, mut memory: Memory) {
        registers.set_y(0x03);
        memory.write_zero_page_word(0x10, 0x01FD);
        memory.write_byte(0x0200, 0x42);
        assert_eq!(
            AddressingMode::IndirectIndexed.resolve_value(&registers, &memory, &[0x10]),
            0x42
        );
    }

    // resolve_store_address

    #[rstest]
    fn test_store_zero_page(registers: Registers, memory: Memory) {
        assert_eq!(
            AddressingMode::ZeroPage.resolve_address(&registers, &memory, &[0x10]),
            0x0010
        );
    }

    #[rstest]
    fn test_store_zero_page_x(mut registers: Registers, memory: Memory) {
        registers.set_x(0x05);
        assert_eq!(
            AddressingMode::ZeroPageX.resolve_address(&registers, &memory, &[0x10]),
            0x0015
        );
    }

    #[rstest]
    fn test_store_zero_page_y(mut registers: Registers, memory: Memory) {
        registers.set_y(0x05);
        assert_eq!(
            AddressingMode::ZeroPageY.resolve_address(&registers, &memory, &[0x10]),
            0x0015
        );
    }

    #[rstest]
    fn test_store_absolute(registers: Registers, memory: Memory) {
        assert_eq!(
            AddressingMode::Absolute.resolve_address(&registers, &memory, &[0x00, 0x02]),
            0x0200
        );
    }

    #[rstest]
    fn test_store_absolute_x(mut registers: Registers, memory: Memory) {
        registers.set_x(0x04);
        assert_eq!(
            AddressingMode::AbsoluteX.resolve_address(&registers, &memory, &[0x00, 0x02]),
            0x0204
        );
    }

    #[rstest]
    fn test_store_absolute_y(mut registers: Registers, memory: Memory) {
        registers.set_y(0x04);
        assert_eq!(
            AddressingMode::AbsoluteY.resolve_address(&registers, &memory, &[0x00, 0x02]),
            0x0204
        );
    }

    #[rstest]
    fn test_store_indexed_indirect(mut registers: Registers, mut memory: Memory) {
        registers.set_x(0x03);
        memory.write_zero_page_word(0x13, 0x0200);
        assert_eq!(
            AddressingMode::IndexedIndirect.resolve_address(&registers, &memory, &[0x10]),
            0x0200
        );
    }

    #[rstest]
    fn test_store_indirect_indexed(mut registers: Registers, mut memory: Memory) {
        registers.set_y(0x03);
        memory.write_zero_page_word(0x10, 0x01FD);
        assert_eq!(
            AddressingMode::IndirectIndexed.resolve_address(&registers, &memory, &[0x10]),
            0x0200
        );
    }
}
