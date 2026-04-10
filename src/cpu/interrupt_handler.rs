use crate::{addressable::Addressable, cpu::registers::Registers};

#[cfg_attr(test, unimock::unimock(api=InterruptHandlerMock))]
pub trait InterruptHandler {
    fn handle_interrupt(&self, registers: &mut Registers, memory: &mut dyn Addressable);
}

pub(crate) struct DefaultInterruptHandler;

// TODO: NMI doesn't go to 0xFFFE/0xFFFF, it goes to 0xFFFA/0xFFFB. We need to distinguish between the two types of interrupt in order to handle this correctly.
impl InterruptHandler for DefaultInterruptHandler {
    fn handle_interrupt(&self, registers: &mut Registers, memory: &mut dyn Addressable) {
        let lo = memory.read_byte(0xFFFE) as u16;
        let hi = memory.read_byte(0xFFFF) as u16;
        registers.update_pc((hi << 8) | lo);
    }
}

#[cfg(test)]
mod tests {
    use crate::memory;

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(0x00, 0x00, 0x0000)]
    #[case(0x34, 0x12, 0x1234)]
    #[case(0xFF, 0xFF, 0xFFFF)]
    fn test_handle_interrupt_sets_pc(#[case] lo: u8, #[case] hi: u8, #[case] expected_pc: u16) {
        let handler = DefaultInterruptHandler;
        let mut registers = Registers::default();
        let mut memory = memory::default();
        memory[0xFFFE] = lo;
        memory[0xFFFF] = hi;

        handler.handle_interrupt(&mut registers, &mut memory);

        assert_eq!(registers.pc, expected_pc);
    }
}
