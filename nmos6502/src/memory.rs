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

    /// Convenience: read a zero-page byte (u8 address widened to u16).
    fn read_zp_byte(&mut self, address: u8) -> u8 {
        self.read_byte(address as u16)
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
