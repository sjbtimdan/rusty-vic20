/// Trait for CPU breakpoints. Called every time a new instruction is fetched.
///
/// Implementations might log, pause execution, or trigger a debugger.
/// The `on_hit` method receives the PC address of the instruction about to execute.
pub trait Breakpoint {
    fn on_hit(&self, address: u16);
}

/// A simple breakpoint that prints to stdout when an address matches.
pub struct LoggingBreakpoint {
    address: u16,
}

impl LoggingBreakpoint {
    pub fn new(address: u16) -> Self {
        Self { address }
    }
}

impl Breakpoint for LoggingBreakpoint {
    fn on_hit(&self, address: u16) {
        if address == self.address {
            println!("Breakpoint hit: 0x{:04X}", address);
        }
    }
}
