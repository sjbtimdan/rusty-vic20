use crate::cpu::CPU6502;

/// One cycle of CPU execution: a bus operation plus a concurrent internal operation.
///
/// Every 6502 instruction decomposes into a static sequence of `MicroOp` values.
/// The `bus` field specifies what appears on the address/data bus this cycle.
/// The `internal` field specifies register/flag/ALU changes that happen concurrently.
#[derive(Debug, Clone, Copy)]
pub struct MicroOp {
    pub bus: BusOp,
    pub internal: InternalOp,
}

/// What appears on the address/data bus during one CPU cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum BusOp {
    /// Read byte at PC+1 → operands[0].
    ReadPC1,
    /// Read byte at PC+2 → operands[1].
    ReadPC2,

    /// Read memory[addr] → data_latch.
    ReadAddr,
    /// Read memory[zp_base] — dummy, then addr = (zp_base + X) & 0xFF.
    ReadDummyZpX,
    /// Read memory[zp_base] — dummy, then addr = (zp_base + Y) & 0xFF.
    ReadDummyZpY,
    /// Read memory[addr + 1] with zero-page wrap (for INDX hi-byte read).
    ReadAddrZp1,
    /// Read next instruction byte (PC+1) as dummy read (for 2-cycle implied ops).
    ReadDummyNext,
    /// Read return address (PC - 1) for RTS final dummy cycle.
    ReadRTS,

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
    /// Write Y & ((addr >> 8) + 1) to memory[addr] (SHY unofficial opcode).
    WriteAddrSHY,
    /// Write X & ((addr >> 8) + 1) to memory[addr] (SHX unofficial opcode).
    WriteAddrSHX,
    /// Write data_latch to memory[addr].
    WriteDataLatch,

    /// Push PC high byte: write to 0x0100+SP, SP--.
    PushPCH,
    /// Push PC low byte: write to 0x0100+SP, SP--.
    PushPCL,

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

    /// Read interrupt/reset vector low byte.
    ReadVecLo(u16),
    /// Read vector high byte; combine with stored low → PC.
    ReadVecHi(u16),
}

pub type InternalOp = fn(&mut CPU6502);
