# rusty-nmos6502

Cycle-perfect NMOS 6502 CPU emulator — all 151 documented opcodes plus illegal opcodes, implemented with declarative micro-op execution sequences.

[![crates.io](https://img.shields.io/crates/v/rusty-nmos6502.svg)](https://crates.io/crates/rusty-nmos6502)
[![docs.rs](https://img.shields.io/docsrs/rusty-nmos6502)](https://docs.rs/rusty-nmos6502)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
rusty-nmos6502 = "<latest version>"
```

Then:

```rust
use nmos6502::{CPU6502, memory::Ram, Addressable};

let mut cpu = CPU6502::default();
let mut mem = Ram::new();

// Set the program counter, initialise memory, then step cycle by cycle.
cpu.registers.pc = 0x0600;
cpu.cycle(&mut mem);          // one micro-op per call
```

The CPU interacts with memory exclusively through the [`Addressable`] trait, for plugging in bus or memory implementations.

### Execution Model

- `cpu.cycle(&mut bus)` executes **one micro-op**, not one instruction. Instructions take 2–8 cycles.
- `cpu.is_at_instruction_boundary()` returns `true` at a stable PC value.
- `cpu.reset(&mut bus)` clears all registers and loads the reset vector.

### Bus Interface

```rust
pub trait Addressable {
    fn read_byte(&mut self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);
}
```

The `&mut self` on reads is intentional — some hardware reads have side effects.

## Test Suites

The emulator passes:

- **Klaus Dormann functional, decimal and interrupt tests** — assembled from bundled sources using the built-in assembler. These run from the crates.io tarball with no external data.
- **Tom Harte 6502 JSON test suite** — 256 opcode files, each with 10,000 randomised test cases (2.56 million total). Full bus-cycle traces are validated per cycle. The test data (1 GB) is not bundled with the published crate; clone the repo and download Tom Harte's [ProcessorTests](https://github.com/TomHarte/ProcessorTests) to `external/6502/` to run these locally.

### Benchmarks

On an Apple Mac M1 the CPU runs at the equivalent of ~300 MHz.

```bash
cargo +nightly bench -p rusty-nmos6502
```

## Optional Binary: Disassembler

The crate ships a standalone disassembler binary:

```bash
cargo run -p rusty-nmos6502 --bin disassembler -- kernal.bin E000 FF72 | head -20
```

## Crate Features

- Cycle-accurate micro-op execution (no "run to completion" — every bus cycle is observable)
- All 151 documented opcodes plus undocumented (illegal) opcodes
- NMI (falling-edge, latching), IRQ (level-sensitive), BRK, and Reset
- Binary and BCD arithmetic (ADC, SBC) with correct NMOS 6502 flag quirks
- Built-in two-pass AS65-syntax assembler (`tools::assembler`)
- Built-in disassembler (`tools::disassembler`)
- Full programmatic breakpoint support (`Breakpoint` trait)
- Tested against 2.56M Harte test cases + Dormann functional/decimal/interrupt suites

## License

GPLv3
