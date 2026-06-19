# AGENTS.md

Guidance for AI coding agents working in this repository.
Deep architectural reference in [ARCHITECTURE.md](ARCHITECTURE.md).
VIC-20 hardware references in [docs/REFERENCES.md](docs/REFERENCES.md).

## Fast Start

- Build: `cargo build`
- Test: `cargo test` (unit-only: `cargo test --lib`; integration: `cargo test --test '*'` — requires ROM files in `data/`)
- Format: `cargo +nightly fmt` — **must use nightly**; `cargo fmt` will error due to `unstable_features = true` in `rustfmt.toml`
- Lint: `cargo clippy` — run after every change and fix all warnings
- Bench: `cargo +nightly bench` (requires ROM files in `data/`, uses `#![feature(test)]`)
- Run emulator: `cargo run --bin vic20`
- Run disassembler: `cargo run --bin disassembler -- <file> [base_address] [disassemble_start_addr]`
- Enable logging: `RUST_LOG=debug cargo run --bin vic20` (uses `env_logger`)

## Development Rules

- Write in TDD style: write a small test first, see it fail, fix with minimal code, repeat.
- Inline `#[cfg(test)]` modules with `rstest` fixtures and `unimock` for trait mocking.
- Integration tests in `tests/` require ROM files in `data/`. Unit tests (`cargo test --lib`) do not.
- ROM files needed: `data/basic.901486-01.bin`, `data/characters.901460-03.bin`, `data/kernal.901486-07.bin`

## File Hygiene

- Format with `cargo +nightly fmt` (max_width=120, field_init_shorthand, imports_granularity="Crate").
- Do not edit `target/` artifacts.
- Preserve existing public APIs unless the task explicitly requires API changes.
- No comments unless necessary for non-obvious logic.

## Module Architecture

Single crate (not a workspace), `edition = "2024"` (requires Rust >= 1.85.0). `#![feature(likely_unlikely)]` in `src/lib.rs`.
Entries: `src/bin/vic20.rs` (emulator — trivial main, env_logger init + controller run), `src/bin/disassembler.rs` (6502 disassembler).

### Top-level modules (`src/lib.rs`)

| Module | Directory | Purpose |
|--------|-----------|---------|
| `controller` | `src/controller.rs` | `Vic20Controller` — winit event loop, thread spawning, cross-thread shared state |
| `cpu` | `src/cpu/` | `CPU6502`, registers, instructions, addressing modes, executor traits |
| `emulator` | `src/emulator/` | `EmulatorRunner`, `spawn_emulator()`, `ThreadSenders`/`ThreadReceivers`, paste |
| `hardware` | `src/hardware/` | `Addressable` trait, `Bus`, `Memory`, `VIC`, `VIA`, `EdgeLatch` |
| `peripherals` | `src/peripherals/` | Brake, keyboard (matrix), joystick, cassette, serial, speaker, direct loader |
| `tools` | `src/tools/` | `Breakpoint` trait, watchpoints, disassembler |
| `ui` | `src/ui/` | Audio (`audio.rs`), control panel, keyboard UI, screen rendering |
| `virtual_clock` | `src/virtual_clock.rs` | `Clock` trait + `SystemClock`/`MockClock` (for testable keyboard timing) |

### Key types and locations

- **`Addressable` trait** — `src/hardware/addressable.rs`: `read_byte`/`write_byte`. Implemented by `Memory`, `VIC`, `VIA`, and `Bus`. CPU interacts with the bus exclusively through `dyn Addressable` / `impl Addressable`.
- **`Bus`** — `src/hardware/bus.rs`: 64KB address router. Owns `Memory`, `VIC`, two `VIA`s, watchpoints, and framebuffer. `via1` and `via2` are `pub`. `step_devices()` steps VIA1 internal → NMI latch → VIA2 internal → IRQ line.
- **`CPU6502`** — `src/cpu/cpu6502.rs`: Cycle-accurate (`cpu.step()` = exactly one clock cycle). State machine with `cycle_count`, `operands_index`, `current_instruction_info`. Uses `InstructionExecutor`/`InterruptHandler`/`OperandResolution` traits for testability.
- **`VIC`** — `src/hardware/vic.rs`: Renders 176×184 text-mode screen. Has sound generators (`generate_sample()`). Dirty-flag optimization.
- **`VIA`** — `src/hardware/via.rs`: 6522 chip. Uses `Cell` for interior mutability on `ifr`, `t1_counter`, `t1_latch`. `port_b_callback` for cassette motor. `joystick_right_pressed` field. CA1 edge detect via `EdgeLatch` (`src/hardware/edge_latch.rs`).
- **`EmulatorRunner`** — `src/emulator/runner.rs`: Main emulation orchestrator. `step_keyboard()` → `step()` (bus.step_devices → cpu.step → peripherals → brake.step). `generate_audio()` mixes VIC + VIA2 CB2. `run_loop()` is the core loop entry point.
- **`spawn_emulator()`** — `src/emulator/mod.rs`: Spawns the `"vic20-core-loop"` thread with all channel receivers.
- **`ThreadSenders` / `ThreadReceivers`** — `src/emulator/api.rs`: All channel types (keyboard, paste, load, cassette, joystick, direct_loader, brake, shutdown).

### UI windows

Three `pixels`/`winit` windows, created in `Vic20Controller::resumed`:
- **screen** (`src/ui/screen/`): 176×184 at 3x scale, displays VIC framebuffer + border.
- **keyboard** (`src/ui/keyboard/`): PNG layout image + virtual keyboard with click/hold/flash interaction.
- **control** (`src/ui/control/`): Tabbed panel (Perf / Io / Joystick / Memory). Cassette, direct load, joystick, memory expansion, speed/brake.

### Tools

- **debug** (`src/tools/debug.rs`): `Breakpoint` trait + `LoggingAddressBreakpoint`, `MemoryWriteWatchpoint` with single-address and range predicates.
- **disassembler** (`src/tools/disassembler.rs`): `DefaultDisassembler` — parse and format 6502 instructions.

## Threading and Shared State

- **`winit` event loop** runs on the main thread (required for macOS).
- **CPU/bus stepping** runs on a named worker thread (`"vic20-core-loop"`), spawned via `spawn_emulator()` in `src/emulator/mod.rs`.
- Peripherals communicate via `sync_channel` — UI sends, emulator `try_recv()`s non-blockingly.
- `LoadQueue` (`.prg` files) and `PasteQueue` (clipboard) use `Arc<Mutex<VecDeque<>>>` with `try_lock()` in the emulator loop.
- **Locking asymmetry:** emulator uses `try_lock()` for non-blocking reads; UI uses blocking `lock()` for video/perf reads.
- **Shared state:** only `Arc<Mutex<SharedVideoState>>` and `Arc<Mutex<SharedPerformanceMetrics>>`. No memory mirror or register debug state.

## Known Pitfalls

- Avoid self-referential lifetime designs around CPU execution helpers; construct short-lived executors per step.
- In bus/device stepping, avoid aliasing mutable borrows of a field and `&mut self` in the same call path.
- For `unimock` with trait objects, a local `Debug` impl is needed for test expectations (see `src/hardware/addressable.rs:33-38`).
- Integration tests and benchmarks require ROM files in `data/` — they'll panic if ROMs are missing.
- `cargo test` without ROMs: use `cargo test --lib` for unit tests only.
- CI (`rust.yml`) runs `cargo +nightly test --verbose` on ubuntu-latest with ALSA dev libs.

## References

- [README.md](README.md) — project overview
- [ARCHITECTURE.md](ARCHITECTURE.md) — full architectural detail
- [WIP.md](WIP.md) — missing features / roadmap
- [docs/REFERENCES.md](docs/REFERENCES.md) — VIC-20 hardware references
