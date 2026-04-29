use crate::addressable::Addressable;

#[cfg_attr(test, unimock::unimock(api=InterruptHandlerMock))]
pub trait InterruptHandler {
    fn handle_interrupt(&self, memory: &mut dyn Addressable);
}
#[derive(Default)]
pub struct NoOpInterruptHandler;
impl InterruptHandler for NoOpInterruptHandler {
    fn handle_interrupt(&self, _memory: &mut dyn Addressable) {}
}
