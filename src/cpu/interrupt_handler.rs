use crate::addressable::Addressable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interrupt {
    NMI,
    IRQ,
    BRK,
}

#[cfg_attr(test, unimock::unimock(api=InterruptHandlerMock))]
pub trait InterruptHandler {
    fn handle_interrupt(
        &mut self,
        registers: &mut crate::cpu::registers::Registers,
        memory: &mut dyn Addressable,
        interrupt: Interrupt,
    );
}
#[derive(Default)]
pub struct NoOpInterruptHandler;
impl InterruptHandler for NoOpInterruptHandler {
    fn handle_interrupt(
        &mut self,
        _registers: &mut crate::cpu::registers::Registers,
        _memory: &mut dyn Addressable,
        _interrupt: Interrupt,
    ) {
    }
}
