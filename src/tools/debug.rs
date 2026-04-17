pub trait Breakpoint {
    fn on_hit(&self, address: u16);
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
    fn on_hit(&self, address: u16) {
        if address == self.address {
            println!("Breakpoint: 0x{:04X}", self.address);
        }
    }
}

pub struct MemoryWriteWatchpoint {
    address: u16,
}

impl MemoryWriteWatchpoint {
    pub fn new(address: u16) -> MemoryWriteWatchpoint {
        MemoryWriteWatchpoint { address }
    }

    pub fn on_write(&self, address: u16, value: u8) {
        if address == self.address {
            println!("Watchpoint: Write of 0x{:02X} to 0x{:04X}", value, self.address);
        }
    }
}
