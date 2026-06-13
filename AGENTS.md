# AGENTS.md

Guidance for AI coding agents working in this repository.
Deep architectural reference in [ARCHITECTURE.md](ARCHITECTURE.md).
VIC-20 hardware references in [docs/REFERENCES.md](docs/REFERENCES.md).

## Fast Start

- Build: `cargo build`
- Test: `cargo test` (unit tests: `cargo test --lib`)
- Format: `cargo +nightly fmt` — **must use nightly**; `cargo fmt` will error due to `unstable_features = true` in `rustfmt.toml`
- Lint: `cargo clippy` — run after every change and fix all warnings
- Bench: `cargo +nightly bench` (requires ROM files in `data/`, uses `#![feature(test)]`)
- Run emulator: `cargo run --bin vic20`
- Run disassembler: `cargo run --bin disassembler -- <file> [base_address] [disassemble_start_addr]`
- Enable logging: `RUST_LOG=info cargo run --bin vic20` (uses `env_logger`)

## Development Rules

- Write in TDD style where possible: write a small test first, see it fail, fix with minimal code, repeat.
- Inline `#[cfg(test)]` modules with `rstest` fixtures and `unimock` for trait mocking.
- Integration tests in `tests/` require ROM files in `data/`; they'll panic if ROMs are missing.

## File Hygiene

- Format with `cargo +nightly fmt` (max_width=120, field_init_shorthand, imports_granularity="Crate").
- Do not edit `target/` artifacts.
- Preserve existing public APIs unless the task explicitly requires API changes.
- No comments unless necessary for non-obvious logic.

## Architecture Overview

- Single crate (not a workspace), `edition = "2024"` (requires Rust >= 1.85.0).
- All modules re-exported via `pub mod` in `src/lib.rs`.

### Core Emulation

- **`Addressable` trait** (`src/addressable.rs`): Foundational `read_byte`/`write_byte` interface. Implemented by `Memory`, `VIC`, `VIA`, and `Bus`. CPU interacts with the bus exclusively through `dyn Addressable` / `impl Addressable`.
- **`Bus`** (`src/bus.rs`): 64KB address router owning `Memory`, `VIC`, two `VIA`s, watchpoints, and framebuffer. Routes reads/writes to the correct device based on address range. `step_devices()` steps each device per cycle; `render_active_screen()` delegates to VIC.
- **`CPU6502`** (`src/cpu/cpu6502.rs`): Cycle-accurate — each `cpu.step()` is exactly one clock cycle. Uses a state machine (`cycle_count`, `operands_index`, `current_instruction_info`) for multi-cycle instructions.
- **Instruction executor traits** (`src/cpu/instruction_executor.rs`, `src/cpu/interrupt_handler.rs`, `src/cpu/addressing_mode.rs`): Traits enable `unimock` testing without real CPU/memory. `DefaultInstructionExecutor` is a zero-sized struct.
- **`VIC`** (`src/vic.rs`): Renders 176×184 text-mode screen from screen RAM + Character ROM + color RAM. Registers at 0x9000–0x900F.
- **`VIA`** (`src/via.rs`): 6522 chip — Timer1 countdown/underflow/latch, IFR/IER/IRQ logic (`Cell` for interior mutability), CA1 edge detect. Port A/B for keyboard matrix. Timer2 not yet counting down. `port_b_callback` fires on port B writes (used for cassette motor control).

### Peripherals (`src/peripherals/`)

- **keyboard**: Matrix scanning; reads `HashSet<Key>` via `sync_channel(2)` from UI → `keyboard.step(port_b)` → `via2.set_port_a()`. Also handles paste injection and RESTORE key (CA1 on VIA1).
- **joystick**: Maps directional + fire to VIA port A bits; channel from UI.
- **cassette_player**: Motor sense via VIA1 port B callback; `.step()` advances tape counters.
- **serial_port**: IEC serial bus stubs.
- **direct_loader**: Bypasses KERNAL loader, injects bytes directly on `step()`.
- **speaker**: Sound output stubs.

### Runner (`src/runner.rs`)

`EmulatorRunner` consolidates emulation state (bus, cpu, keyboard, joystick, cassette, serial, direct_loader) and provides `step()` for a single cycle and `step_keyboard()` for keyboard/paste handling. Call order: `step_keyboard()` before `step()`.

### UI (`src/ui/`)

Three `pixels`/`winit` windows, all created in `resumed`:
- **screen**: 176×184 at 3x scale, displays VIC framebuffer + border.
- **keyboard**: PNG layout image + virtual keyboard with click/hold/flash interaction.
- **control**: Tabbed panel (Perf / Io / Joystick / Memory). Handles cassette file open, direct load, joystick virtual controls, memory expansion setting, and reboot. No memory hex grid — much simpler than the old debug window.

### Controller (`src/controller.rs`)

`Vic20Controller` implements `winit::ApplicationHandler`:
- **`resumed`**: Creates 3 windows, spawns `"vic20-core-loop"` worker thread with all shared state channels.
- **`window_event`**: Routes events to the correct window. Screen window gets keyboard events forwarded to the keyboard handler; control window actions trigger cassette/joystick/load/reboot.
- **`about_to_wait`**: Nearest-deadline timing between 50Hz screen refresh and keyboard animation.
- **Paste**: Ctrl+V / Cmd+V → clipboard text → Unicode→PETSCII → `paste_queue` → injected into KERNAL buffer.

### Tools (`src/tools/`)

- **debug**: `Breakpoint` trait + `LoggingAddressBreakpoint`, `MemoryWriteWatchpoint` with single-address and range predicates.
- **disassembler**: `DefaultDisassembler` — parse and format 6502 instructions.

## Threading and Shared State

- **`winit` event loop** runs on the main thread (required for macOS).
- **CPU/bus stepping** runs on a named worker thread (`"vic20-core-loop"`).
- Shared state uses `Arc<Mutex<>>` for `SharedVideoState` (framebuffer + border RGBA) and `SharedPerformanceMetrics`.
- Peripherals communicate via `sync_channel` — UI sends, emulator `try_recv()`s non-blockingly.
- `LoadQueue` (`.prg` files) and `PasteQueue` (clipboard) use `Arc<Mutex<VecDeque<>>>` with `try_lock()` in the emulator loop.
- **Locking asymmetry:** emulator uses `try_lock()` for non-blocking reads; UI uses blocking `lock()` for frame/perf reads.

## Known Pitfalls

- Avoid self-referential lifetime designs around CPU execution helpers; construct short-lived executors per step.
- In bus/device stepping, avoid aliasing mutable borrows of a field and `&mut self` in the same call path.
- For `unimock` with trait objects, local `Debug` impls may be needed for test expectations (see `src/addressable.rs:33-38`).
- Integration tests and benchmarks require ROM files in `data/` — they'll panic if ROMs are missing.

## References

- [README.md](README.md) — project overview and roadmap
- [ARCHITECTURE.md](ARCHITECTURE.md) — full architectural detail
- [WIP.md](WIP.md) — missing rendering features
