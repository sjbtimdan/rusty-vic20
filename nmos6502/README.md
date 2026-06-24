# nmos6502

Cycle-perfect NMOS 6502 CPU emulator — all 151 documented opcodes plus illegal opcodes, with declarative micro-op execution sequences.

## Usage

```rust
use nmos6502::{CPU6502, memory::Ram, Addressable};

let mut cpu = CPU6502::new();
let mut mem = Ram::new();

// Initialize memory and registers, then step cycle by cycle.
cpu.cycle(&mut mem);
```

The CPU interacts with memory exclusively through the `Addressable` trait, making it easy to plug into any bus or memory implementation.

## Harte CPU Test Suite

The `external/6502/v1/` directory contains the [Tom Harte 6502 JSON test suite](https://github.com/TomHarte/ProcessorTests) — 256 opcode files (`00.json` through `ff.json`), each with 10,000 randomized test cases (2.56 million total). Tests are parametrized with `rstest` for per-opcode isolation.

### Running tests

```bash
# All 256 opcodes
cargo test --test harte_tests

# Single opcode (by hex substring)
cargo test --test harte_tests a9       # LDA immediate
cargo test --test harte_tests 02       # KIL/HALT

# Verbose output (shows individual case failures)
cargo test --test harte_tests a9 -- --nocapture

# Sequential execution (deterministic output)
cargo test --test harte_tests -- --test-threads=1
```

Each failing test prints a summary: first 20 mismatches with register/RAM diffs, plus a count of remaining errors.

### Test format

Each JSON file contains an array of test cases. A single test case:

```json
{
  "name": "b1 28 b5",
  "initial": { "pc": 59082, "s": 39, "a": 57, "x": 33, "y": 174, "p": 96, "ram": [[59082, 177], ...] },
  "final":   { "pc": 59084, "s": 39, "a": 119, "x": 33, "y": 174, "p": 96, "ram": [[59082, 177], ...] },
  "cycles":  [[59082, 177, "read"], [59083, 40, "read"], ...]
}
```

The test harness initializes RAM and CPU registers from `initial`, steps `cycles.len()` cycles, then compares all registers and RAM against `final`.

### Benchmarks

```bash
cargo +nightly bench
```
