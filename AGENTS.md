# AGENTS.md

Guidance for AI coding agents working in this repository.

## Scope

- Project type: Rust VIC-20 emulator (work in progress).
- Make focused, incremental changes; avoid broad refactors unless explicitly requested.
- Prefer updating existing modules and tests over introducing new architecture.

## Fast Start

- Build: `cargo build`
- Test: `cargo test`
- Format: `cargo fmt` (config in `rustfmt.toml`)
- Lint: `cargo clippy` — run after every change and fix all warnings
- Run emulator: `cargo run --bin vic20`
- Run disassembler: `cargo run --bin disassembler -- data/somefile.bin`
- Enable logging: `RUST_LOG=info cargo run --bin vic20` (uses `env_logger`)

## Compiler Requirements

- `edition = "2024"` in `Cargo.toml` requires Rust >= 1.85.

## Runtime Preconditions

- Emulator startup expects ROM files in `data/`:
  - `basic.901486-01.bin`
  - `characters.901460-03.bin`
  - `kernal.901486-07.bin`
- Missing ROM files can panic during boot due to `expect(...)` calls in bus initialization.
- Integration tests in `tests/smoke_test.rs` also require ROM files and may take several seconds (600K+ CPU steps).

## Architecture Landmarks

- `src/bus.rs`: 64KB address space routing, ROM mapping, device stepping.
- `src/cpu/`: 6502 execution core (decode, execute, interrupts, registers).
- `src/vic.rs` and `src/screen/`: video logic and display path (`renderer.rs`, `display.rs`).
- `src/via2.rs`: VIA timer/interrupt behavior (still WIP — only Timer 1 partially implemented).
- `src/debug/mod.rs` and `src/debug/display.rs`: debugger window showing registers, memory, and perf metrics.
- `src/tools/disassembler.rs`: 6502 disassembler used by the `disassembler` binary.
- `src/bin/vic20.rs`: thin CLI entrypoint, accepts optional tick duration in microseconds.
- `src/bin/disassembler.rs`: standalone disassembler binary.
- `src/controller.rs`: main `ApplicationHandler` managing both screen + keyboard windows, dispatches events by `WindowId`, owns emulator thread.
- `src/keyboard/mod.rs`: keyboard state machine (key regions, click/hold/flash, physical key mapping).
- `src/keyboard/display.rs`: interactive keyboard overlay window rendering.

## Platform and Threading Constraints

- Keep `winit` event loop on the main thread (especially important on macOS).
- CPU/bus stepping may run on a worker thread; share framebuffer and keyboard state via `Arc<Mutex<>>` wrappers around `SharedVideoState` and `KeyboardState`.
- Favor short lock hold times around shared frame state updates.

## Known Pitfalls

- Avoid self-referential lifetime designs around CPU execution helpers; construct short-lived executors per step when needed.
- In bus/device stepping, avoid aliasing mutable borrows of a field and `&mut self` in the same call path.
- For `unimock` with trait objects, local `Debug` impls for trait objects may be needed for test expectations.
- The VIC chip's reverse-mode, multi-color mode, and double-height characters are not yet implemented (see `WIP.md`).

## Testing Conventions

- Keep tests close to code in inline `#[cfg(test)]` modules.
- Use `rstest` for fixtures/parameterized tests.
- Use `unimock` for trait mocking in tests (see `instruction_executor`, `via2`, `disassembler`).
- Prefer assertions on CPU/register/PC state transitions over callback-style test hooks.
- Integration tests live in `tests/`; they exercise the full emulator and require ROM files.

## File and Change Hygiene

- Keep formatting aligned with `rustfmt.toml` (max width 120, no hard tabs, field init shorthand, reorder imports).
- Do not edit `target/` artifacts.
- Preserve existing public APIs unless the task explicitly requires API changes.

## Canonical References

- Project overview and quick commands: [README.md](README.md)
- Roadmap and implementation status: [docs/PLAN.md](docs/PLAN.md)
- VIA implementation notes: [docs/VIA.md](docs/VIA.md)
- Source/reference links: [docs/REFERENCES.md](docs/REFERENCES.md)
- Current short-term WIP note: [WIP.md](WIP.md)
