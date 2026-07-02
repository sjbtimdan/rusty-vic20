# AGENTS.md — `rusty-nmos6502` crate

Standalone NMOS 6502 CPU crate (`nmos6502/`). Published on crates.io as `rusty-nmos6502`. Not a workspace member — the root crate references it via `path = "nmos6502"` in `Cargo.toml`. Use `-p rusty-nmos6502` to target it from the root.

## Build / Test / Lint / Bench

```bash
cargo build -p rusty-nmos6502
cargo test -p rusty-nmos6502                           # all tests (no ROMs needed)
cargo test -p rusty-nmos6502 --test harte_tests a9     # single Harte opcode (hex substring)
cargo clippy -p rusty-nmos6502                         # fix all warnings
cargo +nightly bench -p rusty-nmos6502                 # nightly-only (uses #![feature(test)])
cargo +nightly fmt                                     # root-level rustfmt.toml with imports_granularity="Crate"
```

Formatting and import style come from root `rustfmt.toml`. No crate-local config.

## Execution Model

`cpu.cycle(&mut bus)` executes **one micro-op**, not one instruction. The CPU drives the address bus every cycle — there are no idle bus cycles. The number of cycles per instruction varies (2–8 depending on opcode, addressing mode, page-cross, and branch-taken penalties).

```rust
let mut cpu = CPU6502::default();
cpu.cycle(&mut memory); // advance one micro-op
```

`cpu.is_at_instruction_boundary()` returns `true` when the current sequence has finished — useful for detecting stable PC values and stall conditions. `cpu.reset(&mut bus)` sets default register state and loads reset vector (`$FFFC`–`$FFFD`).

## Bus Interface

`nmos6502::Addressable` trait — the only coupling between CPU and memory:

```rust
pub trait Addressable {
    fn read_byte(&mut self, address: u16) -> u8;   // &mut self — reads can have side effects
    fn write_byte(&mut self, address: u16, value: u8);
}
```

Testing `Ram` provided at `nmos6502::memory::Ram` (64KB, zero-initialized). CPU is generic over `impl Addressable` — plug in any bus.

## Architecture

| File | Purpose |
|------|---------|
| `src/cpu.rs` | `CPU6502` struct, `cycle()`, internal op implementations |
| `src/sequences.rs` | All instruction micro-op sequences — **heart of the emulator** |
| `src/micro_op.rs` | `MicroOp`, `BusOp`, `InternalOp` types |
| `src/registers.rs` | `Registers` struct, flag bit constants |
| `src/memory.rs` | `Addressable` trait, `Ram` |
| `src/alu.rs` | `adc()`, `sbc()`, `compare()` (binary + BCD) |
| `src/edge_latch.rs` | NMI line edge detector (falling-edge, latching) |
| `src/opcode.rs` | Opcode metadata for disassembly (no execution logic) |
| `src/breakpoint.rs` | `Breakpoint` trait, `LoggingBreakpoint` |
| `src/tools/assembler.rs` | Two-pass AS65-syntax assembler (2.3k lines) |
| `src/tools/disassembler.rs` | `Disassembler` trait, `DefaultDisassembler` |

sequences.rs uses `const fn` helpers (`seq_imm`, `seq_zp`, `seq_abs`, `rmw_zp`, `branch_seq`, etc.) for static `&[MicroOp]` arrays. The only macro is `interrupt_seq!`. Shorthand: `m(bus, internal)`, `i(internal)`, `b(bus)`, constants `N`, `BN`, `NONE`.

## Interrupts

- **NMI**: `cpu.nmi_latch: EdgeLatch` — falling-edge triggered. Set level before each cycle, fetched via `take()` during opcode fetch.
- **IRQ**: `cpu.irq_line_low: bool` — level-sensitive. Sampled during opcode fetch when I flag is clear.
- **BRK**: Internal software interrupt (handler same as IRQ vector).
- **Reset**: `cpu.reset(&mut bus)` — loads PC from `$FFFC`–`$FFFD`, clears all state.

## Test Suite (all self-contained, no ROM files)

| Test | File | What it covers |
|------|------|----------------|
| Harte opcodes | `tests/harte_tests.rs` | 2.56M parameterized cases from Tom Harte JSON — 256 files (`external/6502/v1/*.json`), 10k cases each. Per-opcode isolation via `rstest`. Also validates bus cycle traces. |
| Dormann functional | `tests/dormann_functional_test.rs` | Full Klaus Dormann functional test — assembled from bundled `external/Klaus2m5/6502_functional_test.a65` using built-in assembler. Runs until stall at `jmp *`. |
| Dormann decimal | `tests/dormann_decimal_test.rs` | Decimal mode ADC/SBC. Bundled `.a65` source, custom capture for per-failure state. |
| Dormann interrupt | `tests/dormann_interrupt_test.rs` | IRQ/NMI/BRK with a custom `InterruptBus` that maps feedback port writes to interrupt lines. |
| Unit tests | `src/registers.rs`, `src/alu.rs`, `src/edge_latch.rs`, `src/cpu.rs` | Inline `#[cfg(test)]` with `rstest` fixtures. |

Shared helpers in `tests/common/mod.rs`: `assemble_program()`, `load_program()`, `run_until()`. Harte tests validate bus cycles per cycle (address, value, read/write direction) against the expected trace.

## Key Gotchas

- **`-p rusty-nmos6502` everywhere** — not a workspace member, so `cargo test` (without `-p`) tests the root crate only.
- **`cpu.cycle()` ≠ one instruction** — it's one micro-op. Instructions take 2–8 calls depending on opcode/addressing/page-cross/branch.
- **No ROMs or external files needed** for any test — all data is bundled in `external/` or generated inline.
- **Two `Addressable` traits in the repo** — this crate's `nmos6502::Addressable` uses `&mut self` on reads. The root crate has its own `crate::hardware::addressable::Addressable` with `&self`. Don't confuse them.
- **`cpu.reset()` clears everything** — sets all registers to defaults, including `pc = 0`. Must follow with vector load sequence.
- **`cpu.halted`** — set to `true` by KIL/HLT opcodes (`$02`, `$12`, `$22`, etc.). CPU stops fetching; `cycle()` becomes a no-op.
- **Bench requires nightly** — `#![feature(test)]` in `benches/cpu_bench.rs`.
- **Interrupt test uses custom `Addressable`** — `InterruptBus` maps `$BFFC` writes to `cpu.irq_line_low` and `cpu.nmi_latch.set_level()`.
