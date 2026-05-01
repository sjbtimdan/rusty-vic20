use crate::addressable::Addressable;

#[cfg_attr(test, unimock::unimock(api=InterruptHandlerMock))]
pub trait InterruptHandler {
    fn handle_interrupt(
        &mut self,
        registers: &mut crate::cpu::registers::Registers,
        memory: &mut dyn Addressable,
        is_break: bool,
    );
}
#[derive(Default)]
pub struct NoOpInterruptHandler;
impl InterruptHandler for NoOpInterruptHandler {
    fn handle_interrupt(
        &mut self,
        _registers: &mut crate::cpu::registers::Registers,
        _memory: &mut dyn Addressable,
        _is_break: bool,
    ) {
    }
}
