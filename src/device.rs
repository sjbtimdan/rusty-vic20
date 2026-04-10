pub trait Device {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

pub struct UnimplementedDevice;

impl Device for UnimplementedDevice {
    fn read(&self, _address: u16) -> u8 {
        0xFF
    }

    fn write(&mut self, _address: u16, _value: u8) {}
}
