/// Memory bus interface.
///
/// On the VIC-20, the 6502 and VIC chip use opposite clock phases (φ2 for CPU, φ1 for VIC).
/// The VIC has no RDY output — the CPU is never stalled. Reads and writes always succeed.
///
/// `read_byte` takes `&mut self` because some hardware reads have side effects
/// (e.g., reading a VIA timer register clears its interrupt flag).
pub trait Addressable {
    fn read_byte(&mut self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);

    fn read_word(&mut self, address: u16) -> u16 {
        let lo = self.read_byte(address) as u16;
        let hi = self.read_byte(address.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    fn read_zp_byte(&mut self, address: u8) -> u8 {
        self.read_byte(address as u16)
    }

    fn read_zp_word(&mut self, address: u8) -> u16 {
        self.read_word(address as u16)
    }

    fn write_word(&mut self, address: u16, value: u16) {
        self.write_byte(address, value as u8);
        self.write_byte(address.wrapping_add(1), (value >> 8) as u8);
    }

    fn write_zp_byte(&mut self, address: u8, value: u8) {
        self.write_byte(address as u16, value);
    }
}

/// A simple RAM-only memory implementation for testing.
pub struct Ram([u8; 65536]);

impl Ram {
    pub fn new() -> Self {
        Self([0; 65536])
    }
}

impl Addressable for Ram {
    fn read_byte(&mut self, address: u16) -> u8 {
        self.0[address as usize]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.0[address as usize] = value;
    }
}

impl Default for Ram {
    fn default() -> Self {
        Self([0; 65536])
    }
}
