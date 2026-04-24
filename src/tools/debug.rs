use log::info;

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
            info!("Breakpoint: 0x{:04X}", self.address);
        }
    }
}

pub struct MemoryWriteWatchpoint {
    address: Box<dyn Fn(u16) -> bool>,
}

impl MemoryWriteWatchpoint {
    pub fn watch_address(address: u16) -> MemoryWriteWatchpoint {
        MemoryWriteWatchpoint {
            address: Box::new(move |addr| addr == address),
        }
    }

    pub fn watch_address_range(start: u16, end: u16) -> MemoryWriteWatchpoint {
        MemoryWriteWatchpoint {
            address: Box::new(move |addr| addr >= start && addr <= end),
        }
    }

    pub fn on_write(&self, address: u16, value: u8) {
        if (self.address)(address) {
            println!("Watchpoint: Write of 0x{:02X} to 0x{:04X}", value, address);
        }
    }
}
