# AGENTS.md

Guidance for AI coding agents working in this repository.

## Scope

- Project type: Rust VIC-20 emulator (work in progress).
- Make focused, incremental changes; avoid broad refactors unless explicitly requested.
- Prefer updating existing modules and tests over introducing new architecture.

## Fast Start

- Build: `cargo build`
- Test: `cargo test`
- Run emulator: `cargo run --bin vic20`
- Run disassembler: `cargo run --bin disassembler -- data/somefile.bin`

## Runtime Preconditions

- Emulator startup expects ROM files in `data/`:
  - `basic.901486-01.bin`
  - `characters.901460-03.bin`
  - `kernal.901486-07.bin`
- Missing ROM files can panic during boot due to `expect(...)` calls in bus initialization.

## Architecture Landmarks

- `src/bus.rs`: 64KB address space routing, ROM mapping, device stepping.
- `src/cpu/`: 6502 execution core (decode, execute, interrupts, registers).
- `src/vic.rs` and `src/screen/`: video logic and display path.
- `src/via2.rs`: VIA timer/interrupt behavior (still WIP).
- `src/bin/vic20.rs`: app entrypoint and threading model.

## Platform and Threading Constraints

- Keep `winit` event loop on the main thread (especially important on macOS).
- CPU/bus stepping may run on a worker thread; share framebuffer state via synchronization primitives.
- Favor short lock hold times around shared frame state updates.

## Known Pitfalls

- Avoid self-referential lifetime designs around CPU execution helpers; construct short-lived executors per step when needed.
- In bus/device stepping, avoid aliasing mutable borrows of a field and `&mut self` in the same call path.
- For `unimock` with trait objects, local `Debug` impls for trait objects may be needed for test expectations.

## Testing Conventions

- Keep tests close to code in inline `#[cfg(test)]` modules.
- Use `rstest` for fixtures/parameterized tests.
- Prefer assertions on CPU/register/PC state transitions over callback-style test hooks.

## File and Change Hygiene

- Keep formatting aligned with `rustfmt.toml`.
- Do not edit `target/` artifacts.
- Preserve existing public APIs unless the task explicitly requires API changes.

## Canonical References

- Project overview and quick commands: [README.md](README.md)
- Roadmap and implementation status: [docs/PLAN.md](docs/PLAN.md)
- VIA implementation notes: [docs/VIA.md](docs/VIA.md)
- Source/reference links: [docs/REFERENCES.md](docs/REFERENCES.md)
- Current short-term WIP note: [WIP.md](WIP.md)
