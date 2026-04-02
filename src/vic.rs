#[derive(Default)]
pub struct VIC {
    registers: [u8; 16],
}

impl VIC {
    pub fn border_colour(&self) -> u8 {
        self.registers[0xF] & 0x0F
    }

    pub fn set_border_color(&mut self, value: u8) {
        self.registers[0x0F] = value & 0x0F;
    }
}
