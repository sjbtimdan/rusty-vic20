# nmos6502

NMOS 6502 CPU emulator — all 151 documented opcodes plus illegal opcodes, with declarative micro-op execution sequences. Cycle level accuracy is targetted.

## Usage

```rust
use nmos6502::{CPU6502, memory::Ram, Addressable};

let mut cpu = CPU6502::default();
let mut mem = Ram::new();

// Initialize memory, program counter and registers, then step cycle by cycle.
cpu.cycle(&mut mem);
```

The CPU interacts with memory exclusively through the `Addressable` trait, making it easy to plug into any bus or memory implementation.

## Test Suites

The code passes all the [Tom Harte 6502 JSON test suite](https://github.com/TomHarte/ProcessorTests) — 256 opcode files (`00.json` through `ff.json`), each with 10,000 randomized test cases (2.56 million total). 

The Klauss Dormann functional, decimal and interrupt tests also pass.

### Benchmarks

On an Apple Mac M1, the benchmark tests clock it at the equivalent of around 300Mhz.

```bash
cargo +nightly bench
```

## References

- [AGENTS.md](AGENTS.md) — agent guide for working in this crate
