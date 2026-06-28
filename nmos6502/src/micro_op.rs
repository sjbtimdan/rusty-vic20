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

/// Internal operation performed concurrently with the bus operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum InternalOp {
    /// No internal state change.
    None,

    /// Decode opcode fetched by Fetch, load the instruction sequence.
    Decode,
    /// Instruction complete — advance PC, check for pending interrupts.
    EndInstr,

    // ── Address computation ──
    /// addr = operands[0] as u16 (zero-page mode).
    SetAddrZp,
    /// addr = (operands[0] + X) & 0xFF.
    SetAddrZpX,
    /// addr = (operands[0] + Y) & 0xFF.
    SetAddrZpY,
    /// addr = operands[1]<<8 | operands[0] (absolute mode).
    SetAddrAbs,
    /// addr = base + X; set page_crossed flag (absolute,X mode).
    SetAddrAbsX,
    /// addr = base + Y; set page_crossed flag (absolute,Y mode).
    SetAddrAbsY,
    /// addr = zp[(operands[0]+X)] | (zp[(operands[0]+X+1)] << 8)  (indexed indirect).
    SetAddrIndX,
    /// addr = zp[ptr] | (zp[ptr+1] << 8); addr += Y; set page_crossed (indirect indexed).
    SetAddrIndY,

    // ── Register operations (set Z,N from result) ──
    /// A = data_latch.
    SetA,
    /// X = data_latch.
    SetX,
    /// Y = data_latch.
    SetY,

    /// A = X (TXA).
    Txa,
    /// A = Y (TYA).
    Tya,
    /// X = A (TAX).
    Tax,
    /// Y = A (TAY).
    Tay,
    /// X = SP (TSX).
    Tsx,
    /// SP = X (TXS, no flag changes).
    Txs,

    /// X = X + 1.
    IncX,
    /// Y = Y + 1.
    IncY,
    /// X = X - 1.
    DecX,
    /// Y = Y - 1.
    DecY,

    // ── Flag operations ──
    SetC,
    ClrC,
    SetD,
    ClrD,
    SetI,
    ClrI,
    ClrV,

    // ── ALU operations (operate on A and data_latch, or data_latch alone) ──
    /// A = A + data_latch + C; sets C,Z,N,V (binary or BCD based on D flag).
    Adc,
    /// A = A - data_latch - (1-C); sets C,Z,N,V.
    Sbc,
    /// A = A & data_latch; sets Z,N.
    And,
    /// A = A | data_latch; sets Z,N.
    Ora,
    /// A = A ^ data_latch; sets Z,N.
    Eor,

    /// Compare A - data_latch (no store); sets C,Z,N.
    CmpA,
    /// Compare X - data_latch (no store); sets C,Z,N.
    CmpX,
    /// Compare Y - data_latch (no store); sets C,Z,N.
    CmpY,
    /// A & data_latch (no store); Z = (result==0), N = data_latch.7, V = data_latch.6.
    Bit,

    /// data_latch <<= 1; C = old bit 7; sets Z,N.
    Asl,
    /// data_latch >>= 1; C = old bit 0; sets Z,N.
    Lsr,
    /// data_latch = (data_latch << 1) | C; C = old bit 7; sets Z,N.
    Rol,
    /// data_latch = (data_latch >> 1) | (C << 7); C = old bit 0; sets Z,N.
    Ror,
    /// data_latch += 1; sets Z,N.
    Inc,
    /// data_latch -= 1; sets Z,N.
    Dec,

    /// A <<= 1; C = old bit 7; sets Z,N (ASL accumulator).
    AslA,
    /// A >>= 1; C = old bit 0; sets Z,N (LSR accumulator).
    LsrA,
    /// A = (A << 1) | C; C = old bit 7; sets Z,N (ROL accumulator).
    RolA,
    /// A = (A >> 1) | (C << 7); C = old bit 0; sets Z,N (ROR accumulator).
    RorA,

    // ── Unofficial opcode ALU operations (RMW memory then combine with A) ──
    /// A = data_latch, X = data_latch; sets Z,N (LAX).
    Lax,
    /// A = A & data_latch; X = A; sets Z,N from result (LAX immediate).
    LaxImm,
    /// data_latch <<= 1; C = old bit 7; Z,N from data_latch; A |= data_latch; Z,N from A (SLO).
    Slo,
    /// data_latch = (data_latch << 1) | C; C = old bit 7; Z,N from data_latch; A &= data_latch; Z,N from A (RLA).
    Rla,
    /// data_latch >>= 1; C = old bit 0; Z,N from data_latch; A ^= data_latch; Z,N from A (SRE).
    Sre,
    /// data_latch = (data_latch >> 1) | (C << 7); C = old bit 0; Z,N from data_latch; ADC A + data_latch + C (RRA).
    Rra,
    /// data_latch -= 1; Z,N from data_latch; CMP A - data_latch; C,Z,N from compare (DCP).
    Dcp,
    /// data_latch += 1; Z,N from data_latch; SBC A - data_latch - (1-C); C,Z,N,V from SBC (ISC).
    Isc,
    /// A = A & data_latch; Z,N from result; C = N (ANC immediate).
    Anc,
    /// A = (A & data_latch) >> 1; N=0; Z from result; C = bit 0 of AND result (ALR immediate).
    Alr,
    /// A = ((A & data_latch) >> 1) | (C << 7); C = bit 0 of AND result; V = bit6 ^ bit5 of result (ARR immediate).
    Arr,

    // ── Control flow and branches ──
    /// Read offset from PC+1; if C=0, compute target PC and set branch_taken/page_crossed.
    BranchCC,
    /// Read offset; if C=1, compute target.
    BranchCS,
    /// Read offset; if Z=1, compute target.
    BranchEQ,
    /// Read offset; if Z=0, compute target.
    BranchNE,
    /// Read offset; if N=1, compute target.
    BranchMI,
    /// Read offset; if N=0, compute target.
    BranchPL,
    /// Read offset; if V=0, compute target.
    BranchVC,
    /// Read offset; if V=1, compute target.
    BranchVS,

    /// PC = addr (JMP absolute).
    JumpAbs,
    /// PC = (operands[1]<<8)|operands[0]; instruction_length=0 (JMP abs, same cycle as addr fetch).
    JmpAbs,
    /// PC = mem[addr] with NMOS 6502 page-wrap bug (JMP indirect).
    JumpInd,

    /// JSR cycle 6: read PC+2, combine target address, set PC.
    JsrC6,
    /// RTS final cycle: combine popped bytes, PC = (hi<<8|lo)+1, dummy read at new PC.
    RtsFinish,
    /// RTI final cycle: pop PC_lo, PC_hi from stack, set PC (no +1).
    RtiFinish,

    /// Set status register from data_latch: status = (data_latch | UNUSED) & !BREAK.
    SetStatus,

    // ── Control transfer ──
    /// Halt the CPU — JAM/KIL instruction. PC was already incremented by the
    /// operand fetch; no further execution occurs until reset.
    JamHalt,

    // ── Sequence control ──
    /// If page_crossed is true, skip the next `n` MicroOps in the sequence.
    SkipIfCrossed(u8),
    /// If page_crossed is false, skip the next `n` MicroOps.
    SkipIfNotCrossed(u8),
    /// If branch_taken is false, skip the next `n` MicroOps.
    SkipIfNotTaken(u8),
}
