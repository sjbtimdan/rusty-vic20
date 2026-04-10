use crate::addressable::Addressable;

pub type Memory = [u8; 65536];

pub fn default() -> Memory {
    [0; 65536]
}

impl Addressable for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        self[address as usize]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self[address as usize] = value;
    }
}
