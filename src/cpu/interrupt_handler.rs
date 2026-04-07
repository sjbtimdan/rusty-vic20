use crate::cpu::registers::Registers;
use crate::memory::Memory;

#[cfg_attr(test, unimock::unimock(api=InterruptHandlerMock))]
pub trait InterruptHandler {
    fn handle_interrupt(&self, registers: &mut Registers, memory: &mut Memory);
}

pub(crate) struct DefaultInterruptHandler;

// TODO: NMI doesn't go to 0xFFFE/0xFFFF, it goes to 0xFFFA/0xFFFB. We need to distinguish between the two types of interrupt in order to handle this correctly.
impl InterruptHandler for DefaultInterruptHandler {
    fn handle_interrupt(&self, registers: &mut Registers, memory: &mut Memory) {
        let lo = memory.read_byte(0xFFFE) as u16;
        let hi = memory.read_byte(0xFFFF) as u16;
        registers.pc = (hi << 8) | lo;
    }
}
