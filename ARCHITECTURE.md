# ARCHITECTURE.md

In-depth architectural guide for the `rusty-vic20` VIC-20 emulator.

## Table of Contents

1. [Project Structure & Build System](#1-project-structure--build-system)
2. [Module Map](#2-module-map)
3. [The `Addressable` Trait](#3-the-addressable-trait)
4. [Bus — 64KB Address Space Routing](#4-bus--64kb-address-space-routing)
5. [Memory](#5-memory)
6. [CPU Subsystem](#6-cpu-subsystem)
7. [VIC-I Chip](#7-vic-i-chip)
8. [VIA 6522 Chip](#8-via-6522-chip)
9. [Virtual Clock](#9-virtual-clock)
10. [Keyboard Subsystem](#10-keyboard-subsystem)
11. [UI — Screen Window](#11-ui--screen-window)
12. [UI — Keyboard Window](#12-ui--keyboard-window)
13. [Debug Subsystem](#13-debug-subsystem)
14. [Tools — Breakpoints, Watchpoints, Disassembler](#14-tools--breakpoints-watchpoints-disassembler)
15. [Controller — Threading & Event Loop](#15-controller--threading--event-loop)
16. [Data Flow Summary](#16-data-flow-summary)
17. [Design Patterns & Pitfalls](#17-design-patterns--pitfalls)

## 1. Project Structure & Build System

**`Cargo.toml`** — Single crate (not a workspace), Rust `edition = "2024"` (requires >= 1.85.0).

**Key runtime dependencies:**
- `winit 0.30.13` — windowing/event loop (3 windows: screen, keyboard, debug)
- `pixels 0.17.1` — GPU-accelerated pixel buffer rendering
- `image 0.25.10` — PNG decoding (keyboard layout image)
- `font8x8 0.3.1` — embedded 8x8 bitmap font for debug overlays
- `log 0.4.29` / `env_logger 0.11.10` — logging infrastructure

**Dev-only dependencies:**
- `rstest 0.26.1` — fixture-based parametrized tests
- `unimock 0.6.8` — trait-object mocking framework

**Nightly features:** `#![feature(test)]` used only in `benches/run_bench.rs`.

**Binary targets:**
| Binary | Entrypoint | Purpose |
|--------|-----------|---------|
| `vic20` | `src/bin/vic20.rs` | Main emulator, accepts optional `tick-duration` in microseconds |
| `disassembler` | `src/bin/disassembler.rs` | Standalone 6502 disassembler, accepts `<file> [base] [start]` |

**`rustfmt.toml`** — `max_width=120`, `use_field_init_shorthand`, imports granularity `"Crate"`, `unstable_features=true`.

## 2. Module Map

```
src/lib.rs
├── addressable.rs
├── bus.rs
├── controller.rs
├── cpu/
│   ├── cpu6502.rs
│   ├── registers.rs
│   ├── addressing_mode.rs
│   ├── instructions.rs
│   ├── instruction_executor.rs
│   ├── instruction_tracking.rs
│   └── interrupt_handler.rs
├── debug/
│   ├── mod.rs
│   └── display.rs
├── keyboard.rs
├── memory.rs
├── tools/
│   ├── debug.rs
│   └── disassembler.rs
├── ui/
│   ├── mod.rs
│   ├── screen/
│   │   ├── display.rs
│   │   └── renderer.rs
│   └── keyboard/
│       ├── mod.rs
│       ├── key.rs
│       └── display.rs
├── via.rs
├── vic.rs
└── virtual_clock.rs
```

All top-level modules re-exported via `pub mod` in `src/lib.rs`.

## 3. The `Addressable` Trait

**File:** `src/addressable.rs`

The foundational trait that unifies all readable/writable entities on the 64KB bus:

```rust
pub trait Addressable {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);
}
```

**Provided convenience methods:**
- `read_zero_page_byte(address: u8) -> u8` / `write_zero_page_byte(address: u8, value: u8)` — casts `u8` to `u16` for zero-page (`0x00`–`0xFF`) access
- `read_word(address: u16) -> u16` / `write_word(address: u16, value: u16)` — little-endian 16-bit access (low byte first, high byte at `address+1`)
- `read_zero_page_word(address: u8) -> u16` / `write_zero_page_word(address: u8, value: u16)` — convenient zero-page word access

**Implementors:**
- `Memory` (`[u8; 65536]`) — RAM + ROM backing store
- `VIC` — video chip registers at `0x9000`–`0x900F`
- `VIA` — I/O chip registers at `0x9110`–`0x911F` (VIA1) and `0x9120`–`0x912F` (VIA2)
- `Bus` — the top-level address router that delegates to devices + memory

**Test support:** A `#[cfg(test)]` `Debug` impl exists for `dyn Addressable` because `unimock` requires `Debug` on trait objects. The stub `UnimplementedAddressable` (returns `0xFF` on reads, no-ops on writes) is used in VIA tests.

The CPU interacts with the bus exclusively through `dyn Addressable` or `impl Addressable` — it never directly touches memory or device structs.

## 4. Bus — 64KB Address Space Routing

**File:** `src/bus.rs`

```rust
pub struct Bus {
    memory: Memory,
    vic: VIC,
    via1: VIA,
    pub via2: VIA,         // pub for keyboard.rs to read port_b
    watchpoints: Vec<MemoryWriteWatchpoint>,
    frame_buffer: [u8; ACTIVE_HEIGHT * ACTIVE_WIDTH * 4],
}
```

### 4.1 Address Map

| Address Range | Device | Size | Notes |
|---------------|--------|------|-------|
| `0x0000`–`0x03FF` | RAM | 1 KB | Zero page (`0x00`–`0xFF`) + stack (`0x0100`–`0x01FF`) |
| `0x0400`–`0x0FFF` | — | 3 KB | Expansion area, writable via memory |
| `0x1000`–`0x1FFF` | RAM expansion | 4 KB | |
| `0x2000`–`0x7FFF` | — | 24 KB | Expansion area |
| `0x8000`–`0x8FFF` | Character ROM | 4 KB | `CHARACTER_ROM_START..END`, read-only |
| `0x9000`–`0x900F` | VIC registers | 16 B | Routed to `self.vic.read_byte(addr - 0x9000)` |
| `0x9110`–`0x911F` | VIA 1 registers | 16 B | Routed to `self.via1.read_byte(addr - 0x9110)` |
| `0x9120`–`0x912F` | VIA 2 registers | 16 B | Routed to `self.via2.read_byte(addr - 0x9120)` |
| `0x9400`–`0x97FF` | Color RAM | 1 KB | Writable RAM, lower nibble per byte used |
| `0xC000`–`0xDFFF` | BASIC ROM | 8 KB | `BASIC_ROM_START..END`, read-only |
| `0xE000`–`0xFFFF` | KERNAL ROM | 8 KB | `KERNEL_ROM_START..END`, read-only |

### 4.2 Bus `Addressable` Implementation

**`read_byte(addr)`:**
1. If `addr` is in VIC range → `self.vic.read_byte(addr - VIC_REGISTERS_START)`
2. If `addr` is in VIA1 range → `self.via1.read_byte(addr - VIA1_REGISTERS_START)`
3. If `addr` is in VIA2 range → `self.via2.read_byte(addr - VIA2_REGISTERS_START)`
4. Otherwise → `self.memory.read_byte(addr)`

**`write_byte(addr, value)`:**
1. Invoke all active watchpoints for `(addr, value)`
2. Same routing as read, but writes to the appropriate device/memory

### 4.3 Key Bus Methods

```rust
// Step all devices for one cycle: VIC, VIA1 (with NMI), VIA2 (internal only),
// then set cpu.irq_line_low = via2.irq_active()
pub fn step_devices(&mut self, cpu: &mut CPU6502);

// Render the current VIC screen into self.frame_buffer
pub fn render_active_screen(&mut self);

// Returns the RGBA framebuffer (176×184×4 bytes)
pub fn frame_buffer(&self) -> &[u8; ACTIVE_HEIGHT * ACTIVE_WIDTH * 4];

// Returns border RGBA from VIC screen control bits 0-2
pub fn border_rgba(&self) -> [u8; 4];

// Full 64KB memory dump into caller's buffer (for debug mirror)
pub fn copy_memory_to(&self, dest: &mut [u8; 65536]);

// Load ROMs from data/ directory into memory
fn load_standard_roms_from_data_dir(&mut self);

// Watchpoints
pub fn add_watchpoint(&mut self, watchpoint: MemoryWriteWatchpoint);
```

### 4.4 ROM Loading

`load_standard_roms_from_data_dir()` reads three files from `data/` and copies them directly into the memory array (bypassing `write_byte` to avoid write-protection issues):
- `data/basic.901486-01.bin` → `0xC000`
- `data/characters.901460-03.bin` → `0x8000`
- `data/kernal.901486-07.bin` → `0xE000`

## 5. Memory

**File:** `src/memory.rs`

```rust
pub type Memory = [u8; 65536];
```

Implements `Addressable` with write-protection logic in `write_byte`:
- Writable ranges: `0x0000..=0x03FF`, `0x1000..=0x1FFF`, and `0x9600..=0x97FF` (RAM areas)
- All other addresses silently ignore writes (ROM protection)
- No protection on reads — returns whatever is at the address (loaded ROM or zero)

## 6. CPU Subsystem

### 6.1 Registers (`src/cpu/registers.rs`)

```rust
pub struct Registers {
    pub a: u8,      // Accumulator
    pub x: u8,      // X index register
    pub y: u8,      // Y index register
    pub sp: u8,     // Stack pointer (stack at 0x0100-0x01FF, grows downward)
    pub pc: u16,    // Program counter
    pub status: u8, // Processor status flags
}
```

**Status flags (bitmask constants):**

| Bit | Name | Constant | Decimal |
|-----|------|----------|---------|
| 0 | Carry | `CARRY` | `0x01` |
| 1 | Zero | `ZERO` | `0x02` |
| 2 | IRQ disable | `INTERRUPT` | `0x04` |
| 3 | Decimal | `DECIMAL` | `0x08` |
| 4 | Break | `BREAK` | `0x10` |
| 5 | Unused (always 1) | `UNUSED` | `0x20` |
| 6 | Overflow | `OVERFLOW` | `0x40` |
| 7 | Negative | `NEGATIVE` | `0x80` |

**Key methods:**
- `set_flag(flag, on)` / `is_flag_set(flag)` — manipulate individual flags
- `set_accumulator(a)` / `set_x(x)` / `set_y(y)` — set register + auto-update Z/N flags
- `update_carry_flag(value)` / `update_overflow_flag(value)`, etc.
- `update_zero_and_negative(value)` — sets Z if value==0, N from bit 7
- `update_pc(delta)` — signed PC update with wrap-around

### 6.2 Instructions (`src/cpu/instructions.rs`)

```rust
pub enum Instruction {
    ADC, AND, ASL, BCC, BCS, BEQ, BIT, BMI, BNE, BPL, BRK, BVC, BVS,
    CLC, CLD, CLI, CLV, CMP, CPX, CPY, DEC, DEX, DEY, EOR, INC, INX, INY,
    JMP, JSR, LDA, LDX, LDY, LSR, NOP, ORA, PHA, PHP, PLA, PLP, ROL, ROR,
    RTI, RTS, SBC, SEC, SED, SEI, STA, STX, STY, TAX, TAY, TSX, TXA, TXS, TYA,
    Illegal,
}

pub struct InstructionInfo {
    pub opcode: u8,
    pub instruction: Instruction,
    pub mode: AddressingMode,
    pub cycles: u8,
}

// The main opcode decoder — const fn with a massive match over 151 opcodes
pub const fn decode(opcode: u8) -> InstructionInfo;
```

- `Instruction::is_branch()` — returns `true` for BCC, BCS, BEQ, BMI, BNE, BPL, BVC, BVS
- `Instruction::has_page_cross_cycle_penalty(mode)` — returns `true` for ADC/AND/CMP/EOR/LDA/LDX/LDY/ORA/SBC on AbsoluteX/Y or IndirectIndexed when crossing a page boundary

### 6.3 Addressing Modes (`src/cpu/addressing_mode.rs`)

```rust
pub enum AddressingMode {
    ImpliedBreak,       // BRK has a single "signature" byte (opcode + 1)
    Implied,            // CLC, CLI, etc. — no operands
    Accumulator,        // ASL A, ROL A, etc.
    Immediate,          // LDA #$FF
    ZeroPage,           // LDA $FF
    ZeroPageX,          // LDA $FF,X
    ZeroPageY,          // LDX $FF,Y
    Relative,           // BCC + signed offset
    Absolute,           // LDA $1234
    AbsoluteX,          // LDA $1234,X
    AbsoluteY,          // LDA $1234,Y
    Indirect,           // JMP ($1234)
    IndexedIndirect,    // LDA ($FF,X)  — (X-indexed zero-page indirect)
    IndirectIndexed,    // LDA ($FF),Y  — (zero-page indirect, Y-indexed)
}
```

- `operand_count()` — 0 for Implied/Accumulator, 1 for most (including ImpliedBreak), 2 for Absolute variants

**`OperandResolution` trait** (mockable via `unimock`):
- `resolve_value(registers, memory, operands) -> u8` — resolve operand to a byte value (for read-like instructions)
- `resolve_address(registers, memory, operands) -> u16` — resolve operand to a write address (for store-like instructions)
- `crosses_page_boundary(registers, memory, operands) -> bool` — true when indexed addressing crosses a 256-byte page

### 6.4 Instruction Executor (`src/cpu/instruction_executor.rs`)

```rust
pub trait InstructionExecutor {
    fn execute_instruction(
        &self,
        registers: &mut Registers,
        memory: &mut dyn Addressable,
        instruction: Instruction,
        operand_resolution: &dyn OperandResolution,
        operands: &[u8],
        interrupt_handler: &mut dyn InterruptHandler,
    ) -> bool;  // true = auto-increment PC; false = JMP/JSR/RTI/RTS (handle PC themselves)
}

pub struct DefaultInstructionExecutor;  // Zero-sized struct
```

All 58 instructions are implemented in one large `execute_instruction` free function:

**Load/Store:** LDA/LDX/LDY resolve value → set register with Z/N flags. STA/STX/STY resolve address → write register to memory.

**Arithmetic:** ADC/SBC with both binary and NMOS 6502 BCD (decimal mode) semantics, correct overflow flag behavior.

**Logical:** AND/EOR/ORA — bitwise operations on accumulator with Z/N flag updates.

**Shifts/Rotates:** ASL/LSR/ROL/ROR — accumulator or memory operand. Carry flag propagation and Z/N flag updates.

**Branches:** 8 conditional branch instructions — add signed offset to PC if condition matches.

**Jumps:** JMP (absolute/indirect), JSR (push return-1, jump), RTS (pull, add 1), RTI (pull status then PC).

**Stack:** PHA/PHP (push), PLA/PLP (pull). PHP pushes status with B=1 and unused=1 set; PLP clears B and forces unused=1.

**Transfers:** TAX/TAY/TXA/TYA (set N/Z), TSX (set N/Z), TXS (no flag changes).

**Flag operations:** CLC/SEC, CLD/SED, CLI/SEI, CLV.

**Increment/Decrement:** INC/DEC (memory), INX/INY/DEX/DEY (registers). All update Z/N flags.

**Compare:** CMP/CPX/CPY — subtraction without storing result, sets C/Z/N.

**BIT:** Tests A & memory, sets Z (for zero), N (bit 7 of memory), V (bit 6 of memory).

**BRK:** Delegates to `interrupt_handler.handle_interrupt(registers, memory, Interrupt::BRK)`.

**Helper functions:**
- `stack_push(registers, memory, value)` — writes to `0x0100 + sp`, decrements sp
- `stack_push_u16(registers, memory, value)` — pushes high byte then low byte
- `stack_pull(registers, memory) -> u8` — increments sp, reads from `0x0100 + sp`
- `branch_if(registers, operands, condition)` — signed offset branch
- `compare(registers, reg, value)` — subtract for flag updates
- `apply_shift(...)` — generic shift/rotate for accumulator or memory

### 6.5 Interrupt Handler (`src/cpu/interrupt_handler.rs`)

```rust
pub enum Interrupt { NMI, IRQ, BRK }

#[cfg_attr(test, unimock::unimock(api=InterruptHandlerMock))]
pub trait InterruptHandler {
    fn handle_interrupt(&mut self, registers: &mut Registers, memory: &mut dyn Addressable, interrupt: Interrupt);
}

pub struct NoOpInterruptHandler;  // Stub used in tests
```

### 6.6 Instruction Tracking (`src/cpu/instruction_tracking.rs`)

```rust
pub struct InstructionTracking {
    pub current_instruction_info: Option<InstructionInfo>,
    pub interrupt_requested: Option<Interrupt>,
}
```

Implements `InterruptHandler` to manage interrupt timing:
- If an interrupt arrives mid-instruction (`current_instruction_info` is `Some`), defers it to `interrupt_requested`
- If no instruction is executing, processes immediately via `do_interrupt()`
- `do_interrupt()` pushes PC (or PC+2 for BRK) and status, loads vector from `0xFFFA` (NMI) or `0xFFFE` (IRQ/BRK), sets I flag, clears B for non-BRK

### 6.7 CPU6502 — Core Execution Engine (`src/cpu/cpu6502.rs`)

```rust
pub struct CPU6502 {
    pub registers: Registers,
    cycle_count_at_end_of_this_instruction: u8,
    cycle_count: u8,
    operands_index: usize,
    operands_buffer: [u8; 2],
    total_cycles: u64,
    last_performance_log_cycle: u64,
    last_performance_log_instant: Instant,
    breakpoints: Vec<Box<dyn Breakpoint>>,
    pub instruction_tracking: InstructionTracking,
    pub irq_line_low: bool,  // When true, IRQ is pending (checked each step)
}
```

**`step(&mut self, memory: &mut impl Addressable, instruction_executor: &impl InstructionExecutor)`**

Each call is exactly one CPU clock cycle. Multi-cycle instructions use an explicit state machine:

**Prologue (when no instruction is mid-execution):**
1. If `interrupt_requested` is pending from the previous cycle, process it via `do_interrupt()` and return
2. If `irq_line_low` is true AND I flag is clear, trigger IRQ interrupt and return

**Cycle progression (every call):**
1. `total_cycles += 1`
2. `cycle_count += 1`
3. Performance logging every 1M cycles (logs at debug level)

**Opcode fetch (when `current_instruction_info` is `None`):**
1. Read opcode from `memory[pc]`
2. `decode(opcode)` → `InstructionInfo`
3. Store in `current_instruction_info`
4. Reset `operands_index = 0`
5. Set `cycle_count_at_end_of_this_instruction = cycle_count + info.cycles - 1`

**Operand read (when instruction decoded but operands remain):**
1. Read next operand byte from `memory[pc + 1 + operands_index]`
2. Store in `operands_buffer[operands_index]`
3. After final operand: check page-cross penalty (may extend `cycle_count_at_end_of_this_instruction` by 1)

**Execution (when `cycle_count == cycle_count_at_end_of_this_instruction`):**
1. Check breakpoints
2. Create debug log line (info-level)
3. Call `instruction_executor.execute_instruction()`
4. If `auto_increment_pc`: `pc += 1 + operand_count`
5. Clear `current_instruction_info`, reset `cycle_count = 0`
6. Post-execution: re-check deferred interrupts and IRQ line

**Other methods:**
- `reset(reset_vector: u16)` — initializes SP=0xFD, PC=reset_vector, clears D and I flags
- `add_breakpoint_address(address)` — adds a logging breakpoint for a specific address
- `total_cycles() -> u64` — total cycles since reset

## 7. VIC-I Chip

**File:** `src/vic.rs`

```rust
pub struct VIC {
    registers: [u8; 15],   // 0x9000–0x900E
    screen_control: u8,    // 0x900F (separate field for convenience)
    cycle_count: u64,
}
```

**Default state:** `registers[0x03] = 0x1E`, `registers[0x05] = 0x80`, `screen_control = 0x0E`.

**`Addressable` impl:** Routes offsets 0x00–0x0E to the `registers` array, offset 0x0F to `screen_control`.

**Rendering pipeline (`render_active_screen`):**
1. Compute screen RAM start address:
   - Bit from register 0x02: `4 * (reg[2] & 0x80)`
   - Bits from register 0x05: `64 * (reg[5] & 0x70)`
2. Compute color RAM start: `0x9400 + 4 * (reg[2] & 0x80)`
3. For each of 176×184 pixels (22×23 characters at 8×8):
   - Read character code from screen RAM
   - Read foreground color from color RAM (lower nibble)
   - Look up character bitmap from Character ROM (8 bytes per character at `0x8000`)
   - Extract pixel bit: 1 = foreground, 0 = background
   - Look up palette colors and write RGBA to framebuffer

**Key methods:**
- `step()` — increments `cycle_count`
- `border_rgba()` — palette color from screen_control bits 0–2
- `background_colour()` — bits 4–7 of screen_control
- `screen_ram_start()` — computed as above
- `colour_ram_start()` — computed as above

**Constants** (from `ui/screen/renderer.rs`): `TEXT_COLUMNS=22`, `TEXT_ROWS=23`, `CHAR_WIDTH=8`, `CHAR_HEIGHT=8`, `ACTIVE_WIDTH=176`, `ACTIVE_HEIGHT=184`.

**Not yet implemented:** reverse mode (bit 3 + char bit 7), multi-color mode, double-height characters (bit 0 of 0x9003), sound registers (0x900A–0x900D), light pen, raster line register, paddle registers.

## 8. VIA 6522 Chip

**File:** `src/via.rs`

```rust
pub struct VIA {
    pb: u8, pa: u8,                    // Port B / Port A data
    ddrb: u8, ddra: u8,                // Data Direction Registers
    timer2_counter_lo: u8, timer2_counter_hi: u8,
    shift_register: u8,
    auxiliary_control: u8,             // ACR
    peripheral_control: u8,            // PCR
    ifr: Cell<u8>,                     // Interrupt Flag Register (Cell for interior mutability)
    ier: u8,                           // Interrupt Enable Register
    t1_counter: Cell<u16>,             // Timer 1 counter
    t1_latch: Cell<u16>,               // Timer 1 latch
    ca1_pending: bool,
}
```

Uses `std::cell::Cell` for `ifr`, `t1_counter`, `t1_latch` to enable mutation through `&self` references during `read_byte()` — needed because reading the timer low byte must clear the IFR timer flag.

**Register map (offset from base):**

| Offset | Register | Notes |
|--------|----------|-------|
| 0x00 | PB (Port B) | Keyboard column drive |
| 0x01 | PA (Port A) | Keyboard row read |
| 0x02 | DDRB | Data direction for port B |
| 0x03 | DDRA | Data direction for port A |
| 0x04–0x05 | Timer 1 counter lo/hi | Reading lo clears IFR_TIMER1 |
| 0x06–0x07 | Timer 1 latch lo/hi | Writing hi reloads counter from latch |
| 0x08–0x09 | Timer 2 counter lo/hi | Storage only (no countdown yet) |
| 0x0A | Shift Register | |
| 0x0B | ACR (Auxiliary Control) | |
| 0x0C | PCR (Peripheral Control) | CA1 edge polarity |
| 0x0D | IFR (Interrupt Flag Register) | Bit 7 = IRQ active, bits 0–6 = source flags |
| 0x0E | IER (Interrupt Enable Register) | Bit 7 = set/clear control |
| 0x0F | PA (no handshake) | Same as 0x01 |

**Public methods:**
```rust
// Step with interrupt handling (NMI for VIA1, IRQ line check for VIA2)
pub fn step(&mut self, registers: &mut Registers, memory: &mut dyn Addressable,
            interrupt_handler: &mut dyn InterruptHandler, interrupt: Interrupt);

// Step internal state only (timer1 + CA1 edge detect + IFR update)
pub fn step_internal(&mut self);

// True when IFR bit 7 is set (any enabled interrupt is active)
pub fn irq_active(&self) -> bool;

// Keyboard support
pub fn port_b(&self) -> u8;       // Read column drive value
pub fn set_port_a(&mut self, value: u8);  // Write row result
```

**Timer 1 logic** (`step_timer1`): Each call decrements `t1_counter`. On underflow (reaching 0): reloads from `t1_latch`, sets `IFR_TIMER1` bit (0x40).

**IFR/IRQ update** (`update_ifr_irq`): Computes `(ifr & 0x7F) & (ier & 0x7F)`. If any bits are set: sets `IFR_IRQ` bit (0x80), otherwise clears it.

**CA1 edge detect** (`check_ca1_edge`): If `ca1_pending` is true, checks PCR bit 0 for edge polarity and sets `IFR_CA1`.

**IFR/IER write semantics:**
- Writing IFR: bit 7=1 to set bits in bits 0–6, bit 7=0 to clear
- Writing IER: bit 7=1 to enable bits in bits 0–6, bit 7=0 to disable

**Not yet implemented:** Timer 2 countdown, shift register behavior, full ACR/PCR (CB1/CB2/CA2 interrupts), port handshaking.

## 9. Virtual Clock

**File:** `src/virtual_clock.rs`

```rust
pub trait Clock {
    fn now(&self) -> Instant;
}

pub struct SystemClock;    // Wraps Instant::now()
pub struct MockClock { pub now: Instant }  // Controllable for tests
```

The `Clock` trait enables testing time-dependent logic (keyboard click timing, flash duration) without real time. Used by `KeyboardState` in `ui/keyboard/mod.rs`.

## 10. Keyboard Subsystem

### 10.1 Physical Key Mapping (`src/keyboard.rs`)

```rust
pub struct Keyboard {
    cache: HashSet<Key>,                          // Currently pressed keys
    receiver: Receiver<HashSet<Key>>,              // Receives from UI thread
    keyboard_map: HashMap<(Key, u8), u8>,          // (key, port_b_column) -> port_a_row_value
}
```

**Channel:** `make_keyboard_channel() -> (SyncSender<HashSet<Key>>, Receiver<HashSet<Key>>)` creates an `mpsc::sync_channel(2)` (bounded to 2 pending messages).

**`step(port_b: u8) -> Option<u8>`:**
1. Non-blocking receive: `self.receiver.try_recv()` — if a new keyset is available, update `self.cache`
2. If cache is empty, return `None`
3. For each key in cache: look up `(key, port_b)` in `keyboard_map`, get port_a row value for that column
4. Fold all results with AND (VIC-20 keyboard is active-low: 0 = key pressed). If result == `0xFF` (all rows high), return `None` (no key pressed in this column). Otherwise return `Some(result)`.

**Matrix dimensions:** 8 columns (port_b bits 7–0) × 8 rows (port_a bits 7–0). Also includes a secondary mapping with `port_b=0x00` (all columns driven) for matching keys regardless of column selection.

### 10.2 UI Key Types (`src/ui/keyboard/key.rs`)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Single(char),   // Alphanumeric: 'A', '1', etc.
    // Special keys:
    Left, Up, ClrHome, InsDel, Ctrl, Restore, RunStop, ShiftLock,
    Return, Cbm, Shift, CrsrUD, CrsrLR,
    F1F2, F3F4, F5F6, F7F8,
}
```

### 10.3 Keyboard State (`src/ui/keyboard/mod.rs`)

```rust
pub struct KeyboardState<C = SystemClock> {
    pub clock: C,
    pub key_regions: Vec<KeyRegion>,       // Hit-test regions from layout PNG
    pub last_click: Option<(Key, Instant)>,
    pub held_key: Option<Key>,             // Double-click toggles hold
    pub flash_key: Option<(Key, Instant)>, // Visual feedback flash (200ms)
    pub physical_keys: HashSet<Key>,        // Sent to emulator thread
    pub status_message: String,
}
```

**Key interaction model:**
- **Single click:** flashes key (200ms), adds to `physical_keys`, removes after flash expires
- **Double click (within 350ms):** toggles hold — key stays in `physical_keys` until next double-click
- **Chord (click while holding):** previous hold released, both keys flash briefly
- `classify_click(key)` uses `clock.now()` to differentiate single vs double
- `tick_flash()` expires flashed keys after `FLASH_DURATION`
- `flash_remaining()` returns remaining flash time for animation deadline calculation

**`build_key_regions()`** — hardcoded pixel coordinates matching `data/vic20-c64-layout.png` (1006×290 image). Each key has a `KeyRegion { label: Key, x, y, w, h }`.

## 11. UI — Screen Window

### 11.1 Renderer Constants & Palette (`src/ui/screen/renderer.rs`)

**Constants:** `TEXT_COLUMNS=22`, `TEXT_ROWS=23`, `CHAR_WIDTH=8`, `CHAR_HEIGHT=8`, `ACTIVE_WIDTH=176`, `ACTIVE_HEIGHT=184`, `BORDER_*` = 16 pixels on each side, `PAL_WIDTH=208`, `PAL_HEIGHT=216`.

**`palette(index: u8) -> [u8; 4]`** — 16 VIC-20 colors as RGBA:
0: black, 1: white, 2: red, 3: cyan, 4: purple, 5: green, 6: blue, 7: yellow,
8: orange, 9: light orange, 10: pink, 11: light cyan, 12: light purple, 13: light green, 14: light blue, 15: light yellow.

**`display_vic20_screen(frame, border_rgba, screen_rgba)`** — fills a 208×216 RGBA buffer with border color, then blits the 176×184 active screen at the center.

### 11.2 Shared Video State (`src/ui/screen/display.rs`)

```rust
pub struct SharedVideoState {
    pub screen_rgba: Vec<u8>,   // ACTIVE_WIDTH * ACTIVE_HEIGHT * 4 bytes
    pub border_rgba: [u8; 4],
}
```

`ScreenWindow` owns a winit window titled "VIC-20" at 3× scale (624×648 logical pixels) and a `Pixels` surface. `draw()` calls `display_vic20_screen()` then `pixels.render()`.

## 12. UI — Keyboard Window

**File:** `src/ui/keyboard/display.rs`

```rust
pub struct KeyboardWindow {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    image_rgba: Vec<u8>,           // Keyboard PNG as RGBA
    image_width: u32,              // 1006
    image_height: u32,             // 290
    cursor_pos: Option<(f64, f64)>,
}
```

**Features:**
- Renders keyboard layout PNG (embedded via `include_bytes!`)
- Mouse interaction: `CursorMoved` tracking, pixel-to-key lookup via `key_at_pixel`, `on_key_click` → `KeyboardState`
- Physical keyboard mapping: `keycode_to_vickeys` translates modern `KeyCode` to VIC-20 `Key` enums (e.g., ArrowUp → `[CrsrUD, Shift]`, F1 → `[F1F2]`)
- Visual feedback: held keys rendered with blue tint, pressed keys with light blue, flash keys with cyan
- Status bar with held key name and status message (8×8 font)
- `next_deadline()` returns next animation deadline
- `request_redraw()` schedules a winit redraw

## 13. Debug Subsystem

### 13.1 Shared State Types (`src/debug/mod.rs`)

```rust
pub type SharedMemory = Arc<Mutex<[u8; 65536]>>;
pub type PendingWrites = Arc<Mutex<Vec<(u16, u8)>>>;
pub type SharedRegistersState = Arc<Mutex<SharedRegisters>>;
pub type PendingRegisterWrites = Arc<Mutex<Vec<(RegisterField, u16)>>>;
pub type SharedPerfState = Arc<Mutex<SharedPerformanceMetrics>>;

pub struct SharedRegisters {
    pub a: u8, pub x: u8, pub y: u8, pub sp: u8, pub pc: u16, pub status: u8,
}

pub struct SharedPerformanceMetrics {
    pub cycles_per_second: f64, pub frames_per_second: f64,
    pub total_cycles: u64, pub total_frames: u64,
}

pub enum RegisterField { A, X, Y, SP, PC, Status }
```

**`DebugState`** tracks user interaction:
- `start_address: u16` — memory view base address (default `0x1000`)
- `selected_offset: Option<usize>` — cursor in memory hex grid (0–255)
- `address_input: String` / `edit_byte_input: String` — text entry buffers
- `mode: DebugMode` — `Browse | EditingAddress | EditingByte | EditingRegister(RegisterField)`
- Navigation: `navigate_address(delta)` wraps with `wrapping_add_signed`
- `commit_address()` / `commit_byte_edit()` / `commit_register_edit()` — commit edits, return pending write operations

### 13.2 Debug Display (`src/debug/display.rs`)

```rust
pub struct DebugWindow {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    cursor_pos: Option<(f64, f64)>,
}
```

**Features:**
- 16×16 memory hex grid with address column, hex bytes, and ASCII representation
- Address bar with input (G key to edit, hex digits auto-start entry)
- Byte editing: E key on selected byte, Enter to commit → `PendingWrites`
- Register display: A, X, Y, SP, PC, SR with hex values and flag visualization (N/V/-/B/D/I/Z/C with gold=set, gray=clear). Click to edit registers.
- Performance metrics: CPU freq (Hz/KHz/MHz), FPS, total cycles/frames
- Keyboard navigation: arrows for cursor, PageUp/PageDown for scroll, G/E for commands

## 14. Tools — Breakpoints, Watchpoints, Disassembler

### 14.1 Breakpoints and Watchpoints (`src/tools/debug.rs`)

```rust
pub trait Breakpoint {
    fn on_hit(&self, address: u16);
}

pub struct LoggingAddressBreakpoint { address: u16 }
// on_hit: logs "Breakpoint: 0x{address}" at info level when PC matches

pub struct MemoryWriteWatchpoint {
    address: Box<dyn Fn(u16) -> bool>,  // Closure predicate
}
// watch_address(addr) — single address
// watch_address_range(start, end) — range predicate
// on_write(address, value) — logs "Watchpoint: Write of 0x{value:02X} to 0x{address:04X}"
```

### 14.2 Disassembler (`src/tools/disassembler.rs`)

```rust
pub struct DefaultDisassembler { separator: String }

// Parses one instruction from a byte slice, returns (len, InstructionInfo)
pub fn parse_instruction(&self, data: &[u8]) -> (usize, InstructionInfo);

// Formats an instruction as a human-readable string
// e.g., "LDA #$FF", "JMP ($1234)", "BCC $C012"
pub fn disassemble_instruction(&self, instruction_info: &InstructionInfo,
                                operands: &[u8], instruction_address: u16) -> String;
```

**`disassemble_bytes(data, disassembler, base_address, start_address) -> Vec<String>`** iterates from `start_address - base_address`, formats each instruction as `"0x{address}:\t{disassembly}"`.

Supports all addressing modes: implied (just instruction name), immediate (`#$XX`), zero page (`$XX`, `$XX,X`, `$XX,Y`), relative (target address), absolute (`$XXXX`, `$XXXX,X`, `$XXXX,Y`), indirect (`($XXXX)`), indexed indirect (`($XX,X)`), indirect indexed (`($XX),Y`).

## 15. Controller — Threading & Event Loop

**File:** `src/controller.rs`

```rust
pub struct Vic20Controller {
    screen: ScreenWindow,
    keyboard: KeyboardWindow,
    debug: DebugWindow,
    shared_state: Option<SharedState>,
    keyboard_state: KeyboardState,
    debug_state: DebugState,
    tick_duration: Duration,
    vic_thread: Option<JoinHandle<()>>,
}
```

Implements `winit::application::ApplicationHandler`. This is the central orchestration point connecting the UI (3 windows) to the emulation engine.

### 15.1 Threading Model

**Main thread (UI):** Runs the winit event loop. Handles `resumed`, `window_event`, and `about_to_wait`.

**Worker thread (`"vic20-core-loop"`):** Spawned during `resumed`. Runs a tight emulation loop:

```
loop {
    // 1. Keyboard input: convert physical keys to VIA port A
    let port_a = keyboard.step(via2.port_b());
    if let Some(pa) = port_a { via2.set_port_a(pa); }

    // 2. Debugger writes (try_lock — non-blocking)
    if let Some(writes) = pending_writes.try_lock() { ... }
    if let Some(writes) = pending_register_writes.try_lock() { ... }

    // 3. Step devices for one cycle
    bus.step_devices(&mut cpu);
        // - VIC.step() — increment cycle count
        // - VIA1.step(...) — with NMI interrupt handler
        // - VIA2.step_internal() — timer1 + CA1, no interrupts here
        // - cpu.irq_line_low = via2.irq_active()

    // 4. Step CPU for one cycle
    cpu.step(&mut bus, &executor);

    // 5. Publish state on intervals:
    //    a) Every 20ms: render screen → SharedVideoState, copy registers → SharedRegisters
    //    b) Every 200ms: copy full 64KB memory → SharedMemory
    //    c) Every 1s: compute perf metrics → SharedPerformanceMetrics

    // 6. Sleep tick_duration (default 0µs, configurable via CLI)
}
```

### 15.2 Shared State

```rust
struct SharedState {
    video: Arc<Mutex<SharedVideoState>>,
    memory: SharedMemory,                          // Arc<Mutex<[u8; 65536]>>
    pending_writes: PendingWrites,                 // Arc<Mutex<Vec<(u16, u8)>>>
    registers: SharedRegistersState,               // Arc<Mutex<SharedRegisters>>
    pending_register_writes: PendingRegisterWrites, // Arc<Mutex<Vec<(RegisterField, u16)>>>
    perf: SharedPerfState,                         // Arc<Mutex<SharedPerformanceMetrics>>
    keyboard_sender: SyncSender<HashSet<Key>>,
}
```

### 15.3 Locking Strategy

| Thread | Data | Lock Type | Reason |
|--------|------|-----------|--------|
| UI (main) | video, memory, registers, perf | `lock()` (blocking) | Reading for display; brief holds |
| Emulator | pending_writes, pending_register_writes | `try_lock()` (non-blocking) | Must not block the core loop |
| Emulator | video, memory, registers, perf | `lock()` (blocking) | Publishing state; brief holds |

The `try_lock()` pattern is critical: when the debug UI is holding the pending-writes lock to push edits, the emulator thread skips debug writes for that cycle rather than blocking.

### 15.4 Keyboard Channel

`mpsc::sync_channel(2)` — bounded to 2 pending messages:
- **UI thread:** Sends `HashSet<Key>` on every `KeyboardInput` event
- **Emulator thread:** Uses `try_recv()` to update keyboard cache non-blockingly. If the channel is empty, uses the cached last keyset.

### 15.5 Frame Timing

In `about_to_wait`:
- Computes nearest deadline between 50Hz screen refresh (20ms) and keyboard flash animation expiry
- Uses `ControlFlow::WaitUntil(nearest)`
- Requests redraws on all 3 windows

**Constants:**
- `FRAME_TIME`: 20ms (50Hz)
- `FRAME_PUBLISH_INTERVAL`: 20ms (screen + registers)
- `MEMORY_PUBLISH_INTERVAL`: 200ms
- `PERF_PUBLISH_INTERVAL`: 1s

### 15.6 `resumed` Event

Called once when the winit event loop starts:
1. Creates 3 windows (screen, keyboard, debug)
2. Creates `SharedState` with all `Arc<Mutex<>>` handles
3. Spawns `"vic20-core-loop"` thread running `run_emulator()`

### 15.7 `window_event` Handling

**Screen window:**
- `RedrawRequested` → lock video state → `screen.draw()`
- `KeyboardInput` → forward to keyboard handler, send updated `physical_keys` to emulator

**Keyboard window:**
- `RedrawRequested` → `keyboard.draw()`
- Mouse/key events → keyboard input handler

**Debug window:**
- `RedrawRequested` → lock memory, registers, perf → `debug.draw()`
- Mouse/key events → debug input handler

### 15.8 Binaries

**`src/bin/vic20.rs`**: Initializes `env_logger`, parses optional tick duration (microseconds) from CLI args, creates `Vic20Controller`, calls `run()` to start the winit event loop.

**`src/bin/disassembler.rs`**: Resets SIGPIPE handler for pipe support. Reads file from CLI arg, creates `DefaultDisassembler` with tab separator, calls `disassemble_bytes()`, prints results.

## 16. Data Flow Summary

```
CLI (vic20.rs)
  └─> Vic20Controller::new(tick_duration)
        └─> Vic20Controller::run()
              └─> winit EventLoop::run_app()
                    │
                    ├─ resumed:
                    │    └─> spawn_emulator() → "vic20-core-loop" thread
                    │          load_standard_roms() → CPU.reset(0xFFFC)
                    │          loop {
                    │            keyboard.step(via2.port_b()) → via2.set_port_a()
                    │            apply pending debug writes (try_lock)
                    │            bus.step_devices(&mut cpu)
                    │              ├─ VIC.step()
                    │              ├─ VIA1.step(..., NMI)
                    │              ├─ VIA2.step_internal() (timer1+CA1)
                    │              └─ cpu.irq_line_low = via2.irq_active()
                    │            cpu.step(&mut bus, &executor) // 1 CPU cycle
                    │            publish frame/registers/memory/perf → SharedState
                    │            sleep(tick_duration)
                    │          }
                    │
                    └─ window_event:
                          Screen: Redraw → lock video → display_vic20_screen()
                          Keyboard: click/key → KeyboardState → send physical_keys
                          Debug: click/key → DebugState → push pending writes
```

**Key data paths:**

1. **Keyboard → CPU:** winit event → `KeyboardState.physical_keys` → `SyncSender` → emulator `try_recv` → `keyboard.step(port_b)` → `via2.set_port_a()` → VIA2 register → CPU reads via bus

2. **CPU → Screen:** CPU writes to screen RAM → `bus.render_active_screen()` → VIC looks up chars in ROM, colors in color RAM → framebuffer → `Arc<Mutex<SharedVideoState>>` → UI thread → `display_vic20_screen()` → `pixels.render()`

3. **Debug writes:** Debug UI → `PendingWrites.lock().push((addr, val))` → emulator `try_lock()` → `bus.write_byte(addr, val)` → device routing

4. **Memory mirror:** Emulator every 200ms → `bus.copy_memory_to()` → `SharedMemory.lock()` → Debug UI `draw()` → hex grid

## 17. Design Patterns & Pitfalls

### Key Design Patterns

1. **Trait-based device interface:** `Addressable` unifies all readable/writable entities. The Bus is both an `Addressable` (for the CPU) and an address router (for devices).

2. **Trait-based execution isolation:** `InstructionExecutor`, `OperandResolution`, `InterruptHandler` are traits enabling `unimock` testing without real CPU/memory.

3. **Cycle-accurate stepping:** `cpu.step()` is exactly one clock cycle. Multi-cycle instructions use an explicit state machine (`cycle_count`, `operands_index`, `current_instruction_info`).

4. **Interior mutability in VIA:** `Cell<u8>` / `Cell<u16>` for `ifr`, `t1_counter`, `t1_latch` enables mutation during `read_byte(&self)` — necessary because reading the timer low byte must clear the IFR flag.

5. **Arc\<Mutex\<\>\> for cross-thread access:** All UI↔emulator shared state uses `Arc<Mutex<>>`. The emulator uses `try_lock()` for reads (non-blocking); the UI uses `lock()` for reads (can afford to wait).

6. **Channel-based input:** Keyboard events flow via `sync_channel(2)`, decoupling input polling from rendering.

7. **Clock abstraction:** `Clock` trait enables time-dependent testing of keyboard interactions.

### Known Pitfalls

- **Self-referential lifetimes:** Avoid designs where CPU execution helpers borrow `&self` and `&mut self` simultaneously. Construct short-lived executors per step when needed.

- **Aliasing mutable borrows:** In bus/device stepping, avoid calling methods that borrow `&mut self` while another field borrow already exists. The compiler will reject these; refactor to use independent local borrows.

- **`unimock` Debug requirements:** Trait objects mocked with `unimock` need a local `Debug` impl (see `src/addressable.rs:33-38` for the pattern).

- **`try_lock()` on bounded channels:** If the channel is full, `try_lock()` will fail (no blocking). The emulator skips the operation. Make sure the UI never holds these locks for long.

- **ROM file dependencies:** Tests in `src/bus.rs` call `load_standard_roms_from_data_dir()` — they panic if ROM files are missing from `data/`.

- **Nightly rustfmt:** Formatting requires `cargo +nightly fmt` due to `unstable_features=true` in `rustfmt.toml`.

- **Edition 2024:** Requires Rust >= 1.85.0. If `cargo build` fails after an upgrade, `rm -rf target` first (the edition change can confuse incremental builds).
