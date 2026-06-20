# Cycle-Perfect CPU Refactor — Design Document

## 1. Motivation

The current CPU implementation is _cycle-accurate_ (each `cpu.step()` is one clock cycle), but **not cycle-perfect**: instruction execution happens atomically on the final cycle of each instruction. All bus reads/writes that should be spread across cycles happen simultaneously in `execute_instruction()`.

### Why This Matters

| Concern | Current Behavior | Cycle-Perfect Behavior |
|---------|-----------------|----------------------|
| **Bus timing** | All operand reads happen on last cycle | Reads/writes occur on the exact cycle the real 6502 performs them |
| **Hardware interaction** | VIA/VIC registers read once atomically | Devices see reads/writes on specific cycles — critical for self-modifying code and hardware race conditions |
| **Dummy reads/writes** | Not modeled | Correctly modeled (affects VIA timer reads, VIC raster effects, etc.) |
| **RMW (read-modify-write)** | Read + write on same cycle | Read on cycle N, dummy write on N+1, final write on N+2 |
| **Interrupt timing** | Approximate | Exact — NMI/IRQ sampled between specific cycles |
| **Testability** | Hard to test bus-level timing | Each cycle is a single testable micro-operation |

### Goal

Break each 6502 instruction into a sequence of **micro-operations** — one per clock cycle — that exactly match the real 6502's bus activity. The number of micro-operations equals the instruction's cycle count (including page-cross and branch-taken penalties).

---

## 2. Micro-Operation Catalog

Every cycle, the 6502 performs exactly one of these operations on the bus, plus optional internal register updates:

### 2.1 Bus Operations

| Micro-Op | Bus Activity | Description |
|----------|-------------|-------------|
| `Fetch` | Read `PC` | Opcode fetch. Sets `current_instruction`. `PC` not yet incremented. |
| `ReadImm` | Read `PC+N` | Read immediate operand byte from program stream. `PC` advances. |
| `ReadZP` | Read `PC+N` | Read zero-page address from program stream. |
| `ReadAbsLo` | Read `PC+N` | Read low byte of absolute address. |
| `ReadAbsHi` | Read `PC+N` | Read high byte of absolute address. Combines with low byte. |
| `ReadPtrLo` | Read `(zp+X)` or `(zp)` | Read low byte of pointer from zero page. |
| `ReadPtrHi` | Read `(zp+X+1)` or `(zp+1)` | Read high byte of pointer from zero page. |
| `ReadData` | Read computed address | Read operand value from final effective address. |
| `ReadDummy` | Read computed address | Dummy read — data discarded. For page-cross fixup or indexed addressing timing. |
| `WriteData` | Write computed address | Write register value to final effective address. |
| `WriteDummy` | Write computed address | Dummy write — original (unmodified) value. RMW instruction cycle 4 behavior. |
| `Push` | Write `0x0100 + SP`, then `SP--` | Push byte to stack. |
| `PullDummy` | Read `0x0100 + SP` (SP unchanged) | Internal stack read, SP not yet incremented. |
| `Pull` | `SP++`, then Read `0x0100 + SP` | Pull byte from stack. |

### 2.2 Internal Operations

These happen in parallel with the bus operation on a cycle:

| Internal Op | Description |
|-------------|-------------|
| `SetFlags` | Update Z and N flags from a value. |
| `SetFlag` | Set/clear a single flag (C, D, I, V). |
| `RegTransfer` | Copy one register to another, set Z/N. |
| `RegInc` | Increment register, set Z/N. |
| `RegDec` | Decrement register, set Z/N. |
| `ALU_ADC` | Perform ADC computation, update A/C/Z/N/V. |
| `ALU_SBC` | Perform SBC computation, update A/C/Z/N/V. |
| `ALU_AND` | A = A & operand, set Z/N. |
| `ALU_ORA` | A = A | operand, set Z/N. |
| `ALU_EOR` | A = A ^ operand, set Z/N. |
| `ALU_CMP` | Compare register to operand, set C/Z/N. |
| `ALU_BIT` | Test A & operand, set Z/N/V. |
| `ALU_SHIFT` | Shift/rotate accumulator or memory value. |
| `ALU_INC` | Increment memory value, set Z/N. |
| `ALU_DEC` | Decrement memory value, set Z/N. |
| `SetPC` | Set program counter to target address. |
| `BranchCalc` | Add signed offset to PC (internal). |
| `AddIndex` | Add X or Y to address low byte (internal). |
| `PushPC` | Prepare PC for pushing (internal address calc). |
| `PullPC` | Combine pulled bytes into PC. |

### 2.3 Proposed Rust Types

```rust
/// A single cycle's bus operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusOp {
    Fetch,
    ReadPC1,
    ReadPC2,
    ReadZP,
    ReadPtrLo,
    ReadPtrHi,
    ReadData(u16),       // effective address already computed
    ReadDummy(u16),
    WriteData(u16, u8),  // address + value
    WriteDummy(u16, u8), // RMW: write original value back
    Push(u8),            // value to push
    PullDummy,
    Pull,
}

/// ALU operation variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluOp {
    Adc,
    Sbc,
    And,
    Ora,
    Eor,
    Cmp(u8),    // which register to compare
    Bit,
    Asl,
    Lsr,
    Rol,
    Ror,
    Inc,
    Dec,
}

/// Internal computation performed in parallel with a bus operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Internal {
    None,
    SetFlagC(bool),
    SetFlagD(bool),
    SetFlagI(bool),
    SetFlagV(bool),
    SetA(u8),
    SetX(u8),
    SetY(u8),
    TxS,
    Inx,
    Iny,
    Dex,
    Dey,
    AddIndexX,
    AddIndexY,
    CombineAddress,
    BranchCalc,
    SetPC(u16),
    PushPC,
    Alu(AluOp),
}

/// One cycle of an instruction: exactly one bus operation + optional internal.
#[derive(Debug, Clone)]
pub struct MicroCycle {
    pub bus: BusOp,
    pub internal: Internal,
}

/// A complete instruction broken into per-cycle micro-operations.
pub type InstructionSequence = Vec<MicroCycle>;
```

---

## 3. Complete Cycle-by-Cycle Breakdown
### Conventions

- **C1, C2, …** = cycle number within the instruction
- **Bus** = what appears on the address/data bus this cycle
- **Internal** = register/logic changes happening concurrently
- **Base cycles** shown; `+1` indicates an extra cycle when conditions are met
- `PC` = PC at instruction start (opcode address)
- `EA` = effective address (fully computed target)
- `oper` = operand value (byte at EA)
- `zp_base` = zero-page base from operand byte

All 151 NMOS 6502 opcodes are covered by the groups below.

---

### 3.1 Flag Set/Clear — 2 cycles

**Opcodes:** `CLC(0x18)`, `CLD(0xD8)`, `CLI(0x58)`, `CLV(0xB8)`,
`SEC(0x38)`, `SED(0xF8)`, `SEI(0x78)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | `SetFlag{Carry,Decimal,Interrupt,Overflow}` |

### 3.2 NOP — 2 cycles

**Opcodes:** `NOP(0xEA)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | `NoOp` |

### 3.3 Register Transfers — 2 cycles

**Opcodes:** `TAX(0xAA)`, `TAY(0xA8)`, `TSX(0xBA)`, `TXA(0x8A)`, `TYA(0x98)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | `SetX(A)` / `SetY(A)` / `SetX(SP)` / `SetA(X)` / `SetA(Y)` — sets Z,N |

`TXS(0x9A)` is a special case (no flag changes):

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | `TxS` (SP = X) |

### 3.4 Register Increment/Decrement — 2 cycles

**Opcodes:** `INX(0xE8)`, `INY(0xC8)`, `DEX(0xCA)`, `DEY(0x88)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | `Inx` / `Iny` / `Dex` / `Dey` |

### 3.5 Stack Push — 3 cycles

**Opcodes:** `PHA(0x48)`, `PHP(0x08)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | Internal (no bus access) |
| C3 | `Push(A)` or `Push(status \| B \| UNUSED)` | — |

### 3.6 Stack Pull — 4 cycles

**Opcodes:** `PLA(0x68)`, `PLP(0x28)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | Internal (no bus access) |
| C3 | `PullDummy` | — |
| C4 | `Pull` | `SetA(value)` / `status = (value \| UNUSED) & !B` |

### 3.7 Immediate — 2 cycles

**Opcodes:** `ADC #(0x69)`, `AND #(0x29)`, `CMP #(0xC9)`, `CPX #(0xE0)`, `CPY #(0xC0)`,
`EOR #(0x49)`, `LDA #(0xA9)`, `LDX #(0xA2)`, `LDY #(0xA0)`,
`ORA #(0x09)`, `SBC #(0xE9)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → oper | `Alu(Adc,And,Cmp,Eor,Ora,Sbc)` or `SetA(oper)` / `SetX(oper)` / `SetY(oper)` |

### 3.8 Zero Page Read — 3 cycles

**Opcodes:** `ADC zp(0x65)`, `AND zp(0x25)`, `BIT zp(0x24)`, `CMP zp(0xC5)`,
`CPX zp(0xE4)`, `CPY zp(0xC4)`, `EOR zp(0x45)`, `LDA zp(0xA5)`,
`LDX zp(0xA6)`, `LDY zp(0xA4)`, `ORA zp(0x05)`, `SBC zp(0xE5)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_addr | — |
| C3 | `ReadData(zp_addr)` → oper | `Alu(...)` or `SetA/X/Y(oper)` or `Bit` |

### 3.9 Zero Page Write — 3 cycles

**Opcodes:** `STA zp(0x85)`, `STX zp(0x86)`, `STY zp(0x84)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_addr | — |
| C3 | `WriteData(zp_addr, reg)` | — |

### 3.10 Zero Page Indexed X Read — 4 cycles

**Opcodes:** `ADC zp,X(0x75)`, `AND zp,X(0x35)`, `CMP zp,X(0xD5)`, `EOR zp,X(0x55)`,
`LDA zp,X(0xB5)`, `LDY zp,X(0xB4)`, `ORA zp,X(0x15)`, `SBC zp,X(0xF5)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_base | — |
| C3 | `ReadDummy(zp_base)` | `AddIndexX` → EA = zp_base + X |
| C4 | `ReadData(EA)` → oper | `Alu(...)` or `SetA/Y(oper)` |

### 3.11 Zero Page Indexed X Write — 4 cycles

**Opcodes:** `STA zp,X(0x95)`, `STY zp,X(0x94)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_base | — |
| C3 | `ReadDummy(zp_base)` | `AddIndexX` → EA |
| C4 | `WriteData(EA, reg)` | — |

### 3.12 Zero Page Indexed Y Read — 4 cycles

**Opcode:** `LDX zp,Y(0xB6)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_base | — |
| C3 | `ReadDummy(zp_base)` | `AddIndexY` → EA |
| C4 | `ReadData(EA)` → oper | `SetX(oper)` |

### 3.13 Zero Page Indexed Y Write — 4 cycles

**Opcode:** `STX zp,Y(0x96)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_base | — |
| C3 | `ReadDummy(zp_base)` | `AddIndexY` → EA |
| C4 | `WriteData(EA, X)` | — |

### 3.14 Absolute Read — 4 cycles

**Opcodes:** `ADC abs(0x6D)`, `AND abs(0x2D)`, `BIT abs(0x2C)`, `CMP abs(0xCD)`,
`CPX abs(0xEC)`, `CPY abs(0xCC)`, `EOR abs(0x4D)`, `LDA abs(0xAD)`,
`LDX abs(0xAE)`, `LDY abs(0xAC)`, `ORA abs(0x0D)`, `SBC abs(0xED)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress` → EA |
| C4 | `ReadData(EA)` → oper | `Alu(...)` or `SetA/X/Y(oper)` or `Bit` |

### 3.15 Absolute Write — 4 cycles

**Opcodes:** `STA abs(0x8D)`, `STX abs(0x8E)`, `STY abs(0x8C)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress` → EA |
| C4 | `WriteData(EA, reg)` | — |

### 3.16 Absolute Indexed X Read — 4/5 cycles

**Opcodes:** `ADC abs,X(0x7D)`, `AND abs,X(0x3D)`, `CMP abs,X(0xDD)`, `EOR abs,X(0x5D)`,
`LDA abs,X(0xBD)`, `LDY abs,X(0xBC)`, `ORA abs,X(0x1D)`, `SBC abs,X(0xFD)`

**Without page cross (4 cycles):**

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress`; `AddIndexX` → EA |
| C4 | `ReadData(EA)` → oper | `Alu(...)` or `SetA/Y(oper)` |

**With page cross (+1 cycle, 5 cycles total):**

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress`; `AddIndexX` → EA_wrong (bad page) |
| C4 | `ReadDummy(EA_wrong)` | Fix page → EA_correct |
| C5 | `ReadData(EA_correct)` → oper | `Alu(...)` or `SetA/Y(oper)` |

### 3.17 Absolute Indexed Y Read — 4/5 cycles

**Opcodes:** `ADC abs,Y(0x79)`, `AND abs,Y(0x39)`, `CMP abs,Y(0xD9)`, `EOR abs,Y(0x59)`,
`LDA abs,Y(0xB9)`, `LDX abs,Y(0xBE)`, `ORA abs,Y(0x19)`, `SBC abs,Y(0xF9)`

Same pattern as Absolute Indexed X Read, substituting `AddIndexY` and the Y register.

**Without page cross (4 cycles):**

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress`; `AddIndexY` → EA |
| C4 | `ReadData(EA)` → oper | `Alu(...)` or `SetA/X(oper)` |

**With page cross (5 cycles):** same fixup as 3.16.

### 3.18 Absolute Indexed X Write — 5 cycles (always)

**Opcode:** `STA abs,X(0x9D)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress`; `AddIndexX` → EA |
| C4 | `ReadDummy(EA)` | Always — even without page cross |
| C5 | `WriteData(EA, A)` | — |

**Why 5 cycles?** The 6502 always does a dummy read before indexed writes. Unlike reads, the penalty is always taken because the CPU needs the extra cycle to resolve the final address before committing the write.

### 3.19 Absolute Indexed Y Write — 5 cycles (always)

**Opcode:** `STA abs,Y(0x99)`

Same as 3.18, substituting Y.

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress`; `AddIndexY` → EA |
| C4 | `ReadDummy(EA)` | — |
| C5 | `WriteData(EA, A)` | — |

### 3.20 Indexed Indirect `(zp,X)` Read — 6 cycles

**Opcodes:** `ADC (zp,X)(0x61)`, `AND (zp,X)(0x21)`, `CMP (zp,X)(0xC1)`,
`EOR (zp,X)(0x41)`, `LDA (zp,X)(0xA1)`, `ORA (zp,X)(0x01)`, `SBC (zp,X)(0xE1)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_base | — |
| C3 | `ReadDummy(zp_base)` | `AddIndexX` → ptr_zp = zp_base + X |
| C4 | `ReadPtrLo(ptr_zp)` → addr_lo | — |
| C5 | `ReadPtrHi(ptr_zp + 1)` → addr_hi | `CombineAddress` → EA |
| C6 | `ReadData(EA)` → oper | `Alu(...)` or `SetA(oper)` |

### 3.21 Indexed Indirect `(zp,X)` Write — 6 cycles

**Opcode:** `STA (zp,X)(0x81)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_base | — |
| C3 | `ReadDummy(zp_base)` | `AddIndexX` → ptr_zp |
| C4 | `ReadPtrLo(ptr_zp)` → addr_lo | — |
| C5 | `ReadPtrHi(ptr_zp + 1)` → addr_hi | `CombineAddress` → EA |
| C6 | `WriteData(EA, A)` | — |

### 3.22 Indirect Indexed `(zp),Y` Read — 5/6 cycles

**Opcodes:** `ADC (zp),Y(0x71)`, `AND (zp),Y(0x31)`, `CMP (zp),Y(0xD1)`,
`EOR (zp),Y(0x51)`, `LDA (zp),Y(0xB1)`, `ORA (zp),Y(0x11)`, `SBC (zp),Y(0xF1)`

**Without page cross (5 cycles):**

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_ptr | — |
| C3 | `ReadPtrLo(zp_ptr)` → addr_lo | — |
| C4 | `ReadPtrHi(zp_ptr + 1)` → addr_hi | `CombineAddress`; `AddIndexY` → EA |
| C5 | `ReadData(EA)` → oper | `Alu(...)` or `SetA(oper)` |

**With page cross (6 cycles):** C4 produces wrong page → C5 is `ReadDummy(EA_wrong)` → C6 is `ReadData(EA_correct)`.

### 3.23 Indirect Indexed `(zp),Y` Write — 6 cycles (always)

**Opcode:** `STA (zp),Y(0x91)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_ptr | — |
| C3 | `ReadPtrLo(zp_ptr)` → addr_lo | — |
| C4 | `ReadPtrHi(zp_ptr + 1)` → addr_hi | `CombineAddress`; `AddIndexY` → EA |
| C5 | `ReadDummy(EA)` | Always — extra dummy read |
| C6 | `WriteData(EA, A)` | — |

### 3.24 JMP Absolute — 3 cycles

**Opcode:** `JMP abs(0x4C)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `SetPC(addr_hi:addr_lo)` |

### 3.25 JMP Indirect — 5 cycles

**Opcode:** `JMP (abs)(0x6C)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → ptr_lo | — |
| C3 | `ReadPC2` → ptr_hi | `CombineAddress` → ptr |
| C4 | `ReadData(ptr)` → PC_lo | — |
| C5 | `ReadData(ptr + 1)` → PC_hi | `SetPC(PC_hi:PC_lo)` |

> **NMOS 6502 bug**: When `ptr` is `$xxFF`, the high byte at C5 is fetched from `$xx00` instead of `$(xx+1)00`. The emulator must replicate this: wrap the low byte, do NOT increment the page.

### 3.26 JSR Absolute — 6 cycles

**Opcode:** `JSR abs(0x20)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → target_lo | `PushPC` (prepare return = PC+2) |
| C3 | `PullDummy` | Internal: stack pointer not yet decremented |
| C4 | `Push(PCH_of_return)` | SP-- |
| C5 | `Push(PCL_of_return)` | SP-- |
| C6 | `ReadPC2` → target_hi | `SetPC(target_hi:target_lo)` |

**Return address detail:** JSR pushes `PC+2` (last byte of the JSR instruction). RTS pops this and adds 1, landing on the next instruction. Push order is high byte first, then low byte (standard 6502 stack convention for addresses).

### 3.27 RTS — 6 cycles

**Opcode:** `RTS(0x60)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | Internal (no bus access) |
| C3 | `PullDummy` | — |
| C4 | `Pull` → PC_lo | — |
| C5 | `Pull` → PC_hi | `SetPC(PC_hi:PC_lo)` |
| C6 | `ReadDummy(new_PC)` | Increment PC by 1 (now points to return instruction) |

### 3.28 RTI — 6 cycles

**Opcode:** `RTI(0x40)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | Internal (no bus access) |
| C3 | `PullDummy` | — |
| C4 | `Pull` → status_byte | `status = (status_byte \| UNUSED) & !B` |
| C5 | `Pull` → PC_lo | — |
| C6 | `Pull` → PC_hi | `SetPC(PC_hi:PC_lo)` |

Unlike RTS, RTI does NOT add 1 to the restored PC — the pushed PC is the exact return address.

### 3.29 BRK — 7 cycles

**Opcode:** `BRK(0x00)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` — signature byte, discarded | Advance internal PC past signature |
| C3 | `Push(PCH_of_PC+2)` | SP-- |
| C4 | `Push(PCL_of_PC+2)` | SP-- |
| C5 | `Push(status \| B \| UNUSED)` | SP-- ; `SetFlagI(true)` |
| C6 | `ReadData(0xFFFE)` → vec_lo | — |
| C7 | `ReadData(0xFFFF)` → vec_hi | `SetPC(vec_hi:vec_lo)` |

Key details:
- The signature byte at PC+1 is read but **discarded**. BRK is technically a 2-byte instruction.
- The pushed PC is `PC+2` (skipping the signature byte).
- The B flag is set in the **pushed** status but not in the P register.
- I flag is set **after** the push (pushed status has I clear unless it was already set).
- BRK uses the IRQ vector (`$FFFE–$FFFF`), same as IRQ and RESET.

### 3.30 Branches — 2/3/4 cycles

**Opcodes:** `BCC(0x90)`, `BCS(0xB0)`, `BEQ(0xF0)`, `BMI(0x30)`,
`BNE(0xD0)`, `BPL(0x10)`, `BVC(0x50)`, `BVS(0x70)`

**Not taken (2 cycles):**

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → offset | Check condition → false. Done. |

**Taken, same page (3 cycles):**

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → offset | Check condition → true. `BranchCalc` → new_PC (same page). |
| C3 | `ReadDummy(new_PC)` | `SetPC(new_PC)` |

**Taken, different page (4 cycles):**

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → offset | Check condition → true. `BranchCalc` → new_PC (page crossed). |
| C3 | `ReadDummy(PC+2 wrong page)` | Fix PCH |
| C4 | `ReadDummy(new_PC)` | `SetPC(new_PC)` |

### 3.31 Read-Modify-Write (RMW) Instructions

RMW instructions (ASL, LSR, ROL, ROR, INC, DEC on memory) read a value, write the original back (dummy), then write the modified value. This dummy-write behavior is critical for hardware (e.g., clearing VIA interrupt flags on register reads).

#### Accumulator RMW — 2 cycles

**Opcodes:** `ASL A(0x0A)`, `LSR A(0x4A)`, `ROL A(0x2A)`, `ROR A(0x6A)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | — | `Alu(Asl,Lsr,Rol,Ror)` on A. Sets C,Z,N. |

#### Zero Page RMW — 5 cycles

**Opcodes:** `ASL zp(0x06)`, `DEC zp(0xC6)`, `INC zp(0xE6)`, `LSR zp(0x46)`,
`ROL zp(0x26)`, `ROR zp(0x66)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → EA | — |
| C3 | `ReadData(EA)` → value | — |
| C4 | `WriteDummy(EA, value)` | `Alu(...)` → modified |
| C5 | `WriteData(EA, modified)` | Set C,Z,N from modified |

#### Zero Page Indexed X RMW — 6 cycles

**Opcodes:** `ASL zp,X(0x16)`, `DEC zp,X(0xD6)`, `INC zp,X(0xF6)`, `LSR zp,X(0x56)`,
`ROL zp,X(0x36)`, `ROR zp,X(0x76)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadZP` → zp_base | — |
| C3 | `ReadDummy(zp_base)` | `AddIndexX` → EA |
| C4 | `ReadData(EA)` → value | — |
| C5 | `WriteDummy(EA, value)` | `Alu(...)` → modified |
| C6 | `WriteData(EA, modified)` | Set C,Z,N from modified |

#### Absolute RMW — 6 cycles

**Opcodes:** `ASL abs(0x0E)`, `DEC abs(0xCE)`, `INC abs(0xEE)`, `LSR abs(0x4E)`,
`ROL abs(0x2E)`, `ROR abs(0x6E)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress` → EA |
| C4 | `ReadData(EA)` → value | — |
| C5 | `WriteDummy(EA, value)` | `Alu(...)` → modified |
| C6 | `WriteData(EA, modified)` | Set C,Z,N from modified |

#### Absolute Indexed X RMW — 7 cycles

**Opcodes:** `ASL abs,X(0x1E)`, `DEC abs,X(0xDE)`, `INC abs,X(0xFE)`, `LSR abs,X(0x5E)`,
`ROL abs,X(0x3E)`, `ROR abs,X(0x7E)`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | `Fetch` | — |
| C2 | `ReadPC1` → addr_lo | — |
| C3 | `ReadPC2` → addr_hi | `CombineAddress`; `AddIndexX` → EA |
| C4 | `ReadData(EA)` → value | Always reads — if page crossed, C4 is dummy read of wrong page |
| C5 | `ReadData(EA)` → value | If page crossed: correct page read. If same page: redundant read of same address. |
| C6 | `WriteDummy(EA, value)` | `Alu(...)` → modified |
| C7 | `WriteData(EA, modified)` | Set C,Z,N |

**Why always 7 cycles?** The 6502 always takes the indexed addressing penalty for RMW operations. Even when no page is crossed, the address computation still needs an extra cycle (C5 redundant read). This is a hardware constraint of the NMOS 6502.

---

## 4. Interrupt Micro-Operations

Interrupts are not instructions per se, but they consume cycles and perform bus operations. For cycle-perfect emulation, interrupts must be broken into micro-operations too.

### 4.1 NMI / IRQ — 7 cycles

**Vector:** NMI = `$FFFA`, IRQ = `$FFFE`

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | — | Internal (interrupt recognized, no bus access yet) — fetch suppressed |
| C2 | — | Internal (no bus access) |
| C3 | `Push(PCH)` | SP-- |
| C4 | `Push(PCL)` | SP-- |
| C5 | `Push(status)` — B=0 (unlike BRK), UNUSED=1 | SP-- ; `SetFlagI(true)` |
| C6 | `ReadData(VECTOR_LO)` → vec_lo | — |
| C7 | `ReadData(VECTOR_HI)` → vec_hi | `SetPC(vec_hi:vec_lo)` |

> NMI is edge-triggered (detected on the falling edge of the NMI line); IRQ is level-triggered (sampled when `IRQ` line is low and I flag is clear).

### 4.2 RESET — 7 cycles

**Vector:** `$FFFC`

Reset is not a regular instruction. The first 7 cycles after reset perform:

| Cycle | Bus | Internal |
|-------|-----|----------|
| C1 | — | Internal |
| C2 | — | Internal |
| C3 | — | Internal |
| C4 | `ReadDummy(0x0100 + SP)` | SP-- |
| C5 | `ReadDummy(0x0100 + SP)` | SP-- |
| C6 | `ReadData(0xFFFC)` → PC_lo | — |
| C7 | `ReadData(0xFFFD)` → PC_hi | `SetPC(PC_hi:PC_lo)` |

The reset sequence decrements SP by 3 (dummy stack writes are reads on the 6502 — the SP decrement is internal) and loads the reset vector.

---

## 5. Complete Opcode Matrix

| Opcode | Instruction | Addressing Mode | Cycles | Section |
|--------|------------|----------------|--------|---------|
| 0x00 | BRK | Implied | 7 | 3.29 |
| 0x01 | ORA | (zp,X) | 6 | 3.20 |
| 0x05 | ORA | zp | 3 | 3.8 |
| 0x06 | ASL | zp | 5 | 3.31 |
| 0x08 | PHP | Implied | 3 | 3.5 |
| 0x09 | ORA | # | 2 | 3.7 |
| 0x0A | ASL | A | 2 | 3.31 |
| 0x0D | ORA | abs | 4 | 3.14 |
| 0x0E | ASL | abs | 6 | 3.31 |
| 0x10 | BPL | rel | 2/3/4 | 3.30 |
| 0x11 | ORA | (zp),Y | 5/6 | 3.22 |
| 0x15 | ORA | zp,X | 4 | 3.10 |
| 0x16 | ASL | zp,X | 6 | 3.31 |
| 0x18 | CLC | Implied | 2 | 3.1 |
| 0x19 | ORA | abs,Y | 4/5 | 3.17 |
| 0x1D | ORA | abs,X | 4/5 | 3.16 |
| 0x1E | ASL | abs,X | 7 | 3.31 |
| 0x20 | JSR | abs | 6 | 3.26 |
| 0x21 | AND | (zp,X) | 6 | 3.20 |
| 0x24 | BIT | zp | 3 | 3.8 |
| 0x25 | AND | zp | 3 | 3.8 |
| 0x26 | ROL | zp | 5 | 3.31 |
| 0x28 | PLP | Implied | 4 | 3.6 |
| 0x29 | AND | # | 2 | 3.7 |
| 0x2A | ROL | A | 2 | 3.31 |
| 0x2C | BIT | abs | 4 | 3.14 |
| 0x2D | AND | abs | 4 | 3.14 |
| 0x2E | ROL | abs | 6 | 3.31 |
| 0x30 | BMI | rel | 2/3/4 | 3.30 |
| 0x31 | AND | (zp),Y | 5/6 | 3.22 |
| 0x35 | AND | zp,X | 4 | 3.10 |
| 0x36 | ROL | zp,X | 6 | 3.31 |
| 0x38 | SEC | Implied | 2 | 3.1 |
| 0x39 | AND | abs,Y | 4/5 | 3.17 |
| 0x3D | AND | abs,X | 4/5 | 3.16 |
| 0x3E | ROL | abs,X | 7 | 3.31 |
| 0x40 | RTI | Implied | 6 | 3.28 |
| 0x41 | EOR | (zp,X) | 6 | 3.20 |
| 0x45 | EOR | zp | 3 | 3.8 |
| 0x46 | LSR | zp | 5 | 3.31 |
| 0x48 | PHA | Implied | 3 | 3.5 |
| 0x49 | EOR | # | 2 | 3.7 |
| 0x4A | LSR | A | 2 | 3.31 |
| 0x4C | JMP | abs | 3 | 3.24 |
| 0x4D | EOR | abs | 4 | 3.14 |
| 0x4E | LSR | abs | 6 | 3.31 |
| 0x50 | BVC | rel | 2/3/4 | 3.30 |
| 0x51 | EOR | (zp),Y | 5/6 | 3.22 |
| 0x55 | EOR | zp,X | 4 | 3.10 |
| 0x56 | LSR | zp,X | 6 | 3.31 |
| 0x58 | CLI | Implied | 2 | 3.1 |
| 0x59 | EOR | abs,Y | 4/5 | 3.17 |
| 0x5D | EOR | abs,X | 4/5 | 3.16 |
| 0x5E | LSR | abs,X | 7 | 3.31 |
| 0x60 | RTS | Implied | 6 | 3.27 |
| 0x61 | ADC | (zp,X) | 6 | 3.20 |
| 0x65 | ADC | zp | 3 | 3.8 |
| 0x66 | ROR | zp | 5 | 3.31 |
| 0x68 | PLA | Implied | 4 | 3.6 |
| 0x69 | ADC | # | 2 | 3.7 |
| 0x6A | ROR | A | 2 | 3.31 |
| 0x6C | JMP | (abs) | 5 | 3.25 |
| 0x6D | ADC | abs | 4 | 3.14 |
| 0x6E | ROR | abs | 6 | 3.31 |
| 0x70 | BVS | rel | 2/3/4 | 3.30 |
| 0x71 | ADC | (zp),Y | 5/6 | 3.22 |
| 0x75 | ADC | zp,X | 4 | 3.10 |
| 0x76 | ROR | zp,X | 6 | 3.31 |
| 0x78 | SEI | Implied | 2 | 3.1 |
| 0x79 | ADC | abs,Y | 4/5 | 3.17 |
| 0x7D | ADC | abs,X | 4/5 | 3.16 |
| 0x7E | ROR | abs,X | 7 | 3.31 |
| 0x81 | STA | (zp,X) | 6 | 3.21 |
| 0x84 | STY | zp | 3 | 3.9 |
| 0x85 | STA | zp | 3 | 3.9 |
| 0x86 | STX | zp | 3 | 3.9 |
| 0x88 | DEY | Implied | 2 | 3.4 |
| 0x8A | TXA | Implied | 2 | 3.3 |
| 0x8C | STY | abs | 4 | 3.15 |
| 0x8D | STA | abs | 4 | 3.15 |
| 0x8E | STX | abs | 4 | 3.15 |
| 0x90 | BCC | rel | 2/3/4 | 3.30 |
| 0x91 | STA | (zp),Y | 6 | 3.23 |
| 0x94 | STY | zp,X | 4 | 3.11 |
| 0x95 | STA | zp,X | 4 | 3.11 |
| 0x96 | STX | zp,Y | 4 | 3.13 |
| 0x98 | TYA | Implied | 2 | 3.3 |
| 0x99 | STA | abs,Y | 5 | 3.19 |
| 0x9A | TXS | Implied | 2 | 3.3 |
| 0x9D | STA | abs,X | 5 | 3.18 |
| 0xA0 | LDY | # | 2 | 3.7 |
| 0xA1 | LDA | (zp,X) | 6 | 3.20 |
| 0xA2 | LDX | # | 2 | 3.7 |
| 0xA4 | LDY | zp | 3 | 3.8 |
| 0xA5 | LDA | zp | 3 | 3.8 |
| 0xA6 | LDX | zp | 3 | 3.8 |
| 0xA8 | TAY | Implied | 2 | 3.3 |
| 0xA9 | LDA | # | 2 | 3.7 |
| 0xAA | TAX | Implied | 2 | 3.3 |
| 0xAC | LDY | abs | 4 | 3.14 |
| 0xAD | LDA | abs | 4 | 3.14 |
| 0xAE | LDX | abs | 4 | 3.14 |
| 0xB0 | BCS | rel | 2/3/4 | 3.30 |
| 0xB1 | LDA | (zp),Y | 5/6 | 3.22 |
| 0xB4 | LDY | zp,X | 4 | 3.10 |
| 0xB5 | LDA | zp,X | 4 | 3.10 |
| 0xB6 | LDX | zp,Y | 4 | 3.12 |
| 0xB8 | CLV | Implied | 2 | 3.1 |
| 0xB9 | LDA | abs,Y | 4/5 | 3.17 |
| 0xBA | TSX | Implied | 2 | 3.3 |
| 0xBC | LDY | abs,X | 4/5 | 3.16 |
| 0xBD | LDA | abs,X | 4/5 | 3.16 |
| 0xBE | LDX | abs,Y | 4/5 | 3.17 |
| 0xC0 | CPY | # | 2 | 3.7 |
| 0xC1 | CMP | (zp,X) | 6 | 3.20 |
| 0xC4 | CPY | zp | 3 | 3.8 |
| 0xC5 | CMP | zp | 3 | 3.8 |
| 0xC6 | DEC | zp | 5 | 3.31 |
| 0xC8 | INY | Implied | 2 | 3.4 |
| 0xC9 | CMP | # | 2 | 3.7 |
| 0xCA | DEX | Implied | 2 | 3.4 |
| 0xCC | CPY | abs | 4 | 3.14 |
| 0xCD | CMP | abs | 4 | 3.14 |
| 0xCE | DEC | abs | 6 | 3.31 |
| 0xD0 | BNE | rel | 2/3/4 | 3.30 |
| 0xD1 | CMP | (zp),Y | 5/6 | 3.22 |
| 0xD5 | CMP | zp,X | 4 | 3.10 |
| 0xD6 | DEC | zp,X | 6 | 3.31 |
| 0xD8 | CLD | Implied | 2 | 3.1 |
| 0xD9 | CMP | abs,Y | 4/5 | 3.17 |
| 0xDD | CMP | abs,X | 4/5 | 3.16 |
| 0xDE | DEC | abs,X | 7 | 3.31 |
| 0xE0 | CPX | # | 2 | 3.7 |
| 0xE1 | SBC | (zp,X) | 6 | 3.20 |
| 0xE4 | CPX | zp | 3 | 3.8 |
| 0xE5 | SBC | zp | 3 | 3.8 |
| 0xE6 | INC | zp | 5 | 3.31 |
| 0xE8 | INX | Implied | 2 | 3.4 |
| 0xE9 | SBC | # | 2 | 3.7 |
| 0xEA | NOP | Implied | 2 | 3.2 |
| 0xEC | CPX | abs | 4 | 3.14 |
| 0xED | SBC | abs | 4 | 3.14 |
| 0xEE | INC | abs | 6 | 3.31 |
| 0xF0 | BEQ | rel | 2/3/4 | 3.30 |
| 0xF1 | SBC | (zp),Y | 5/6 | 3.22 |
| 0xF5 | SBC | zp,X | 4 | 3.10 |
| 0xF6 | INC | zp,X | 6 | 3.31 |
| 0xF8 | SED | Implied | 2 | 3.1 |
| 0xF9 | SBC | abs,Y | 4/5 | 3.17 |
| 0xFD | SBC | abs,X | 4/5 | 3.16 |
| 0xFE | INC | abs,X | 7 | 3.31 |

---

## 6. Implementation Design

### 6.1 Overview

Replace the current monolithic `execute_instruction()` + `step()` state machine with a **micro-operation sequencer**. Each opcode maps to a static `InstructionSequence` (a `Vec<MicroCycle>`). The CPU's `step()` function advances through the sequence, executing one `MicroCycle` per call.

```
Current:  step() → [decode + operand fetch + atomic execute]
Proposed: step() → advance sequence pointer → execute one MicroCycle
```

### 6.2 Instruction Sequence Table

A `const` lookup table indexed by opcode, generated at compile time:

```rust
/// Pre-computed instruction sequences for all 256 opcodes.
/// Index by opcode byte.
static INSTRUCTION_SEQUENCES: [InstructionSequence; 256] = build_sequences();

const fn build_sequences() -> [InstructionSequence; 256] {
    // Generated by a build script or const fn.
    // Each entry is a Vec<MicroCycle> matching the cycle counts above.
    // For now, construct at startup with lazy_static/OnceLock.
}
```

Since `Vec` in `const` context is limited (requires `#![feature(const_vec_new)]` on some Rust versions, or we use arrays with sentinel lengths), we can either:

1. **Use `&'static [MicroCycle]` with static slices** — store sequences as static arrays, reference via slice.
2. **Use `OnceLock<[Vec<MicroCycle>; 256]>`** — initialize once at startup.
3. **Use `phf` or a build script** — generate the table in a `build.rs`.

**Recommended**: Use a build script (`build.rs`) to generate `src/cpu/instruction_sequences.rs` containing the static table. This avoids const-eval limitations and is trivially debuggable.

### 6.3 Modified CPU State

```rust
pub struct CPU6502 {
    pub registers: Registers,
    
    // Instruction sequencing
    current_sequence: &'static [MicroCycle],   // Pointer into sequence table
    sequence_index: usize,                       // Current cycle within sequence
    operands_buffer: [u8; 2],                    // Operand bytes (populated during execution)
    effective_address: u16,                      // Computed target address
    operand_value: u8,                           // Latest read value
    alu_value: u8,                               // For RMW: modified value between read and write
    branch_target: u16,                          // Computed branch destination
    
    // Counters
    total_cycles: u64,
    last_performance_log_cycle: u64,
    last_performance_log_instant: Instant,
    
    // Interrupt state
    irq_line_low: bool,
    nmi_latch: EdgeLatch,
    interrupt_sequence: &'static [MicroCycle],   // Interrupt micro-op sequence
    interrupt_sequence_index: usize,
    
    // Debug
    breakpoints: Vec<Box<dyn Breakpoint>>,
    instruction_tracking: InstructionTracking,
}
```

### 6.4 Step Function Redesign

```rust
pub fn step(&mut self, memory: &mut impl Addressable) {
    // 1. Interrupt handling (before instruction execution)
    if self.sequence_index == 0 {
        if self.nmi_latch.take() {
            self.start_interrupt(Interrupt::NMI);
            // Fall through to execute first interrupt micro-op
        } else if self.irq_line_low && !self.registers.is_flag_set(INTERRUPT_FLAG_BITMASK) {
            self.start_interrupt(Interrupt::IRQ);
        }
    }
    
    // 2. Cycle counting
    self.total_cycles += 1;
    self.performance_log();
    
    // 3. Execute one micro-cycle
    if self.interrupt_sequence_index > 0 {
        self.execute_interrupt_micro_cycle(memory);
    } else if let Some(cycle) = self.current_sequence.get(self.sequence_index) {
        self.execute_micro_cycle(cycle, memory);
        self.sequence_index += 1;
        
        // End of instruction
        if self.sequence_index >= self.current_sequence.len() {
            self.sequence_index = 0;
            self.current_sequence = &[]; // Will be set on next fetch
            self.breakpoints.iter().for_each(|bp| bp.on_hit(self.registers.pc));
        }
    }
    
    // 4. Post-instruction interrupt checks
    if self.sequence_index == 0 && self.interrupt_sequence_index == 0 {
        if self.nmi_latch.take() {
            self.start_interrupt(Interrupt::NMI);
        }
    }
}

fn execute_micro_cycle(&mut self, cycle: &MicroCycle, memory: &mut impl Addressable) {
    match &cycle.bus {
        BusOp::Fetch => {
            let opcode = memory.read_byte(self.registers.pc);
            let info = decode(opcode);
            self.current_sequence = &INSTRUCTION_SEQUENCES[opcode as usize];
            self.instruction_tracking.current_instruction_info = Some(info);
        }
        BusOp::ReadPC1 => {
            self.operands_buffer[0] = memory.read_byte(self.registers.pc.wrapping_add(1));
        }
        BusOp::ReadPC2 => {
            self.operands_buffer[1] = memory.read_byte(self.registers.pc.wrapping_add(2));
        }
        BusOp::ReadZP => {
            let zp = self.operands_buffer[0];
            self.effective_address = zp as u16;
        }
        BusOp::ReadData(addr) => {
            self.operand_value = memory.read_byte(*addr);
        }
        BusOp::ReadDummy(addr) => {
            let _ = memory.read_byte(*addr); // Discarded
        }
        BusOp::ReadPtrLo => {
            let zp = self.effective_address; // Already computed
            self.effective_address = memory.read_zero_page_byte(zp as u8) as u16;
        }
        BusOp::ReadPtrHi => {
            let zp = self.effective_address.wrapping_add(1);
            let hi = memory.read_zero_page_byte(zp as u8);
            self.effective_address = (hi as u16) << 8 | (self.effective_address & 0xFF);
        }
        BusOp::WriteData(addr, value) => {
            memory.write_byte(*addr, *value);
        }
        BusOp::WriteDummy(addr, value) => {
            memory.write_byte(*addr, *value); // Write original value back
        }
        BusOp::Push(value) => {
            memory.write_byte(0x0100 + self.registers.sp as u16, *value);
            self.registers.sp = self.registers.sp.wrapping_sub(1);
        }
        BusOp::PullDummy => {
            let _ = memory.read_byte(0x0100 + self.registers.sp as u16);
        }
        BusOp::Pull => {
            self.registers.sp = self.registers.sp.wrapping_add(1);
            self.operand_value = memory.read_byte(0x0100 + self.registers.sp as u16);
        }
    }
    
    // Internal operations run after the bus operation
    match &cycle.internal {
        Internal::None => {}
        Internal::SetFlagC(v) => self.registers.update_carry_flag(*v),
        Internal::SetFlagD(v) => self.registers.update_decimal_flag(*v),
        Internal::SetFlagI(v) => self.registers.update_interrupt_flag(*v),
        Internal::SetFlagV(v) => self.registers.update_overflow_flag(*v),
        Internal::SetA(v) => self.registers.set_accumulator(*v),
        Internal::SetX(v) => self.registers.set_x(*v),
        Internal::SetY(v) => self.registers.set_y(*v),
        Internal::TxS => self.registers.sp = self.registers.x,
        Internal::Inx => self.registers.set_x(self.registers.x.wrapping_add(1)),
        Internal::Iny => self.registers.set_y(self.registers.y.wrapping_add(1)),
        Internal::Dex => self.registers.set_x(self.registers.x.wrapping_sub(1)),
        Internal::Dey => self.registers.set_y(self.registers.y.wrapping_sub(1)),
        Internal::AddIndexX => {
            self.effective_address = self.effective_address.wrapping_add(self.registers.x as u16);
        }
        Internal::AddIndexY => {
            self.effective_address = self.effective_address.wrapping_add(self.registers.y as u16);
        }
        Internal::CombineAddress => {
            // hi byte is in operands_buffer[1], lo is in operands_buffer[0]
            self.effective_address = (self.operands_buffer[1] as u16) << 8 | self.operands_buffer[0] as u16;
        }
        Internal::BranchCalc => {
            let offset = self.operands_buffer[0] as i8 as i16;
            self.branch_target = self.registers.pc.wrapping_add(2).wrapping_add(offset as u16);
        }
        Internal::SetPC(addr) => self.registers.pc = *addr,
        Internal::PushPC => {
            // Store PC+2 for later pushing
            self.effective_address = self.registers.pc.wrapping_add(2); // Temp use of EA field
        }
        Internal::Alu(op) => execute_alu(op, &mut self.registers, self.operand_value, &mut self.alu_value),
    }
}
```

### 6.5 Address Computation Notes

The micro-operation sequences are **static** — they don't contain variable addresses. Instead, the sequencer uses CPU state fields to carry addresses forward:

- `effective_address` — accumulates the target address as it's built across cycles (ReadPC1 + ReadPC2 + AddIndexX → final EA)
- `operand_value` — stores the last data byte read from memory
- `alu_value` — for RMW: stores the modified value between the dummy write and final write
- `branch_target` — for branches: stores the computed destination
- `operands_buffer` — raw operand bytes from the instruction stream (PC+1, PC+2)

This avoids dynamic allocation per instruction while keeping the sequences purely declarative.

### 6.6 Page Crossing Detection

For indexed modes that may cross pages, the micro-operations need to signal whether a page crossing occurred. This can be done with:

1. **Conditional sequences**: The sequence table includes both `WITHOUT_CROSS` and `WITH_CROSS` variants. The `AddIndexX`/`AddIndexY` internal op sets a flag; if page crossed, skip the `ReadData` micro-cycle and instead execute the `ReadDummy` → `ReadData` pair.

2. **Dynamic skip**: The `AddIndexX`/`AddIndexY` internal op computes whether a page was crossed and stores a skip count. The sequencer advances past micro-cycles that are marked as conditional.

The simpler approach (and more testable) is **conditional sequences** — the sequence table has two entries per page-cross-capable opcode, and on C3 (when the page cross is detected), the sequencer swaps to the extended sequence.

### 6.7 Build Script Generation

`build.rs` generates `src/cpu/instruction_sequences.rs`:

```rust
// build.rs
use std::io::Write;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = std::path::Path::new(&out_dir).join("instruction_sequences.rs");
    let mut f = std::fs::File::create(dest).unwrap();
    
    writeln!(f, "// Auto-generated by build.rs — do not edit").unwrap();
    writeln!(f, "use crate::cpu::cycle_perfect::{{BusOp, Internal, MicroCycle}};").unwrap();
    writeln!(f).unwrap();
    
    // For each opcode 0x00..0xFF:
    for opcode in 0x00u8..=0xFF {
        let seq = generate_sequence(opcode);
        writeln!(f, "pub const SEQ_{:02X}: &[MicroCycle] = &[", opcode).unwrap();
        for cycle in &seq {
            writeln!(f, "    MicroCycle {{ bus: {:?}, internal: {:?} }},", 
                     cycle.bus, cycle.internal).unwrap();
        }
        writeln!(f, "];").unwrap();
    }
    
    // Build the lookup table
    writeln!(f, "pub static SEQUENCES: [&[MicroCycle]; 256] = [").unwrap();
    for opcode in 0x00u8..=0xFF {
        writeln!(f, "    SEQ_{:02X},", opcode).unwrap();
    }
    writeln!(f, "];").unwrap();
}
```

Then in `cpu6502.rs`:

```rust
include!(concat!(env!("OUT_DIR"), "/instruction_sequences.rs"));

pub fn step(&mut self, memory: &mut impl Addressable) {
    // ...
    if self.sequence_index == 0 {
        let opcode = memory.read_byte(self.registers.pc);
        self.current_sequence = SEQUENCES[opcode as usize];
    }
    // ...
}
```

---

## 7. Migration Strategy

This is a large refactor. A phased approach mitigates risk:

### Phase 1: Core Infrastructure (no behavior change)
1. Add `MicroOp`, `Internal`, `MicroCycle` types to `src/cpu/cycle_perfect.rs`
2. Add `MicroCycleExecutor` trait (implemented by `CPU6502`) that dispatches `MicroCycle` bus/internal operations
3. Write the build script to generate static sequences
4. Add unit tests for each micro-operation type in isolation
5. Add the sequence fields to `CPU6502` (gated behind `#[cfg(feature = "cycle-perfect")]`)

### Phase 2: Instruction Groups, One at a Time
For each instruction group, in order of increasing complexity:
1. Register transfers (TAX, TXA, etc.) — simplest, 2 cycles
2. Flag set/clear (CLC, SEC, etc.)
3. Immediate mode
4. Zero page read/write
5. Absolute read/write
6. Indexed modes (with page crossing)
7. Branches (with taken/not-taken/page-cross)
8. Stack operations (PHA, PLA, etc.)
9. JMP/JSR/RTS/RTI
10. RMW instructions (with dummy writes)
11. Indirect modes (JMP indirect, (zp,X), (zp),Y)
12. BRK

For each group:
- Write integration tests comparing old vs new behavior (identical register output, identical cycle count)
- Verify against test ROMs (Klaus Dormann 6502 functional test, etc.)
- Run `cargo test --lib` after each group

### Phase 3: Interrupts
1. Convert NMI/IRQ/RESET handling to micro-operation sequences
2. Verify interrupt timing (NMI edge detection, IRQ level sampling)
3. Test with programs that use interrupts

### Phase 4: Cleanup
1. Remove old `execute_instruction()`, `resolve_value()`, `resolve_address()`, `OperandResolution` trait
2. Remove `InstructionExecutor` trait
3. Simplify `InstructionTracking` (it becomes a thin wrapper over the new state)
4. Remove feature gate, make cycle-perfect the default
5. Run full test suite + manual smoke test with known ROMs

### Phase 5: Illegal Opcodes (Optional)
The NMOS 6502 has documented behavior for "illegal" opcodes (e.g., `LAX`, `SAX`, `DCP`, `ISB`, `SLO`, `RLA`, `SRE`, `RRA`). These can be added as additional entries in the sequence table once the framework is stable.

---

## 8. Verification Strategy

### 8.1 Automated Tests

Each instruction group must pass:
- **Cycle count test**: `cpu.total_cycles()` after instruction = expected cycle count
- **Register output test**: A, X, Y, SP, PC, status match expected values
- **Memory test**: All reads and writes go to correct addresses with correct values
- **Bus trace test**: The exact sequence of `(cycle_number, address, r/w, value)` matches a golden trace from a known-good emulator (e.g., Visual 6502)

### 8.2 Golden Trace Comparison

Generate JSON bus traces from the emulator and compare against traces from Visual 6502 or the Nintendulator/NES emulator test suite:

```json
{
  "instruction": "LDA $1234,X",
  "opcode": "0xBD",
  "cycles": [
    {"c": 1, "addr": "0x8000", "rw": "r", "val": "0xBD"},
    {"c": 2, "addr": "0x8001", "rw": "r", "val": "0x34"},
    {"c": 3, "addr": "0x8002", "rw": "r", "val": "0x12"},
    {"c": 4, "addr": "0x1235", "rw": "r", "val": "0x42"}
  ]
}
```

### 8.3 Functional Test ROMs

Run against:
- **Klaus Dormann 6502 functional test** — comprehensive opcode tests
- **Bruce Clark 6502 BCD test** — decimal mode accuracy
- **NES CPU interrupt tests** — NMI/IRQ/BRK timing

---

## 9. References

- [Visual 6502](http://www.visual6502.org/) — transistor-level simulation showing exact bus activity
- [6502.org Opcodes Tutorial](http://www.6502.org/tutorials/6502opcodes.html) — instruction set reference with cycle breakdowns
- [NESdev Wiki: CPU](https://www.nesdev.org/wiki/CPU) — cycle-by-cycle behavior for NMOS 6502
- [obelisk 6502 Reference](https://www.nesdev.org/obelisk-6502-guide/reference.html) — addressing mode details
- [mass:werk 6502 Instruction Set](https://www.masswerk.at/6502/6502_instruction_set.html) — comprehensive cycle timing
- [MOS 6502 Datasheet](http://archive.6502.org/datasheets/mos_6500_mpu_nov_1985.pdf) — official timing specifications
