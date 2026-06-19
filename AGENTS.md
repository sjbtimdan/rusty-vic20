# AGENTS.md

Guidance for AI coding agents working in this repository.
Deep architectural reference in [ARCHITECTURE.md](ARCHITECTURE.md).
VIC-20 hardware references in [docs/REFERENCES.md](docs/REFERENCES.md).

## Fast Start

- Build: `cargo build`
- Test: `cargo test` (unit: `cargo test --lib`; integration: `cargo test --test '*'` — requires ROM files in `data/`)
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

## Architecture Overview

- Single crate (not a workspace), `edition = "2024"` (requires Rust >= 1.85.0).
- All modules re-exported via `pub mod` in `src/lib.rs`.
- Entries: `src/bin/vic20.rs` (emulator), `src/bin/disassembler.rs` (6502 disassembler).

### Core Emulation

- **`Addressable` trait** (`src/addressable.rs`): Foundational `read_byte`/`write_byte` interface. Implemented by `Memory`, `VIC`, `VIA`, and `Bus`. CPU interacts with the bus exclusively through `dyn Addressable` / `impl Addressable`.
- **`Bus`** (`src/bus.rs`): 64KB address router owning `Memory`, `VIC`, two `VIA`s, watchpoints, and framebuffer. `step_devices()` steps VIC, then VIA1 (with NMI), then VIA2 internal. Routes reads/writes to the correct device based on address range. `render_active_screen()` delegates to VIC.
- **`CPU6502`** (`src/cpu/cpu6502.rs`): Cycle-accurate — each `cpu.step()` is exactly one clock cycle. Uses a state machine (`cycle_count`, `operands_index`, `current_instruction_info`) for multi-cycle instructions.
- **Instruction executor traits** (`src/cpu/instruction_executor.rs`, `src/cpu/interrupt_handler.rs`, `src/cpu/addressing_mode.rs`): Traits enable `unimock` testing. `DefaultInstructionExecutor` is a zero-sized struct.
- **`VIC`** (`src/vic.rs`): Renders 176×184 text-mode screen from screen RAM + Character ROM + color RAM. Registers at 0x9000–0x900F.
- **`VIA`** (`src/via.rs`): 6522 chip — Timer1 countdown/underflow/latch, IFR/IER/IRQ logic (`Cell` for interior mutability), CA1 edge detect via `EdgeLatch` (`src/edge_latch.rs`). Port A/B for keyboard matrix. `port_b_callback` fires on port B writes (used for cassette motor control).

### Peripherals (`src/peripherals/`)

Re-exported via `src/peripherals/mod.rs`:

- **brake**: Speed control. Reads `BrakeSpeed` from a channel, adjusts cycle timing every 10k cycles against the real `Clock`. Speeds: Normal, Quarter, Half, TwoX, Max.
- **keyboard**: Matrix scanning; reads `HashSet<Key>` via `sync_channel(2)` from UI → `keyboard.step(port_b)` → `via2.set_port_a()`. Also handles paste injection and RESTORE key (CA1 on VIA1 via `set_ca1_pin`).
- **joystick**: Maps directional + fire to VIA port A bits; channel from UI.
- **cassette_player**: Motor sense via VIA1 port B callback; `.step()` advances tape counters.
- **serial_port**: IEC serial bus stubs.
- **direct_loader**: Bypasses KERNAL loader, injects bytes directly on `step()`.
- **speaker**: Sound output logic.

### Audio (`src/audio.rs`)

Uses `cpal` + `ringbuf` for audio output (44100 Hz, mono). `AudioProducer::push()` feeds samples; `AudioProducer::noop()` creates a no-op for tests. Sample generation happens in `EmulatorRunner::generate_audio()` mixing VIC + VIA2 CB2.

### Paste (`src/paste.rs`)

Clipboard paste: Unicode→PETSCII conversion, injected into KERNAL keyboard buffer at `0x0277` (count at `0x00C6`). `PasteQueue = Arc<Mutex<VecDeque<u8>>>`.

### Runner (`src/runner.rs`)

`EmulatorRunner` consolidates emulation state (bus, cpu, and all peripherals including brake and audio). Key methods:
- `step_keyboard()` — call before `step()`. Handles keyboard matrix + paste injection + RESTORE key.
- `step()` — one cycle: `step_devices` → `cpu.step` → peripherals → `brake.step()`.
- `generate_audio(elapsed_secs)` — generates audio samples for the given wall-clock time.
- `step_multiple(count)` — convenience loop.
- `from_receiver(...)` — full constructor; `default()` creates a no-audio runner (for tests).

### UI (`src/ui/`)

Three `pixels`/`winit` windows, all created in `resumed`:
- **screen**: 176×184 at 3x scale, displays VIC framebuffer + border.
- **keyboard**: PNG layout image + virtual keyboard with click/hold/flash interaction.
- **control**: Tabbed panel (Perf / Io / Joystick / Memory). Handles cassette file open, direct load, joystick virtual controls, memory expansion setting, and speed/brake control. No memory hex grid.

### Controller (`src/controller.rs`)

`Vic20Controller` implements `winit::ApplicationHandler`:
- **`resumed`**: Creates 3 windows, spawns `"vic20-core-loop"` worker thread with all shared state channels.
- **`window_event`**: Routes events to the correct window. Screen window gets keyboard events forwarded; control window actions trigger cassette/joystick/load/brake/reboot.
- **`about_to_wait`**: Nearest-deadline timing between 50Hz screen refresh and keyboard animation.
- **Paste**: Ctrl+V / Cmd+V → clipboard text → Unicode→PETSCII → `paste_queue` → injected into KERNAL buffer.
- **Shared state**: `Arc<Mutex<SharedVideoState>>` (framebuffer + border RGBA) and `SharedPerfState` (perf metrics). No memory or register mirroring.

### Tools (`src/tools.rs`)

- **debug**: `Breakpoint` trait + `LoggingAddressBreakpoint`, `MemoryWriteWatchpoint` with single-address and range predicates.
- **disassembler**: `DefaultDisassembler` — parse and format 6502 instructions.

## Threading and Shared State

- **`winit` event loop** runs on the main thread (required for macOS).
- **CPU/bus stepping** runs on a named worker thread (`"vic20-core-loop"`).
- Peripherals communicate via `sync_channel` — UI sends, emulator `try_recv()`s non-blockingly.
- `LoadQueue` (`.prg` files) and `PasteQueue` (clipboard) use `Arc<Mutex<VecDeque<>>>` with `try_lock()` in the emulator loop.
- **Locking asymmetry:** emulator uses `try_lock()` for non-blocking reads; UI uses blocking `lock()` for frame/perf reads.
- Video state (`SharedVideoState`) and performance metrics (`SharedPerformanceMetrics`) are the only `Arc<Mutex<>>` shared state for UI display.

## Known Pitfalls

- Avoid self-referential lifetime designs around CPU execution helpers; construct short-lived executors per step.
- In bus/device stepping, avoid aliasing mutable borrows of a field and `&mut self` in the same call path.
- For `unimock` with trait objects, local `Debug` impls may be needed for test expectations (see `src/addressable.rs`).
- Integration tests and benchmarks require ROM files in `data/` — they'll panic if ROMs are missing.
- `cargo test` without ROMs: use `cargo test --lib` for unit tests only.

## References

- [README.md](README.md) — project overview and roadmap
- [ARCHITECTURE.md](ARCHITECTURE.md) — full architectural detail
- [WIP.md](WIP.md) — missing rendering features
- [docs/REFERENCES.md](docs/REFERENCES.md) — VIC-20 hardware references
