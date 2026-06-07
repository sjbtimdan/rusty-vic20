use crate::via::VIA;

pub struct SerialPort;

impl SerialPort {
    pub fn step(&self, via1: &mut VIA) {
        via1.port_a_serial_direction();
    }
}
