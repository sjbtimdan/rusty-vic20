use crate::cpu::registers::Registers;
use crate::memory::Memory;

#[cfg_attr(test, unimock::unimock(api=InterruptHandlerMock))]
pub trait InterruptHandler {
    fn handle_interrupt(&mut self, registers: &mut Registers, memory: &mut Memory);
}
