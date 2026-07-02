# AGENTS.md

Guidance for AI coding agents working in this repository.
Deep architectural reference in [ARCHITECTURE.md](ARCHITECTURE.md) (note: some detail predates the nmos6502 extraction — trust code over prose).
VIC-20 hardware references in [docs/REFERENCES.md](docs/REFERENCES.md).

OpenCode configuration in `.opencode/`.

## Fast Start

- Build: `cargo build`
- Test: `cargo test` (unit-only: `cargo test --lib`; integration: `cargo test --test '*'` — requires ROM files in `data/`)
- Test nmos6502 crate: `cargo test -p rusty-nmos6502`
- Test single Harte opcode: `cargo test -p rusty-nmos6502 --test harte_tests a9` (hex substring of opcode)
- Format: `cargo +nightly fmt` — **must use nightly**; `cargo fmt` errors due to `unstable_features = true` in `rustfmt.toml`
- Lint: `cargo clippy` — run after every change and fix all warnings
- Lint nmos6502: `cargo clippy -p rusty-nmos6502`
- Bench: `cargo +nightly bench` (requires ROM files in `data/`, uses `#![feature(test)]`)
- Bench nmos6502: `cargo +nightly bench -p rusty-nmos6502`
- Run emulator: `cargo run --bin vic20` (logging: `RUST_LOG=info cargo run --bin vic20` or `vic20-info-log.sh`)
- Run disassembler: `cargo run -p rusty-nmos6502 --bin disassembler -- <file> [base_address] [disassemble_start_addr]`

## Project Structure

Two crates (NOT a workspace — `nmos6502` is a `path` dependency):

| Crate | Directory | Purpose |
|-------|-----------|---------|
| `rusty-vic20` | `/` | VIC-20 emulator: bus, VIC, VIA, peripherals, UI, controller. Edition 2024 (Rust >= 1.85.0). |
| `nmos6502` | `nmos6502/` | Cycle-perfect NMOS 6502 CPU. Edition 2021. |

Entry points: `src/bin/vic20.rs` (emulator — trivial main, env_logger init + controller run), `nmos6502/src/bin/disassembler.rs` (6502 disassembler).

## The Two `Addressable` Traits

There are TWO distinct `Addressable` traits — do not confuse them:

1. **`nmos6502::Addressable`** (`nmos6502/src/memory.rs`) — `read_byte(&mut self, address: u16) -> u8`. Takes `&mut self` because some hardware reads have side effects. This is what the CPU uses.
2. **`crate::hardware::addressable::Addressable`** (`src/hardware/addressable.rs`) — `read_byte(&self, address: u16) -> u8`. Takes `&self`, used by hardware device implementations (VIC, VIA, Memory).

`Bus` (`src/hardware/bus.rs`) implements BOTH. The `nmos6502::Addressable` impl delegates to the hardware `Addressable` impl. The emulator loop calls `cpu.cycle(&mut bus)` — the `&mut bus` satisfies `nmos6502::Addressable`'s `&mut self` requirement.

## CPU (`nmos6502` crate)

See [`nmos6502/AGENTS.md`](nmos6502/AGENTS.md) for the full crate guide — execution model, bus interface, test suite, architecture, and gotchas.

Key facts that affect the main crate:
- `CPU6502` at `nmos6502/src/cpu.rs`. Execution model: micro-op sequences — `cpu.cycle(&mut bus)` executes ONE micro-op per call (cycle-accurate).
- All instruction sequences are in `nmos6502/src/sequences.rs`, built with **`const fn` helpers** (`seq_imm`, `seq_zp`, `seq_abs`, `rmw_zp`, `branch_seq`, etc.).
- NMI: `cpu.nmi_latch: EdgeLatch` (falling-edge triggered). Set from VIA1 CA1 in `bus.step_devices()`.
- IRQ: `cpu.irq_line_low: bool`. Set from VIA2 in `bus.step_devices()`.
- CPU is generic over `impl nmos6502::Addressable`, not coupled to the main crate's bus.

## Module Map (main crate)

| Module | File/Directory | Purpose |
|--------|---------------|---------|
| `controller` | `src/controller.rs` | `Vic20Controller` — winit event loop, thread spawning, cross-thread shared state |
| `emulator` | `src/emulator/` | `EmulatorRunner`, `spawn_emulator()`, `ThreadSenders`/`ThreadReceivers`, paste |
| `hardware` | `src/hardware/` | `Addressable` trait, `Bus`, `Memory`, `VIC`, `VIA`, `EdgeLatch` (hardware variant) |
| `peripherals` | `src/peripherals/` | Brake, keyboard (matrix), joystick, cassette, serial, speaker, direct loader |
| `tools` | `src/tools/` | `Breakpoint` trait, watchpoints, disassembler |
| `ui` | `src/ui/` | Audio (`audio.rs`), control panel, keyboard UI, screen rendering |
| `virtual_clock` | `src/virtual_clock.rs` | `Clock` trait + `SystemClock`/`MockClock` |

## Key Types

- **`Bus`** — `src/hardware/bus.rs`: 64KB address router. Owns `Memory`, `VIC`, two `VIA`s, watchpoints, framebuffer. `step_devices(&mut self, cpu)` steps VIA1 → NMI latch → VIA2 → IRQ line.
- **`VIC`** — `src/hardware/vic.rs`: Text-mode screen rendering into framebuffer. Dirty-flag optimization. `generate_sample()` for sound.
- **`VIA`** — `src/hardware/via.rs`: 6522 chip. Uses `Cell` for interior mutability on `ifr`, `t1_counter`, `t1_latch`. CA1 edge detect via hardware `EdgeLatch`.
- **`EmulatorRunner`** — `src/emulator/runner.rs`: Main orchestration. `step()` order: keyboard → bus.step_devices → cpu.cycle → peripherals → brake.
- **`spawn_emulator()`** — `src/emulator/mod.rs`: Spawns `"vic20-core-loop"` thread, creates `Bus`, `CPU6502`, `EmulatorRunner` internally.
- **`ThreadSenders` / `ThreadReceivers`** — `src/emulator/api.rs`: All channel types.

## Threading

- **`winit` event loop** on main thread (required for macOS).
- **CPU/bus stepping** on `"vic20-core-loop"` worker thread.
- Keyboard input: `sync_channel(2)`, emulator `try_recv()`s non-blockingly.
- `LoadQueue` / `PasteQueue`: `Arc<Mutex<VecDeque<>>>` with `try_lock()` in emulator loop.
- **Locking asymmetry:** emulator uses `try_lock()` (non-blocking); UI uses blocking `lock()`.
- **Shared state:** `Arc<Mutex<SharedVideoState>>` and `Arc<Mutex<SharedPerformanceMetrics>>`.

## Known Pitfalls

- **Two `Addressable` traits** with different `&self` vs `&mut self` on `read_byte`. CPU goes through `nmos6502::Addressable`; hardware code uses crate-local one. `Bus` bridges them.
- **Two `EdgeLatch` types**: `nmos6502::EdgeLatch` (falling-edge only, CPU NMI) and `crate::hardware::edge_latch::EdgeLatch` (rising/falling, VIA CA1). Not interchangeable.
- Avoid self-referential lifetime designs around CPU executors; construct short-lived executors per step.
- Avoid aliasing mutable borrows of a field and `&mut self` in the same call path in bus/device stepping.
- `unimock` with trait objects needs a local `Debug` impl (see `src/hardware/addressable.rs:33-38`).
- Integration tests (`tests/`) and benchmarks (`benches/`) require ROM files in `data/` — they panic if missing. Use `cargo test --lib` for ROM-free unit tests.
- CI runs on ubuntu-latest with `cargo +nightly test --verbose` and `libasound2-dev` (ALSA) installed.

## Development Rules

- Inline `#[cfg(test)]` modules with `rstest` fixtures and `unimock` for trait mocking.
- Format: `cargo +nightly fmt` (max_width=120, field_init_shorthand, imports_granularity="Crate").
- No comments unless necessary for non-obvious logic.
- Do not edit `target/` artifacts.
- Preserve existing public APIs unless the task explicitly requires API changes.
- ROM files needed: `data/basic.901486-01.bin`, `data/characters.901460-03.bin`, `data/kernal.901486-07.bin`

## References

- [README.md](README.md) — project overview
- [ARCHITECTURE.md](ARCHITECTURE.md) — full architectural detail
- [WIP.md](WIP.md) — missing features / roadmap
- [docs/REFERENCES.md](docs/REFERENCES.md) — VIC-20 hardware references
- [nmos6502/AGENTS.md](nmos6502/AGENTS.md) — CPU crate agent guide
- [nmos6502/README.md](nmos6502/README.md) — CPU crate docs, Harte test suite usage
- [nmos6502/docs/cycle-perfect-cpu.md](nmos6502/docs/cycle-perfect-cpu.md) — micro-op execution model
