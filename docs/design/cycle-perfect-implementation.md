# Cycle-Perfect CPU — Implementation Design

## 1. Architecture Overview

A fresh CPU module (`src/cpu6502/`) replaces the current `src/cpu/` implementation. The core idea: every instruction is a **static, declarative sequence of `MicroOp` variants** — one per clock cycle. The CPU's `step()` function simply executes `sequence[sequence_index]` and increments the index. No monolithic `execute_instruction()`, no separate operand resolution trait.

```
┌──────────────────────────────────────────────────┐
│                   CPU6502                         │
│                                                    │
│  registers: Registers                              │
│  sequence: &'static [MicroOp]   ← loaded on fetch  │
│  sequence_index: usize                              │
│  ea: u16                       ← effective address │
│  data_latch: u8                ← last byte read    │
│  branch_taken: bool                                 │
│  page_crossed: bool                                 │
│  total_cycles: u64                                  │
│                                                    │
│  step(&mut self, memory: &mut impl Memory) {       │
│      let op = self.sequence[self.sequence_index];  │
│      match op { ... }                               │
│      self.sequence_index += 1;                      │
│  }                                                  │
└──────────────────────────────────────────────────┘
```

### 1.1 Bus Contention: Not Applicable

On the VIC-20, the 6502 CPU and 6560/6561 VIC chip use **opposite clock phases** (CPU on φ2, VIC on φ1). The VIC has no RDY output pin. The 6502's RDY input is tied to +5V — the CPU is **never halted**. Unlike the C64 where the VIC-II can stall the CPU, the VIC-20 has zero bus contention. Reads and writes always succeed.

Therefore the `Memory` trait below uses plain `&mut self` methods — no `Result`, no `BusStatus`, no stall/wait logic.

---

## 2. The `Memory` Trait

```rust
/// Memory bus interface. On the VIC-20, reads and writes always succeed —
/// the CPU and VIC use opposite clock phases with no contention.
///
/// `read_byte` takes `&mut self` because some reads have side effects:
/// reading a VIA timer low byte clears the timer interrupt flag.
pub trait Memory {
    fn read_byte(&mut self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);

    // ── Derived operations ──

    fn read_word(&mut self, address: u16) -> u16 {
        let lo = self.read_byte(address) as u16;
        let hi = self.read_byte(address.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    fn read_zp_byte(&mut self, address: u8) -> u8 {
        self.read_byte(address as u16)
    }

    fn read_zp_word(&mut self, address: u8) -> u16 {
        self.read_word(address as u16)
    }

    fn write_word(&mut self, address: u16, value: u16) {
        self.write_byte(address, value as u8);
        self.write_byte(address.wrapping_add(1), (value >> 8) as u8);
    }

    fn write_zp_byte(&mut self, address: u8, value: u8) {
        self.write_byte(address as u16, value);
    }
}
```

**Why `&mut self` on reads?** The VIA 6522 clears interrupt flags when its timer registers are read. This is a hardware side effect. Using `&mut self` avoids the `Cell<T>` interior mutability that the current codebase relies on.

**Extensibility note:** If this emulator is later extended to the C64 (where the VIC-II can stall the CPU via RDY), the `Memory` trait can gain a `fn is_ready(&self) -> bool` method. The CPU would check it before executing a bus micro-op and NOP if not ready. For the VIC-20, the default `true` is correct.

---

## 3. The `MicroOp` Enum

Every variant represents exactly one cycle's worth of work. Variants carry **no data** — all addresses and values flow through CPU state fields (`ea`, `data_latch`, `operands[]`, registers).

### 3.1 Complete Variant List

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroOp {
    // ═══ Bus Reads ═══
    /// Read opcode at PC, decode → load sequence
    Fetch,
    /// Read PC+1 → operands[0]
    ReadPC1,
    /// Read PC+2 → operands[1]
    ReadPC2,
    /// Set EA = operands[0] as u16 (zero-page address)
    SetEA_ZP,
    /// Read memory[EA] → data_latch
    ReadEA,
    /// Read memory[EA] and discard (dummy read for timing)
    ReadDummyEA,

    // ═══ Bus Writes ═══
    /// Write value to memory[EA]
    WriteEA_DataLatch,
    /// Write register A to memory[EA]
    WriteEA_A,
    /// Write register X to memory[EA]
    WriteEA_X,
    /// Write register Y to memory[EA]
    WriteEA_Y,
    /// Write data_latch to memory[EA] (RMW dummy write of original value)
    WriteDummyEA,
    /// Write (status | B | UNUSED) to memory[EA] (PHP/BRK push status)
    WriteEA_StatusWithB,

    // ═══ Stack ═══
    /// Write value to 0x0100+SP, then SP--
    Push(u8),
    /// Push high byte of return_addr, then low byte (JSR/BRK)
    PushReturnAddr,
    /// Increment SP, read 0x0100+SP (stack not ready: read without SP change)
    PopDummy,
    /// SP++, read 0x0100+SP → data_latch
    Pop,

    // ═══ Address Computation ═══
    /// EA = operands[1]<<8 | operands[0]
    SetEA_Abs,
    /// EA = (operands[1]<<8 | operands[0]) + X; set page_crossed flag
    SetEA_AbsX,
    /// EA = (operands[1]<<8 | operands[0]) + Y; set page_crossed flag
    SetEA_AbsY,
    /// EA = (operands[0] + X) & 0xFF (zero page indexed)
    SetEA_ZpX,
    /// EA = (operands[0] + Y) & 0xFF (zero page indexed)
    SetEA_ZpY,
    /// ptr = (operands[0] + X) & 0xFF; EA_lo = mem[ptr]; EA_hi = mem[ptr+1]; combine
    SetEA_IndX,
    /// ptr = operands[0]; base_lo = mem[ptr]; base_hi = mem[ptr+1]; EA = base + Y; set page_crossed
    SetEA_IndY,

    // ═══ Register Operations (set Z,N from value) ═══
    SetA,           // A = data_latch
    SetX,           // X = data_latch
    SetY,           // Y = data_latch
    SetA_X,         // A = X (TXA)
    SetA_Y,         // A = Y (TYA)
    SetX_A,         // X = A (TAX)
    SetX_SP,        // X = SP (TSX)
    SetY_A,         // Y = A (TAY)
    SetSP_X,        // SP = X (TXS, no flags)
    IncX, IncY,     // X++, Y++ (set Z,N)
    DecX, DecY,     // X--, Y-- (set Z,N)

    // ═══ Flag Operations ═══
    SetC, ClrC, SetD, ClrD, SetI, ClrI, ClrV,

    // ═══ ALU Operations ═══
    /// A = A + data_latch + C; set C,Z,N,V (binary or BCD)
    AluADC,
    /// A = A - data_latch - (1-C); set C,Z,N,V
    AluSBC,
    /// A = A & data_latch; set Z,N
    AluAND,
    /// A = A | data_latch; set Z,N
    AluORA,
    /// A = A ^ data_latch; set Z,N
    AluEOR,
    /// Compare A - data_latch (no store); set C,Z,N
    AluCMP,
    /// Compare X - data_latch; set C,Z,N
    AluCPX,
    /// Compare Y - data_latch; set C,Z,N
    AluCPY,
    /// A & data_latch (no store); Z = (result==0), N = data.7, V = data.6
    AluBIT,
    /// data_latch << 1; C = old bit 7; set Z,N
    AluASL,
    /// data_latch >> 1; C = old bit 0; set Z,N
    AluLSR,
    /// data_latch = (data_latch << 1) | C; C = old bit 7; set Z,N
    AluROL,
    /// data_latch = (data_latch >> 1) | (C << 7); C = old bit 0; set Z,N
    AluROR,
    /// data_latch = data_latch + 1; set Z,N
    AluINC,
    /// data_latch = data_latch - 1; set Z,N
    AluDEC,

    // ═══ Control Flow ═══
    /// If C=0: read PC+1 offset, compute target, set branch_taken/page_crossed flags
    BranchIfCC,
    /// If C=1: read PC+1 offset, compute target
    BranchIfCS,
    /// If Z=1: read PC+1 offset, compute target
    BranchIfEQ,
    /// If Z=0: read PC+1 offset, compute target
    BranchIfNE,
    /// If N=1: read PC+1 offset, compute target
    BranchIfMI,
    /// If N=0: read PC+1 offset, compute target
    BranchIfPL,
    /// If V=0: read PC+1 offset, compute target
    BranchIfVC,
    /// If V=1: read PC+1 offset, compute target
    BranchIfVS,
    /// PC = EA (JMP)
    JumpToEA,
    /// PC = mem[EA] | (mem[EA+1]<<8) — with NMOS page-wrap bug
    JumpIndirect,
    /// Push PCH of return addr, then PCL (JSR cycles 4-5)
    PushReturnLo,
    /// Pops PC_lo → PC_hi → PC = (hi<<8)|lo; then PC++ (RTS)
    RtsReturn,

    // ═══ Interrupt Support ═══
    /// Push PCH of PC, then PCL, then status|B|UNUSED; set I
    BrkPush,
    /// Pop status (clear B, set UNUSED), pop PC_lo, pop PC_hi → PC (RTI)
    RtiReturn,
    /// Read vector at given address → PC
    FetchVector(u16),

    // ═══ Sequence Control ═══
    /// If page_crossed, skip next `n` micro-ops
    SkipIfCrossed(u8),
    /// If NOT page_crossed, skip next `n` micro-ops
    SkipIfNotCrossed(u8),
    /// If branch was NOT taken, skip next `n` micro-ops
    SkipIfNotTaken(u8),
    /// End of instruction; prepare for next fetch
    EndInstr,
    /// Do nothing (NOP internal cycles, stack delay cycles)
    NoOp,
}
```

### 3.2 Design Rationale

**Why no data in variants?** Variants like `ReadEA` use `cpu.ea` — the effective address was computed by a prior micro-op (`SetEA_Abs`, `SetEA_AbsX`, etc.). Variants like `SetA` use `cpu.data_latch` — the value was loaded by a prior `ReadEA` or `Pop`. This separation makes each micro-op do exactly one thing and makes sequences self-documenting.

**Why separate `Push(n)` from `PushReturnAddr`?** Stack pushes for PHA/PHP push one byte. JSR/BRK push 2–3 bytes in a specific order. `PushReturnAddr` encapsulates the JSR return-address push sequence (PCH first, then PCL), which spans 2 cycles.

**Why separate ALU variants?** Each ALU operation is semantically distinct. A single `Alu(Kind)` variant with a sub-enum would work, but separate variants make sequences more readable and the executor's match arm simpler.

---

## 4. CPU State

```rust
pub struct CPU6502 {
    pub registers: Registers,

    // ── Instruction sequencing ──
    /// Currently executing micro-op sequence (borrowed from static table)
    sequence: &'static [MicroOp],
    /// Index into `sequence` — which micro-op executes next
    sequence_index: usize,
    /// Operand bytes read from instruction stream (PC+1, PC+2)
    operands: [u8; 2],
    /// Computed effective address (populated by SetEA_* ops)
    ea: u16,
    /// Last byte read from memory (populated by ReadEA, Pop, ReadPC1, etc.)
    data_latch: u8,

    // ── Conditional flags ──
    /// Set by BranchIf* ops when branch condition is met
    branch_taken: bool,
    /// Set by SetEA_AbsX, SetEA_AbsY, SetEA_IndY ops when page crossed
    page_crossed: bool,

    // ── Interrupts ──
    pub irq_line_low: bool,
    pub nmi_latch: EdgeLatch,

    // ── Metrics ──
    pub total_cycles: u64,
}
```

**Key state fields and their lifecycle:**

| Field | Written by | Read by |
|-------|-----------|---------|
| `operands` | `ReadPC1`, `ReadPC2` | `SetEA_Abs`, `SetEA_ZpX`, `BranchIf*`, etc. |
| `ea` | `SetEA_Abs`, `SetEA_AbsX`, `SetEA_ZP`, etc. | `ReadEA`, `WriteEA_*`, `ReadDummyEA`, `JumpToEA`, etc. |
| `data_latch` | `ReadEA`, `Pop`, `ReadPC1` (branch offset) | `SetA`, `SetX`, `SetY`, `Alu*`, `WriteEA_DataLatch` |
| `branch_taken` | `BranchIf*` | `SkipIfNotTaken` |
| `page_crossed` | `SetEA_AbsX`, `SetEA_AbsY`, `SetEA_IndY`, `BranchIf*` | `SkipIfCrossed`, `SkipIfNotCrossed` |

---

## 5. Sequence Table

All 256 opcodes map to static `&[MicroOp]` slices. The table is built at compile time via `build.rs`.

### 5.1 Example Sequences

```rust
// ── Implied / Register ──

static SEQ_NOP: &[MicroOp] = &[
    Fetch,
    NoOp,
    EndInstr,
];

static SEQ_INX: &[MicroOp] = &[
    Fetch,
    IncX,
    EndInstr,
];

static SEQ_TXA: &[MicroOp] = &[
    Fetch,
    SetA_X,
    EndInstr,
];

// ── Immediate ──

static SEQ_LDA_IMM: &[MicroOp] = &[
    Fetch,
    ReadPC1,    // operand → data_latch
    SetA,       // A = data_latch
    EndInstr,
];

static SEQ_ADC_IMM: &[MicroOp] = &[
    Fetch,
    ReadPC1,    // operand → data_latch
    AluADC,     // A += data_latch + C
    EndInstr,
];

// ── Zero Page ──

static SEQ_LDA_ZP: &[MicroOp] = &[
    Fetch,
    ReadPC1,    // zp_addr → operands[0]
    SetEA_ZP,   // EA = operands[0] as u16
    ReadEA,     // mem[EA] → data_latch
    SetA,
    EndInstr,
];

static SEQ_STA_ZP: &[MicroOp] = &[
    Fetch,
    ReadPC1,
    SetEA_ZP,
    WriteEA_A,
    EndInstr,
];

// ── Absolute ──

static SEQ_LDA_ABS: &[MicroOp] = &[
    Fetch,
    ReadPC1,    // addr_lo → operands[0]
    ReadPC2,    // addr_hi → operands[1]
    SetEA_Abs,  // EA = operands[1]<<8 | operands[0]
    ReadEA,     // mem[EA] → data_latch
    SetA,
    EndInstr,
];

static SEQ_JMP_ABS: &[MicroOp] = &[
    Fetch,
    ReadPC1,
    ReadPC2,
    SetEA_Abs,
    JumpToEA,
    EndInstr,
];

// ── Absolute Indexed (with page-cross handling) ──

static SEQ_LDA_ABSX: &[MicroOp] = &[
    Fetch,              // C1
    ReadPC1,            // C2: addr_lo → operands[0]
    SetEA_AbsX,         // C3: read addr_hi, EA = base + X, set page_crossed
    SkipIfNotCrossed(1),// C3: if same page, skip dummy read
    ReadDummyEA,        // C4: dummy read at wrong page (cross only)
    ReadEA,             // C4/C5: mem[EA] → data_latch
    SetA,
    EndInstr,
];
// 4 cycles (no cross): Fetch→ReadPC1→SetEA_AbsX→Skip(skips Dummy)→ReadEA→SetA→EndInstr
// 5 cycles (cross):   Fetch→ReadPC1→SetEA_AbsX→Skip(pass)→ReadDummyEA→ReadEA→SetA→EndInstr

static SEQ_STA_ABSX: &[MicroOp] = &[
    Fetch,              // C1
    ReadPC1,            // C2: addr_lo
    SetEA_AbsX,         // C3: read addr_hi, EA = base + X, set page_crossed
    ReadDummyEA,        // C4: always dummy read (STA indexed always 5 cycles)
    WriteEA_A,          // C5: mem[EA] = A
    EndInstr,
];

// ── RMW Absolute Indexed X (always 7 cycles) ──

static SEQ_ASL_ABSX: &[MicroOp] = &[
    Fetch,              // C1
    ReadPC1,            // C2: addr_lo
    SetEA_AbsX,         // C3: read addr_hi, EA = base + X, set page_crossed
    ReadEA,             // C4: mem[EA] → data_latch (may be wrong page if crossed)
    SkipIfNotCrossed(1),// C4: if same page, skip redundant read
    ReadEA,             // C5: mem[EA] → data_latch (corrected page / redundant)
    WriteDummyEA,       // C6: write original value back
    AluASL,             // C6: data_latch <<= 1, set C/Z/N
    WriteEA_DataLatch,  // C7: write modified value
    EndInstr,
];

// ── Branches ──

static SEQ_BCC: &[MicroOp] = &[
    Fetch,              // C1
    BranchIfCC,         // C2: read offset, check C=0; if taken: compute target, set flags
    SkipIfNotTaken(3),  // C2: if NOT taken, skip to EndInstr (2 cycles total)
    ReadDummyEA,        // C3: dummy read (at target if same page; wrong page if crossed)
    SkipIfNotCrossed(1),// C3: if same page, skip to EndInstr (3 cycles total)
    ReadDummyEA,        // C4: dummy read at corrected target (cross only)
    EndInstr,           // 4 cycles (cross)
    // Note: SkipIfNotTaken(3) lands on EndInstr (index 5)
    //       SkipIfNotCrossed(1) also lands on EndInstr (index 5)
];

// ── JSR / RTS ──

static SEQ_JSR: &[MicroOp] = &[
    Fetch,              // C1
    ReadPC1,            // C2: target_lo → operands[0]
    NoOp,               // C3: internal stack preparation
    Push(PCH),          // C4: push return addr high
    Push(PCL),          // C5: push return addr low
    ReadPC2,            // C6: target_hi → operands[1]
    SetEA_Abs,          // C6: EA = target
    JumpToEA,           // C6: PC = EA
    EndInstr,
];

static SEQ_RTS: &[MicroOp] = &[
    Fetch,              // C1
    NoOp,               // C2: internal
    PopDummy,           // C3: dummy stack read
    Pop,                // C4: PC_lo → data_latch
    Pop,                // C5: PC_hi → data_latch
    RtsReturn,          // C5-C6: combine PC, PC++, end
    EndInstr,
];
// Note: RtsReturn combines the PC assembly and the C6 dummy read + PC++ into one op.
// For true cycle-perfection, this could be split — see §8.

// ── BRK ──
static SEQ_BRK: &[MicroOp] = &[
    Fetch,              // C1
    ReadPC1,            // C2: signature byte (discarded)
    Push(PCH),          // C3: push return addr high
    Push(PCL),          // C4: push return addr low
    WriteEA_StatusWithB,// C5: push status with B=1, UNUSED=1
    SetI,               // C5: set interrupt disable
    FetchVector(0xFFFE),// C6-C7: read IRQ vector → PC
    EndInstr,
];
```

### 5.2 Complete Opcode Table Structure

```rust
// Generated by build.rs
pub static OPCODE_SEQUENCES: [&[MicroOp]; 256] = [
    &SEQ_BRK,             // 0x00
    &SEQ_ORA_INDX,        // 0x01
    &[],                  // 0x02 — illegal
    // ... all 256 entries ...
    &SEQ_INC_ABSX,        // 0xFE
    &[],                  // 0xFF — illegal
];
```

---

## 6. Step Function

```rust
impl CPU6502 {
    pub fn step(&mut self, memory: &mut impl Memory) {
        // Interrupt check (only between instructions)
        if self.sequence_index == 0 {
            if self.nmi_latch.take() {
                self.enter_interrupt(Interrupt::NMI);
            } else if self.irq_line_low && !self.registers.is_flag_set(INTERRUPT) {
                self.enter_interrupt(Interrupt::IRQ);
            }
        }

        self.total_cycles += 1;

        let op = self.sequence[self.sequence_index];

        match op {
            // ── Bus Reads ──
            MicroOp::Fetch => {
                let opcode = memory.read_byte(self.registers.pc);
                self.sequence = OPCODE_SEQUENCES[opcode as usize];
                self.sequence_index = 0; // Will be incremented to 1 after this match
                self.operands = [0; 2];
                self.ea = 0;
                self.data_latch = 0;
                self.branch_taken = false;
                self.page_crossed = false;
                return; // Don't increment sequence_index (Fetch is index 0 of new sequence)
            }

            MicroOp::ReadPC1 => {
                self.operands[0] = memory.read_byte(self.registers.pc.wrapping_add(1));
            }
            MicroOp::ReadPC2 => {
                self.operands[1] = memory.read_byte(self.registers.pc.wrapping_add(2));
            }
            MicroOp::SetEA_ZP => {
                self.ea = self.operands[0] as u16;
            }
            MicroOp::ReadEA => {
                self.data_latch = memory.read_byte(self.ea);
            }
            MicroOp::ReadDummyEA => {
                let _ = memory.read_byte(self.ea);
            }

            // ── Bus Writes ──
            MicroOp::WriteEA_DataLatch => memory.write_byte(self.ea, self.data_latch),
            MicroOp::WriteEA_A => memory.write_byte(self.ea, self.registers.a),
            MicroOp::WriteEA_X => memory.write_byte(self.ea, self.registers.x),
            MicroOp::WriteEA_Y => memory.write_byte(self.ea, self.registers.y),
            MicroOp::WriteDummyEA => memory.write_byte(self.ea, self.data_latch),
            MicroOp::WriteEA_StatusWithB => {
                memory.write_byte(self.ea, self.registers.status | UNUSED | BREAK);
            }

            // ── Stack ──
            MicroOp::Push(what) => {
                let value = match what {
                    PushWhat::PCH => (self.registers.pc >> 8) as u8,
                    PushWhat::PCL => self.registers.pc as u8,
                    PushWhat::A => self.registers.a,
                    PushWhat::StatusWithB => self.registers.status | UNUSED | BREAK,
                };
                memory.write_byte(0x0100 + self.registers.sp as u16, value);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            MicroOp::PopDummy => {
                let _ = memory.read_byte(0x0100 + self.registers.sp as u16);
            }
            MicroOp::Pop => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.data_latch = memory.read_byte(0x0100 + self.registers.sp as u16);
            }

            // ── Address Computation ──
            MicroOp::SetEA_Abs => {
                self.ea = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
            }
            MicroOp::SetEA_AbsX => {
                let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
                // Read the high byte (it's at PC+2; we already have it in operands[1])
                self.ea = base.wrapping_add(self.registers.x as u16);
                self.page_crossed = (base & 0xFF00) != (self.ea & 0xFF00);
            }
            MicroOp::SetEA_AbsY => {
                let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
                self.ea = base.wrapping_add(self.registers.y as u16);
                self.page_crossed = (base & 0xFF00) != (self.ea & 0xFF00);
            }
            MicroOp::SetEA_ZpX => {
                self.ea = (self.operands[0].wrapping_add(self.registers.x) & 0xFF) as u16;
            }
            MicroOp::SetEA_ZpY => {
                self.ea = (self.operands[0].wrapping_add(self.registers.y) & 0xFF) as u16;
            }
            MicroOp::SetEA_IndX => {
                let ptr = self.operands[0].wrapping_add(self.registers.x) & 0xFF;
                let lo = memory.read_zp_byte(ptr);
                let hi = memory.read_zp_byte(ptr.wrapping_add(1));
                self.ea = (hi as u16) << 8 | lo as u16;
            }
            MicroOp::SetEA_IndY => {
                let ptr = self.operands[0];
                let lo = memory.read_zp_byte(ptr);
                let hi = memory.read_zp_byte(ptr.wrapping_add(1));
                let base = (hi as u16) << 8 | lo as u16;
                self.ea = base.wrapping_add(self.registers.y as u16);
                self.page_crossed = (base & 0xFF00) != (self.ea & 0xFF00);
            }

            // ── Register Operations ──
            MicroOp::SetA => self.registers.set_accumulator(self.data_latch),
            MicroOp::SetX => self.registers.set_x(self.data_latch),
            MicroOp::SetY => self.registers.set_y(self.data_latch),
            MicroOp::SetA_X => self.registers.set_accumulator(self.registers.x),
            MicroOp::SetA_Y => self.registers.set_accumulator(self.registers.y),
            MicroOp::SetX_A => self.registers.set_x(self.registers.a),
            MicroOp::SetX_SP => self.registers.set_x(self.registers.sp),
            MicroOp::SetY_A => self.registers.set_y(self.registers.a),
            MicroOp::SetSP_X => { self.registers.sp = self.registers.x; }
            MicroOp::IncX => self.registers.set_x(self.registers.x.wrapping_add(1)),
            MicroOp::IncY => self.registers.set_y(self.registers.y.wrapping_add(1)),
            MicroOp::DecX => self.registers.set_x(self.registers.x.wrapping_sub(1)),
            MicroOp::DecY => self.registers.set_y(self.registers.y.wrapping_sub(1)),

            // ── Flag Operations ──
            MicroOp::SetC => self.registers.update_carry_flag(true),
            MicroOp::ClrC => self.registers.update_carry_flag(false),
            MicroOp::SetD => self.registers.update_decimal_flag(true),
            MicroOp::ClrD => self.registers.update_decimal_flag(false),
            MicroOp::SetI => self.registers.update_interrupt_flag(true),
            MicroOp::ClrI => self.registers.update_interrupt_flag(false),
            MicroOp::ClrV => self.registers.update_overflow_flag(false),

            // ── ALU ──
            MicroOp::AluADC => adc(&mut self.registers, self.data_latch),
            MicroOp::AluSBC => sbc(&mut self.registers, self.data_latch),
            MicroOp::AluAND => self.registers.set_accumulator(self.registers.a & self.data_latch),
            MicroOp::AluORA => self.registers.set_accumulator(self.registers.a | self.data_latch),
            MicroOp::AluEOR => self.registers.set_accumulator(self.registers.a ^ self.data_latch),
            MicroOp::AluCMP => compare(&mut self.registers, self.registers.a, self.data_latch),
            MicroOp::AluCPX => compare(&mut self.registers, self.registers.x, self.data_latch),
            MicroOp::AluCPY => compare(&mut self.registers, self.registers.y, self.data_latch),
            MicroOp::AluBIT => {
                let v = self.data_latch;
                self.registers.set_flag(ZERO, self.registers.a & v == 0);
                self.registers.set_flag(OVERFLOW, v & 0x40 != 0);
                self.registers.set_flag(NEGATIVE, v & 0x80 != 0);
            }
            MicroOp::AluASL => {
                let c = self.data_latch & 0x80 != 0;
                self.data_latch <<= 1;
                self.registers.update_carry_flag(c);
                self.registers.update_zero_and_negative(self.data_latch);
            }
            MicroOp::AluLSR => {
                let c = self.data_latch & 0x01 != 0;
                self.data_latch >>= 1;
                self.registers.update_carry_flag(c);
                self.registers.update_zero_and_negative(self.data_latch);
            }
            MicroOp::AluROL => {
                let old_c = self.registers.is_flag_set(CARRY) as u8;
                let new_c = self.data_latch & 0x80 != 0;
                self.data_latch = (self.data_latch << 1) | old_c;
                self.registers.update_carry_flag(new_c);
                self.registers.update_zero_and_negative(self.data_latch);
            }
            MicroOp::AluROR => {
                let old_c = self.registers.is_flag_set(CARRY) as u8;
                let new_c = self.data_latch & 0x01 != 0;
                self.data_latch = (self.data_latch >> 1) | (old_c << 7);
                self.registers.update_carry_flag(new_c);
                self.registers.update_zero_and_negative(self.data_latch);
            }
            MicroOp::AluINC => {
                self.data_latch = self.data_latch.wrapping_add(1);
                self.registers.update_zero_and_negative(self.data_latch);
            }
            MicroOp::AluDEC => {
                self.data_latch = self.data_latch.wrapping_sub(1);
                self.registers.update_zero_and_negative(self.data_latch);
            }

            // ── Control Flow ──
            MicroOp::BranchIfCC => branch_if(&mut self.registers, &mut self.operands, memory, |r| !r.is_flag_set(CARRY)),
            MicroOp::BranchIfCS => branch_if(&mut self.registers, &mut self.operands, memory, |r| r.is_flag_set(CARRY)),
            MicroOp::BranchIfEQ => branch_if(&mut self.registers, &mut self.operands, memory, |r| r.is_flag_set(ZERO)),
            MicroOp::BranchIfNE => branch_if(&mut self.registers, &mut self.operands, memory, |r| !r.is_flag_set(ZERO)),
            MicroOp::BranchIfMI => branch_if(&mut self.registers, &mut self.operands, memory, |r| r.is_flag_set(NEGATIVE)),
            MicroOp::BranchIfPL => branch_if(&mut self.registers, &mut self.operands, memory, |r| !r.is_flag_set(NEGATIVE)),
            MicroOp::BranchIfVC => branch_if(&mut self.registers, &mut self.operands, memory, |r| !r.is_flag_set(OVERFLOW)),
            MicroOp::BranchIfVS => branch_if(&mut self.registers, &mut self.operands, memory, |r| r.is_flag_set(OVERFLOW)),
            MicroOp::JumpToEA => {
                self.registers.pc = self.ea;
            }
            MicroOp::JumpIndirect => {
                // NMOS 6502 bug: when ptr is $xxFF, high byte from $xx00, not $(xx+1)00
                let lo = memory.read_byte(self.ea);
                let hi_addr = (self.ea & 0xFF00) | ((self.ea as u8).wrapping_add(1) as u16);
                let hi = memory.read_byte(hi_addr);
                self.registers.pc = (hi as u16) << 8 | lo as u16;
            }
            MicroOp::RtsReturn => {
                // Pops performed in prior cycles. data_latch holds PC_hi.
                // PC_lo was popped first (cycle 4), PC_hi second (cycle 5).
                // This op: combine them, do C6 dummy read + PC++.
                let pc_lo = self.pulled_pc_lo; // Set by the Pop that read PC_lo
                let pc_hi = self.data_latch;
                self.registers.pc = ((pc_hi as u16) << 8 | pc_lo as u16).wrapping_add(1);
            }
            MicroOp::FetchVector(addr) => {
                self.registers.pc = memory.read_word(addr);
            }

            // ── Sequence Control ──
            MicroOp::SkipIfCrossed(n) => {
                if self.page_crossed {
                    self.sequence_index += n as usize;
                }
            }
            MicroOp::SkipIfNotCrossed(n) => {
                if !self.page_crossed {
                    self.sequence_index += n as usize;
                }
            }
            MicroOp::SkipIfNotTaken(n) => {
                if !self.branch_taken {
                    self.sequence_index += n as usize;
                }
            }
            MicroOp::EndInstr => {
                // Advance PC past instruction bytes
                // (handled by the sequence — some ops set PC explicitly)
                // Check for pending interrupts
                if self.nmi_latch.take() {
                    self.enter_interrupt(Interrupt::NMI);
                    return;
                }
                if self.irq_line_low && !self.registers.is_flag_set(INTERRUPT) {
                    self.enter_interrupt(Interrupt::IRQ);
                    return;
                }
                // Prepare for next instruction — PC already set by sequence
            }
            MicroOp::NoOp => {}
        }

        self.sequence_index += 1;
    }
}
```

### 6.1 Branch Helper

```rust
fn branch_if(
    registers: &mut Registers,
    operands: &mut [u8; 2],
    memory: &mut impl Memory,
    condition: impl Fn(&Registers) -> bool,
) {
    // Read the offset byte (this is the C2 bus read)
    let offset = memory.read_byte(registers.pc.wrapping_add(1)) as i8 as i16;
    operands[0] = offset as u8;

    if condition(registers) {
        let base = registers.pc.wrapping_add(2);
        let target = base.wrapping_add(offset as u16);
        // cpu.branch_taken and cpu.page_crossed are set by the caller
        // (these are fields on CPU6502, passed via &mut self in the real impl)
    }
}
```

### 6.2 PC Advancement

After `EndInstr`, the PC should point to the next instruction. For most instructions, the sequence explicitly sets PC (via `JumpToEA`, branch helpers, etc.) or the PC is incremented by the sequence length. The cleanest approach: `EndInstr` computes the default next-PC as `PC + instruction_length` and sets it, unless a control-flow op already set PC.

Alternatively, each sequence ends with an explicit `SetPC` or `JumpToEA`. This makes the sequence self-contained at the cost of an extra micro-op for simple instructions. For clarity, I recommend:

```rust
// For simple instructions, EndInstr advances PC by instruction length
MicroOp::EndInstr => {
    let len = self.instruction_length; // set during Fetch
    self.registers.pc = self.registers.pc.wrapping_add(len);
    // ...
}
```

`instruction_length` is set during `Fetch`: 1 for implied/accumulator, 2 for immediate/zp/relative/indexed-indirect, 3 for absolute/indirect.

---

## 7. Interrupt Sequences

Interrupts are handled as special instruction sequences:

```rust
static SEQ_NMI: &[MicroOp] = &[
    NoOp,               // C1: internal (fetch suppressed)
    NoOp,               // C2: internal
    Push(PCH),          // C3
    Push(PCL),          // C4
    WriteEA_StatusWithB,// C5: push status (B=0 for NMI/IRQ)
    SetI,               // C5: set interrupt disable
    FetchVector(0xFFFA),// C6-C7: NMI vector
    EndInstr,
];

static SEQ_IRQ: &[MicroOp] = &[
    NoOp, NoOp,
    Push(PCH), Push(PCL),
    WriteEA_StatusWithB, SetI,
    FetchVector(0xFFFE), // IRQ vector
    EndInstr,
];

static SEQ_RESET: &[MicroOp] = &[
    NoOp, NoOp, NoOp,           // C1-C3: internal
    PopDummy, PopDummy, PopDummy,// C4-C6: SP decremented by 3
    FetchVector(0xFFFC),         // C7: reset vector
    EndInstr,
];
```

When an interrupt triggers, the CPU sets `self.sequence = SEQ_NMI` (or `SEQ_IRQ`) and `self.sequence_index = 0`.

---

## 8. Open Design Questions

### 8.1 JSR Push Order

The real 6502 pushes PCH first (C4), then PCL (C5). The `PushReturnAddr` op currently handles both pushes in one op. For true cycle-perfection, split into two cycles. This requires the CPU to track the return address explicitly:

```rust
// After ReadPC1 (C2), compute return = PC + 2
// C3: NoOp (internal stack setup)
// C4: Push return_addr >> 8   (PCH)
// C5: Push return_addr & 0xFF (PCL)
// C6: ReadPC2 → target_hi, combine, JumpToEA
```

### 8.2 RTS Cycle 6 Detail

The real 6502 does a dummy read at the restored PC on cycle 6, then PC++. The `RtsReturn` op bundles the PC++ with the dummy read. For maximum accuracy, split:

```rust
// C5: Pop → PC_hi, set PC = hi:lo
// C6: ReadDummy(PC)  (or ReadPC1 — reads next opcode but discards)
// C6 internal: PC += 1
// EndInstr
```

### 8.3 Accumulator-mode RMW

`ASL A`, `LSR A`, `ROL A`, `ROR A` operate on the accumulator without memory access beyond the fetch. The current ALU variants (`AluASL`/`AluLSR`/`AluROL`/`AluROR`) operate on `data_latch`. For accumulator mode, we need variants that operate on `A`:

```rust
MicroOp::AluASL_A,  // A <<= 1; carry = old bit 7; set Z,N
MicroOp::AluLSR_A,  // A >>= 1; carry = old bit 0; set Z,N
MicroOp::AluROL_A,  // A = (A<<1) | C; carry = old bit 7
MicroOp::AluROR_A,  // A = (A>>1) | (C<<7); carry = old bit 0
```

### 8.4 Variable Instruction Length

Some instructions are 1 byte (implied), 2 bytes (immediate/zp/branch), or 3 bytes (absolute). `EndInstr` needs to know how far to advance PC. Store `instruction_length: u8` in CPU state, set during `Fetch`.

### 8.5 Build Script vs const fn Table

`build.rs` is the recommended approach for generating the opcode table. It's debuggable, can include comments showing the instruction mnemonic, and avoids `const` evaluation limits. Alternative: use `phf` or `lazy_static` for runtime initialization.

---

## 9. Migration Path

This is a new module — no migration from the existing `src/cpu/` code. The plan:

1. **Create `src/cpu6502/`** as a parallel module (not replacing `src/cpu/` initially)
2. **Implement the `Memory` trait** on `Bus` (trivial wrapper delegating to existing `Addressable`)
3. **Implement `CPU6502::step()`** as described above
4. **Generate `instruction_sequences.rs`** via `build.rs`
5. **Write comprehensive tests** per instruction group (see verification strategy in the design doc)
6. **Wire into the emulator** by replacing `src/emulator/runner.rs`'s `cpu.step(&mut bus)` call with the new CPU
7. **Remove old `src/cpu/`** once validated

---

## 10. Test Strategy

### 10.1 Per-Instruction Tests

```rust
#[test]
fn test_lda_immediate_loads_accumulator() {
    let mut cpu = CPU6502::new();
    let mut mem = Ram::new();
    cpu.registers.pc = 0x0200;
    mem.write_byte(0x0200, 0xA9); // LDA #
    mem.write_byte(0x0201, 0x42); // operand

    cpu.step(&mut mem); // Fetch
    assert_eq!(cpu.sequence_index, 1);
    cpu.step(&mut mem); // ReadPC1 + SetA + EndInstr
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.registers.pc, 0x0202);
}
```

### 10.2 Cycle Count Tests

```rust
#[test]
fn test_lda_immediate_takes_2_cycles() {
    let mut cpu = CPU6502::new();
    let mut mem = Ram::new();
    cpu.registers.pc = 0x0200;
    mem.write_byte(0x0200, 0xA9);
    mem.write_byte(0x0201, 0x42);

    let before = cpu.total_cycles;
    cpu.step(&mut mem);
    cpu.step(&mut mem);
    assert_eq!(cpu.total_cycles - before, 2);
}
```

### 10.3 Sequence Correctness Tests

Verify every sequence has exactly one `Fetch` and one `EndInstr`, and the number of bus-accessing ops matches the published cycle count.

---

## 11. References

- [Visual 6502](http://www.visual6502.org/) — transistor-level cycle simulation
- [NESdev Wiki: CPU](https://www.nesdev.org/wiki/CPU) — NMOS 6502 cycle timing
- [docs/design/cycle-perfect-cpu.md](cycle-perfect-cpu.md) — prerequisite design doc with full opcode breakdown
