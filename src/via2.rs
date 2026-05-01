use crate::{
    // scheduled_operation::ScheduledOperation,
    addressable::Addressable,
    bus::VIA2_REGISTERS_START,
    cpu::interrupt_handler::InterruptHandler,
    cpu::registers::Registers,
};

#[derive(Default)]
pub struct VIA2 {
    registers: [u8; 16],
}

impl VIA2 {
    pub fn step(
        &mut self,
        _registers: &mut Registers,
        _memory: &mut dyn Addressable,
        _interrupt_handler: &mut dyn InterruptHandler,
    ) {
    }
}

impl Addressable for VIA2 {
    fn read_byte(&self, address: u16) -> u8 {
        self.registers[address as usize - VIA2_REGISTERS_START as usize]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.registers[address as usize - VIA2_REGISTERS_START as usize] = value;
    }
}
