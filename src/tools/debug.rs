use log::info;

pub trait Breakpoint {
    fn on_hit(&self, pc: u16);
}

pub struct LoggingAddressBreakpoint {
    address: u16,
}

impl LoggingAddressBreakpoint {
    pub fn new(address: u16) -> LoggingAddressBreakpoint {
        LoggingAddressBreakpoint { address }
    }
}

impl Breakpoint for LoggingAddressBreakpoint {
    fn on_hit(&self, pc: u16) {
        if pc == self.address {
            info!("Breakpoint: 0x{:04X}", self.address);
        }
    }
}
