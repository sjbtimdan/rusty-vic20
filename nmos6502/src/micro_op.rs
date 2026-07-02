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
    ReadPC1,
    ReadPC2,

    ReadAddr,
    /// Dummy ZP read, then addr = (zp_base + X) & 0xFF.
    ReadDummyZpX,
    /// Dummy ZP read, then addr = (zp_base + Y) & 0xFF.
    ReadDummyZpY,
    /// Read memory[addr + 1] with zero-page wrap (INDX hi-byte).
    ReadAddrZp1,
    /// Dummy read of next instruction byte (2-cycle implied ops).
    ReadDummyNext,
    /// Dummy read at PC-1 (RTS final cycle).
    ReadRTS,

    WriteAddrA,
    WriteAddrX,
    WriteAddrY,
    /// Write (A & X) — SAX.
    WriteAddrAX,
    /// Write A & X & ((addr >> 8) + 1) — AHX/SHA.
    WriteAddrAHX,
    /// Write Y & ((addr >> 8) + 1) — SHY.
    WriteAddrSHY,
    /// Write X & ((addr >> 8) + 1) — SHX.
    WriteAddrSHX,
    WriteDataLatch,

    PushPCH,
    PushPCL,
    PushA,
    /// Push status with B=1, UNUSED=1 (PHP, BRK).
    PushStatusB,
    /// Push status with UNUSED=1, B=0 (NMI, IRQ).
    PushStatus,

    /// Dummy stack read (SP unchanged).
    PopDummy,
    Pop,
    PopPCL,

    ReadVecLo(u16),
    ReadVecHi(u16),
}

pub type InternalOp = fn(&mut CPU6502);
