# AGENTS.md

Guidance for AI coding agents working in this repository.
References on the Vic 20 to guide implementation are in [docs/REFERENCES.md]

## Fast Start

- Build: `cargo build`
- Test: `cargo test`
- Format: `cargo +nightly fmt` (config in `rustfmt.toml` — uses nightly-only features)
- Lint: `cargo clippy` — run after every change and fix all warnings
- Bench: `cargo +nightly bench` (uses `#![feature(test)]`)
- Run emulator: `cargo run --bin vic20`
- Run disassembler: `cargo run --bin disassembler -- <file> [base_address] [disassemble_start_addr]`
- Enable logging: `RUST_LOG=info cargo run --bin vic20` (uses `env_logger`)

Note: `rm -rf target` if `cargo build` fails after a Rust upgrade, since the crate uses `edition = "2024"`.

## ROM File Prerequisites

The emulator and integration tests load ROM files from the `data/` directory (not in git):

```
data/basic.901486-01.bin      (BASIC ROM, 8KB at 0xC000)
data/characters.901460-03.bin (Character ROM, 4KB at 0x8000)
data/kernal.901486-07.bin     (KERNEL ROM, 8KB at 0xE000)
```

`cargo test` panics without these files because unit tests in `src/bus.rs` call `load_standard_roms_from_data_dir()`.

## Architecture

- Single crate (not a workspace). Top-level modules re-exported in `src/lib.rs`.
- `edition = "2024"` — requires Rust >= 1.85.0.
- `src/addressable.rs`: `Addressable` trait for read/write across the 64KB bus. Trait has a `#[cfg(test)]` `Debug` impl for `dyn Addressable` needed by `unimock`.
- `src/bus.rs`: 64KB address space routing. Owns `Memory`, `VIC`, two `VIA` instances (VIA1 at 0x9110, VIA2 at 0x9120), watchpoints, and a framebuffer. ROMs loaded from `data/` on startup.
- `src/cpu/`: 6502 execution core — `cpu6502.rs` (struct + reset), `instructions.rs` (opcode dispatch), `addressing_mode.rs` (modes), `instruction_executor.rs` (trait for mocking), `instruction_tracking.rs` (cycle counts), `interrupt_handler.rs`, `registers.rs`.
- `src/vic.rs`: VIC-I chip — text-mode rendering, screen control, palette. Registers at 0x9000–0x900F.
- `src/via.rs`: 6522 VIA chip. Timer 1 countdown/underflow/latch/reload, CA1 edge detect, IFR/IER/IRQ logic. Reads/writes for Ports A/B, DDR, Timer2, shift reg, ACR, PCR. Timer 2 does not count down yet.
- `src/ui/screen/`: Video display — `renderer.rs` (framebuffer constants, palette), `display.rs` (pixels-based window).
- `src/ui/keyboard/`: On-screen keyboard overlay — `mod.rs` (key regions, `Key` enum, `KeyboardState`), `display.rs` (separate winit window).
- `src/debug/mod.rs` and `src/debug/display.rs`: Debug overlays showing registers, memory, and perf metrics in a third window.
- `src/keyboard.rs`: Physical-key → VIA port B/A matrix. Receives key sets via `mpsc::sync_channel(2)` from the UI thread.
- `src/tools/disassembler.rs`: 6502 disassembler used by the `disassembler` binary.
- `src/bin/vic20.rs`: CLI entrypoint accepting optional tick-duration in microseconds.
- `src/bin/disassembler.rs`: Standalone disassembler. Accepts filename, optional base_address (hex), optional start_address (hex).
- `src/controller.rs`: `ApplicationHandler` managing three windows (screen, keyboard, debug). Creates a worker thread for the CPU/bus loop.
- `src/virtual_clock.rs`: Clock abstraction for cycle timing.

## Threading and Shared State

- `winit` event loop runs on the main thread (required for macOS).
- CPU/bus stepping runs on a named worker thread (`"vic20-core-loop"`).
- Shared state uses `Arc<Mutex<>>`: `SharedVideoState` (framebuffer + border RGBA), `SharedMemory` (64KB mirror), `SharedRegisters`, `SharedPerformanceMetrics`, and two `Vec`-based pending-write channels for debugger interaction.
- CPU loop uses `try_lock()` for non-blocking reads of pending writes; UI thread uses blocking `lock()` for frame/memory/register reads.
- Keyboard state: `KeyboardState.physical_keys` (a `HashSet<Key>`) is sent via `SyncSender` from UI thread to the emulator thread each input event. Inside the emulator loop, `keyboard.step(via2.port_b())` converts physical keys to VIA port A values.
- Frame timing in `about_to_wait`: computes nearest deadline between 50Hz screen refresh and keyboard animation deadlines, uses `ControlFlow::WaitUntil`.

## Known Pitfalls

- Avoid self-referential lifetime designs around CPU execution helpers; construct short-lived executors per step when needed.
- In bus/device stepping, avoid aliasing mutable borrows of a field and `&mut self` in the same call path.
- For `unimock` with trait objects, local `Debug` impls for trait objects may be needed for test expectations (see `src/addressable.rs:33-38`).

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
- Roadmap and status: [docs/PLAN.md](docs/PLAN.md)
- VIA implementation notes: [docs/VIA.md](docs/VIA.md)
- VIC-20 reference links: [docs/REFERENCES.md](docs/REFERENCES.md)
- Missing rendering features: [WIP.md](WIP.md)
