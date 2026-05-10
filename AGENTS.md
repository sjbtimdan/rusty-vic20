# AGENTS.md

Guidance for AI coding agents working in this repository.
Deep architectural reference in [ARCHITECTURE.md](ARCHITECTURE.md).
VIC-20 hardware references in [docs/REFERENCES.md](docs/REFERENCES.md).

## Fast Start

- Build: `cargo build`
- Test: `cargo test`
- Format: `cargo +nightly fmt` (config in `rustfmt.toml` — uses nightly-only features)
- Lint: `cargo clippy` — run after every change and fix all warnings
- Bench: `cargo +nightly bench` (uses `#![feature(test)]`)
- Run emulator: `cargo run --bin vic20`
- Run disassembler: `cargo run --bin disassembler -- <file> [base_address] [disassemble_start_addr]`
- Enable logging: `RUST_LOG=info cargo run --bin vic20` (uses `env_logger`)

Note: `rm -rf target` if `cargo build` fails after a Rust upgrade — the crate uses `edition = "2024"`.

## ROM File Prerequisites

The emulator and integration tests load ROM files from the `data/` directory (not in git):

```
data/basic.901486-01.bin      (BASIC ROM, 8KB at 0xC000)
data/characters.901460-03.bin (Character ROM, 4KB at 0x8000)
data/kernal.901486-07.bin     (KERNEL ROM, 8KB at 0xE000)
```

`cargo test` panics without these files — unit tests in `src/bus.rs` call `load_standard_roms_from_data_dir()`.

## Architecture Overview

- Single crate (not a workspace), `edition = "2024"` (requires Rust >= 1.85.0).
- All modules re-exported in `src/lib.rs`.
- **`Addressable` trait** (`src/addressable.rs`): The foundational `read_byte`/`write_byte` interface. Implemented by `Memory`, `VIC`, `VIA`, and `Bus` itself. The CPU interacts with the bus exclusively through `dyn Addressable` / `impl Addressable`.
- **`Bus`** (`src/bus.rs`): 64KB address router owning `Memory`, `VIC`, two `VIA` (VIA1 at 0x9110, VIA2 at 0x9120), watchpoints, and a framebuffer. `step_devices()` steps each device per cycle; `render_active_screen()` delegates to VIC.
- **`CPU6502`** (`src/cpu/cpu6502.rs`): Cycle-accurate emulation — each `cpu.step(&mut bus, &executor)` call is exactly one clock cycle. Uses a state machine (`cycle_count`, `operands_index`, `current_instruction_info`) for multi-cycle instructions. 151 opcodes decoded in `instructions.rs`.
- **Instruction executor traits** (`src/cpu/instruction_executor.rs`, `src/cpu/interrupt_handler.rs`, `src/cpu/addressing_mode.rs`): Traits enable `unimock` testing without real CPU/memory. `DefaultInstructionExecutor` is a zero-sized struct.
- **`VIC`** (`src/vic.rs`): Renders 176×184 text-mode screen from screen RAM + Character ROM + color RAM. Registers at 0x9000–0x900F.
- **`VIA`** (`src/via.rs`): 6522 chip — Timer1 countdown/underflow/latch, IFR/IER/IRQ logic (`Cell` for interior mutability), CA1 edge detect. Port A/B for keyboard matrix. Timer2 not yet counting down.
- **UI** (`src/ui/`): Three `pixels`/`winit` windows — screen (176×184 at 3x), keyboard overlay (PNG image + virtual keyboard), debug (hex memory grid, registers, perf).
- **Keyboard** (`src/keyboard.rs`, `src/ui/keyboard/`): Physical keys → `HashSet<Key>` via `sync_channel(2)` → emulator thread → `keyboard.step(port_b)` → `via2.set_port_a()`.
- **Debug** (`src/debug/`): `Arc<Mutex<>>` shared state for memory mirror (64KB), registers, perf metrics, and `Vec`-based pending-write channels for debugger→emulator edits.
- **Controller** (`src/controller.rs`): `winit::ApplicationHandler` — spawns `"vic20-core-loop"` worker thread in `resumed`, manages frame timing at 50Hz in `about_to_wait`.
- **Tools** (`src/tools/`): Breakpoints/watchpoints (`debug.rs`), 6502 disassembler (`disassembler.rs`).
- **`Clock` trait** (`src/virtual_clock.rs`): `SystemClock`/`MockClock` abstraction for testing time-dependent keyboard interactions.

Full detail: [ARCHITECTURE.md](ARCHITECTURE.md).

## Threading and Shared State

- **`winit` event loop** runs on the main thread (required for macOS).
- **CPU/bus stepping** runs on a named worker thread (`"vic20-core-loop"`).
- Shared state uses `Arc<Mutex<>>`: `SharedVideoState` (framebuffer + border RGBA), `SharedMemory` (64KB mirror), `SharedRegisters`, `SharedPerformanceMetrics`, and two `Vec`-based pending-write channels for debugger interaction.
- **Locking asymmetry:** CPU loop uses `try_lock()` for non-blocking reads of pending writes (skips if held). UI thread uses blocking `lock()` for frame/memory/register reads.
- Keyboard: `KeyboardState.physical_keys` (`HashSet<Key>`) sent via `SyncSender` from UI thread to emulator. Emulator thread uses `try_recv()` — non-blocking.
- Frame timing in `about_to_wait`: computes nearest deadline between 50Hz screen refresh and keyboard animation deadlines, uses `ControlFlow::WaitUntil`.

## Known Pitfalls

- Avoid self-referential lifetime designs around CPU execution helpers; construct short-lived executors per step when needed.
- In bus/device stepping, avoid aliasing mutable borrows of a field and `&mut self` in the same call path.
- For `unimock` with trait objects, local `Debug` impls may be needed for test expectations (see `src/addressable.rs:33-38`).

## Testing

- Inline `#[cfg(test)]` modules with `rstest` fixtures and `unimock` for trait mocking.
- Integration tests in `tests/` require ROM files in `data/`.
- `cargo test` runs both unit and integration tests. Use `cargo test --lib` for unit tests only.
- Benchmarks: `cargo +nightly bench`.

## File Hygiene

- Format with `cargo +nightly fmt` (config: max_width=120, use_field_init_shorthand, reorder_imports, imports_granularity="Crate").
- Do not edit `target/` artifacts.
- Preserve existing public APIs unless the task explicitly requires API changes.
- No comments unless necessary for non-obvious logic.

## References

- Project overview: [README.md](README.md)
- Architecture deep-dive: [ARCHITECTURE.md](ARCHITECTURE.md)
- Roadmap and status: [docs/PLAN.md](docs/PLAN.md)
- VIA implementation notes: [docs/VIA.md](docs/VIA.md)
- VIC-20 reference links: [docs/REFERENCES.md](docs/REFERENCES.md)
- Missing rendering features: [WIP.md](WIP.md)
