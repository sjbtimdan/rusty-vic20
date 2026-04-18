pub trait Addressable {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);

    fn read_zero_page_byte(&self, address: u8) -> u8 {
        self.read_byte(address as u16)
    }

    fn write_zero_page_byte(&mut self, address: u8, value: u8) {
        self.write_byte(address as u16, value);
    }

    fn read_zero_page_word(&self, address: u8) -> u16 {
        self.read_word(address as u16)
    }

    fn write_zero_page_word(&mut self, address: u8, value: u16) {
        self.write_word(address as u16, value);
    }

    fn read_word(&self, address: u16) -> u16 {
        let low = self.read_byte(address) as u16;
        let high = self.read_byte(address.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    fn write_word(&mut self, address: u16, value: u16) {
        self.write_byte(address, (value & 0xFF) as u8);
        self.write_byte(address.wrapping_add(1), (value >> 8) as u8);
    }
}

#[cfg(test)]
impl std::fmt::Debug for dyn Addressable + '_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("dyn Addressable")
    }
}

pub struct UnimplementedAddressable;

impl Addressable for UnimplementedAddressable {
    fn read_byte(&self, _address: u16) -> u8 {
        0xFF
    }

    fn write_byte(&mut self, _address: u16, _value: u8) {}
}
