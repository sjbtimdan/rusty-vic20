use crate::{
    // scheduled_operation::ScheduledOperation,
    addressable::Addressable,
    cpu::cpu6502::CPU6502,
};

#[derive(Default)]
pub struct VIA2 {
    // scheduled_operation: ScheduledOperation<dyn FnMut()>,
}

// impl Default for VIA2 {
//     fn default() -> Self {
//         Self {
//             scheduled_operation: ScheduledOperation::new(|| {}),
//         }
//     }
// }

impl VIA2 {
    pub fn step(&mut self, _cpu: &mut CPU6502, _memory: &mut impl Addressable) {
        // cpu.interrupt(memory, interrupt_handler);
    }
}
