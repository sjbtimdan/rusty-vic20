use std::time::Duration;

use crate::{
    // scheduled_operation::ScheduledOperation,
    addressable::Addressable,
    cpu::interrupt_handler::InterruptHandler,
    cpu::registers::Registers,
    scheduled_operation::ScheduledOperation,
};

pub struct VIA2 {
    scheduled_operation: ScheduledOperation,
}

impl Default for VIA2 {
    fn default() -> Self {
        let jiffy = (1000.0 / 60.0) as u64;
        Self {
            scheduled_operation: ScheduledOperation::new(
                Duration::from_millis(jiffy),
                crate::virtual_clock::SystemClock,
            ),
        }
    }
}

impl VIA2 {
    pub fn step(
        &mut self,
        registers: &mut Registers,
        memory: &mut dyn Addressable,
        interrupt_handler: &mut dyn InterruptHandler,
    ) {
        self.scheduled_operation.call_if_ready(|| {
            // interrupt_handler.handle_interrupt(registers, memory, false);
        });
    }
}
