# AGENTS.md

Guidance for AI coding agents working in this repository.
Deep architectural reference in [ARCHITECTURE.md](ARCHITECTURE.md) (note: some detail predates the nmos6502 extraction — trust code over prose).
VIC-20 hardware references in [docs/REFERENCES.md](docs/REFERENCES.md).

## Fast Start

- Build: `cargo build`
- Test: `cargo test` (unit-only: `cargo test --lib`; integration: `cargo test --test '*'` — requires ROM files in `data/`)
- Test nmos6502 crate: `cargo test -p nmos6502`
- Test single Harte opcode: `cargo test -p nmos6502 --test harte_tests a9` (hex substring of opcode)
- Format: `cargo +nightly fmt` — **must use nightly**; `cargo fmt` will error due to `unstable_features = true` in `rustfmt.toml`
- Lint: `cargo clippy` — run after every change and fix all warnings
- Lint nmos6502: `cargo clippy -p nmos6502`
- Bench: `cargo +nightly bench` (requires ROM files in `data/`, uses `#![feature(test)]` in `benches/run_bench.rs`)
- Bench nmos6502: `cargo +nightly bench -p nmos6502`
- Run emulator: `cargo run --bin vic20`
- Run disassembler: `cargo run --bin disassembler -- <file> [base_address] [disassemble_start_addr]`
- Enable logging: `RUST_LOG=debug cargo run --bin vic20` (uses `env_logger`)

## Project Structure

Two crates (NOT a workspace — `nmos6502` is a `path` dependency):

| Crate | Directory | Purpose |
|-------|-----------|---------|
| `rusty-vic20` | `/` | VIC-20 emulator: bus, VIC, VIA, peripherals, UI, controller. Edition 2024 (Rust >= 1.85.0). |
| `nmos6502` | `nmos6502/` | Cycle-perfect NMOS 6502 CPU: registers, ALU, micro-op sequences, all 151 opcodes + illegals. Edition 2021. |

Entry points: `src/bin/vic20.rs` (emulator — trivial main, env_logger init + controller run), `src/bin/disassembler.rs` (6502 disassembler).

## The Two `Addressable` Traits

There are TWO distinct `Addressable` traits — do not confuse them:

1. **`nmos6502::Addressable`** (`nmos6502/src/memory.rs`) — `read_byte(&mut self, address: u16) -> u8`. Takes `&mut self` because some hardware reads have side effects. This is what the CPU uses.
2. **`crate::hardware::addressable::Addressable`** (`src/hardware/addressable.rs`) — `read_byte(&self, address: u16) -> u8`. Takes `&self`, used by hardware device implementations (VIC, VIA, Memory).

`Bus` (`src/hardware/bus.rs`) implements BOTH. The `nmos6502::Addressable` impl delegates to the hardware `Addressable` impl. The emulator loop calls `cpu.cycle(&mut bus)` — the `&mut bus` satisfies `nmos6502::Addressable`'s `&mut self` requirement.

## CPU (nmos6502 crate)

- `CPU6502` is at `nmos6502/src/cpu.rs`. Created with `CPU6502::new()`.
- Execution model: micro-op sequences. `cpu.cycle(&mut bus)` executes exactly ONE micro-op per call (cycle-accurate). Multi-cycle instructions span multiple `cycle()` calls.
- NMI: `cpu.nmi_latch: EdgeLatch` (falling-edge triggered, `nmos6502/src/edge_latch.rs`). Set from VIA1 CA1 in `bus.step_devices()`.
- IRQ: `cpu.irq_line_low: bool`. Set from VIA2 in `bus.step_devices()`.
- Reset: `cpu.reset(&mut bus)` loads PC from `0xFFFC`-`0xFFFD`.
- The CPU is generic over `impl nmos6502::Addressable`, not coupled to the main crate's bus.

Test suite: `nmos6502/tests/harte_tests.rs` — 2.56M parametrized JSON test cases from the Tom Harte suite. Per-opcode isolation via `rstest`.

## Module Map (main crate, `src/lib.rs`)

| Module | Directory | Purpose |
|--------|-----------|---------|
| `controller` | `src/controller.rs` | `Vic20Controller` — winit event loop, thread spawning, cross-thread shared state |
| `emulator` | `src/emulator/` | `EmulatorRunner`, `spawn_emulator()`, `ThreadSenders`/`ThreadReceivers`, paste |
| `hardware` | `src/hardware/` | `Addressable` trait, `Bus`, `Memory`, `VIC`, `VIA`, `EdgeLatch` (hardware variant) |
| `peripherals` | `src/peripherals/` | Brake, keyboard (matrix), joystick, cassette, serial, speaker, direct loader |
| `tools` | `src/tools/` | `Breakpoint` trait, watchpoints, disassembler |
| `ui` | `src/ui/` | Audio (`audio.rs`), control panel, keyboard UI, screen rendering |
| `virtual_clock` | `src/virtual_clock.rs` | `Clock` trait + `SystemClock`/`MockClock` (testable keyboard timing) |

No `src/cpu/` directory — CPU code lives in `nmos6502/`.

## Key Types and Locations

- **`Bus`** — `src/hardware/bus.rs`: 64KB address router. Owns `Memory`, `VIC`, two `VIA`s, watchpoints, framebuffer. `via1` and `via2` are `pub`. `step_devices(&mut self, cpu)` steps VIA1 internal → NMI latch via `cpu.nmi_latch.set_level()` → VIA2 internal → sets `cpu.irq_line_low`.
- **`VIC`** — `src/hardware/vic.rs`: Renders text-mode screen into framebuffer. Dirty-flag optimization. Sound generators (`generate_sample()`).
- **`VIA`** — `src/hardware/via.rs`: 6522 chip. Uses `Cell` for interior mutability on `ifr`, `t1_counter`, `t1_latch`. `port_b_callback` for cassette motor. `joystick_right_pressed` field. CA1 edge detect via hardware `EdgeLatch` (`src/hardware/edge_latch.rs`).
- **`EmulatorRunner`** — `src/emulator/runner.rs`: Main orchestration. `step()` (keyboard → bus.step_devices → cpu.cycle → peripherals → brake.step). `generate_audio()` mixes VIC + VIA2 CB2. `run_loop()` is the core loop entry point.
- **`spawn_emulator()`** — `src/emulator/mod.rs`: Spawns `"vic20-core-loop"` thread. Creates `Bus`, `CPU6502`, `EmulatorRunner` internally.
- **`ThreadSenders` / `ThreadReceivers`** — `src/emulator/api.rs`: All channel types (keyboard, paste, load, cassette, joystick, direct_loader, brake, shutdown).

## UI Windows

Three `pixels`/`winit` windows, created in `Vic20Controller::resumed`:
- **screen** (`src/ui/screen/`): VIC framebuffer + border, 3x scale.
- **keyboard** (`src/ui/keyboard/`): PNG layout + virtual keyboard with click/hold/flash.
- **control** (`src/ui/control/`): Tabbed panel (Perf / Io / Joystick / Memory). Cassette, direct load, joystick, memory expansion, speed/brake.

## Threading and Shared State

- **`winit` event loop** runs on the main thread (required for macOS).
- **CPU/bus stepping** runs on a named worker thread (`"vic20-core-loop"`), spawned via `spawn_emulator()`.
- Keyboard input: UI sends `HashSet<Key>` via `sync_channel(2)`, emulator `try_recv()`s non-blockingly.
- `LoadQueue` (`.prg` files) and `PasteQueue` (clipboard) use `Arc<Mutex<VecDeque<>>>` with `try_lock()` in the emulator loop.
- **Locking asymmetry:** emulator uses `try_lock()` for non-blocking reads; UI uses blocking `lock()` for video/perf reads.
- **Shared state:** `Arc<Mutex<SharedVideoState>>` and `Arc<Mutex<SharedPerformanceMetrics>>`.

## Development Rules

- Write in TDD style: write a small test first, see it fail, fix with minimal code, repeat.
- Inline `#[cfg(test)]` modules with `rstest` fixtures and `unimock` for trait mocking.
- Integration tests in `tests/` require ROM files in `data/`. Unit tests (`cargo test --lib`) do not.
- ROM files needed: `data/basic.901486-01.bin`, `data/characters.901460-03.bin`, `data/kernal.901486-07.bin`
- Format: `cargo +nightly fmt` (max_width=120, field_init_shorthand, imports_granularity="Crate").
- No comments unless necessary for non-obvious logic.
- Do not edit `target/` artifacts.
- Preserve existing public APIs unless the task explicitly requires API changes.

## Known Pitfalls

- **Two `Addressable` traits** with different `&self` vs `&mut self` signatures on `read_byte`. When the CPU calls `read_byte`, it goes through `nmos6502::Addressable`. When hardware code calls it, it's the crate-local one. `Bus` bridges them.
- **Two `EdgeLatch` types**: `nmos6502::EdgeLatch` (falling-edge only, on CPU for NMI) and `crate::hardware::edge_latch::EdgeLatch` (rising/falling, on VIA for CA1). Not interchangeable.
- Avoid self-referential lifetime designs around CPU execution helpers; construct short-lived executors per step.
- In bus/device stepping, avoid aliasing mutable borrows of a field and `&mut self` in the same call path.
- For `unimock` with trait objects, a local `Debug` impl is needed for test expectations (see `src/hardware/addressable.rs:33-38`).
- Integration tests and benchmarks require ROM files in `data/` — they'll panic if ROMs are missing.
- `cargo test` without ROMs: use `cargo test --lib` for unit tests only.
- CI (`rust.yml`) runs `cargo +nightly test --verbose` on ubuntu-latest with ALSA dev libs.

## References

- [README.md](README.md) — project overview
- [ARCHITECTURE.md](ARCHITECTURE.md) — full architectural detail (some parts predate nmos6502 extraction)
- [WIP.md](WIP.md) — missing features / roadmap
- [docs/REFERENCES.md](docs/REFERENCES.md) — VIC-20 hardware references
- [nmos6502/README.md](nmos6502/README.md) — CPU crate docs, Harte test suite usage
- [nmos6502/docs/cycle-perfect-cpu.md](nmos6502/docs/cycle-perfect-cpu.md) — micro-op execution model
