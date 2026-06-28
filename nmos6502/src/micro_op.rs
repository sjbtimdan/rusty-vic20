/// One cycle of CPU execution: a bus operation plus an optional internal operation.
///
/// Every 6502 instruction decomposes into a static sequence of `MicroOp` values.
/// The `bus` field specifies what appears on the address/data bus this cycle.
/// The `internal` field specifies register/flag/ALU changes that happen concurrently.
#[derive(Debug, Clone, Copy)]
pub struct MicroOp {
    pub bus: BusOp,
    pub internal: InternalOp,
}

impl MicroOp {
    pub const fn new(bus: BusOp, internal: InternalOp) -> Self {
        Self { bus, internal }
    }
}

/// What appears on the address/data bus during one CPU cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum BusOp {
    /// Read opcode at PC (always cycle 1 of every instruction).
    Fetch,

    /// Read byte at PC+1 → operands[0].
    ReadPC1,
    /// Read byte at PC+2 → operands[1].
    ReadPC2,

    /// Read memory[addr] → data_latch.
    ReadAddr,
    /// Read memory[addr] — data discarded (dummy read for timing).
    ReadDummy,
    /// Read memory[zp_base] — dummy, then addr = (zp_base + X) & 0xFF.
    ReadDummyZpX,
    /// Read memory[zp_base] — dummy, then addr = (zp_base + Y) & 0xFF.
    ReadDummyZpY,

    /// Write A to memory[addr].
    WriteAddrA,
    /// Write X to memory[addr].
    WriteAddrX,
    /// Write Y to memory[addr].
    WriteAddrY,
    /// Write (A & X) to memory[addr] (SAX unofficial opcode).
    WriteAddrAX,
    /// Write A & X & ((addr >> 8) + 1) to memory[addr] (AHX/SHA unofficial opcode).
    WriteAddrAHX,
    /// Write data_latch to memory[addr].
    WriteAddrDL,
    /// Write data_latch to memory[addr] — RMW dummy write of original value.
    WriteDummy,

    /// Push PC high byte: write to 0x0100+SP, SP--.
    PushPCH,
    /// Push PC low byte: write to 0x0100+SP, SP--.
    PushPCL,
    /// Push return-address high byte (PC+2 >> 8) for JSR/BRK.
    PushReturnHi,
    /// Push return-address low byte (PC+2 & 0xFF) for JSR/BRK.
    PushReturnLo,
    /// Push accumulator: write A to 0x0100+SP, SP--.
    PushA,
    /// Push status register with B=1 and UNUSED=1 set (PHP, BRK).
    PushStatusB,
    /// Push status register with UNUSED=1 set, B=0 (NMI, IRQ).
    PushStatus,

    /// Dummy stack read: read 0x0100+SP (SP unchanged).
    PopDummy,
    /// Pop stack: SP++, read 0x0100+SP → data_latch.
    Pop,
    /// Pop stack: SP++, read 0x0100+SP — store as PC low byte directly.
    PopPCL,
    /// Pop stack: SP++, read 0x0100+SP — store as PC high byte directly.
    PopPCH,

    /// Read interrupt/reset vector low byte.
    ReadVecLo(u16),
    /// Read vector high byte; combine with stored low → PC.
    ReadVecHi(u16),

    /// No bus access (internal-only cycle: ALU, register transfer, etc.).
    None,
}

use crate::{cpu::CPU6502, memory::Addressable};

/// Function pointer type for internal CPU operations.
///
/// Each `InternalOp` variant (except the sequence-control ones) becomes a
/// standalone function with this signature, eliminating the central dispatch
/// match in `execute_internal()`.
type InternalOpFn = fn(&mut CPU6502, &mut dyn Addressable);

/// Internal operation performed concurrently with the bus operation.
///
/// The vast majority of variants carry no runtime data and are dispatched via
/// function pointer. The three `Skip*` variants carry a `u8` skip count that
/// the function-pointer pattern cannot express.
#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum InternalOp {
    /// Function-pointer-dispatched internal operation.
    Fn(InternalOpFn),

    // ── Sequence control (parameterized — stay as enum variants) ──
    /// If page_crossed is true, skip the next `n` MicroOps in the sequence.
    SkipIfCrossed(u8),
    /// If page_crossed is false, skip the next `n` MicroOps.
    SkipIfNotCrossed(u8),
    /// If branch_taken is false, skip the next `n` MicroOps.
    SkipIfNotTaken(u8),
}

impl std::fmt::Debug for InternalOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InternalOp::Fn(_) => f.write_str("Fn"),
            InternalOp::SkipIfCrossed(n) => write!(f, "SkipIfCrossed({})", n),
            InternalOp::SkipIfNotCrossed(n) => write!(f, "SkipIfNotCrossed({})", n),
            InternalOp::SkipIfNotTaken(n) => write!(f, "SkipIfNotTaken({})", n),
        }
    }
}
