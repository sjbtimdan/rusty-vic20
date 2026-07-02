//! # nmos6502 — Cycle-perfect NMOS 6502 CPU Emulator
//!
//! A cycle-accurate emulator of the NMOS 6502 microprocessor, supporting all
//! 151 documented opcodes plus undocumented (illegal) opcodes. Execution is
//! driven by declarative micro-op sequences in `sequences.rs` — every CPU
//! cycle executes one micro-op, matching the real 6502's bus behaviour cycle
//! for cycle.
//!
//! ## Quick Start
//!
//! ```rust
//! use nmos6502::{CPU6502, memory::Ram, Addressable};
//!
//! let mut cpu = CPU6502::default();
//! let mut mem = Ram::new();
//!
//! // Set the program counter, initialise memory, then step cycle by cycle.
//! cpu.registers.pc = 0x0600;
//! cpu.cycle(&mut mem);          // one micro-op per call
//! ```
//!
//! The CPU interacts with memory exclusively through the
//! [`Addressable`] trait, making it easy to plug into any bus or memory
//! implementation (e.g. a full VIC-20 emulator).
//!
//! ## Execution Model
//!
//! - [`CPU6502::cycle()`] executes **one micro-op**, not one instruction.
//!   Instructions take 2–8 cycles depending on opcode, addressing mode,
//!   page-cross penalties, and branch-taken.
//! - [`CPU6502::is_at_instruction_boundary()`] returns `true` at a stable
//!   PC — useful for detecting instruction boundaries.
//! - [`CPU6502::reset()`] clears all registers and loads the reset vector
//!   from `$FFFC`–`$FFFD`.
//!
//! ## Bus Interface
//!
//! The [`Addressable`] trait is the only coupling between the CPU and memory:
//!
//! ```rust
//! # use nmos6502::Addressable;
//! fn read(addr: u16, bus: &mut impl Addressable) -> u8 {
//!     bus.read_byte(addr)
//! }
//! ```
//!
//! A testing [`Ram`](memory::Ram) type is provided (64 KB, zero-initialised).
//!
//! ## Test Suites
//!
//! The emulator passes:
//!
//! - **Tom Harte 6502 JSON tests** — 2.56 million parameterised cases across
//!   256 opcode files, with full bus-cycle trace validation.
//! - **Klaus Dormann functional, decimal, and interrupt tests** — assembled
//!   from bundled sources using the built-in assembler.
//!
//! ## Feature: Built-in Assembler & Disassembler
//!
//! The `tools` module provides a two-pass AS65-syntax assembler and a
//! disassembler, usable both as a library and via the `disassembler` binary
//! (`cargo run -p nmos6502 --bin disassembler -- <file> [base] [start]`).
pub mod alu;
pub mod breakpoint;
pub mod cpu;
pub mod edge_latch;
pub mod memory;
pub mod micro_op;
pub mod opcode;
pub mod registers;
pub mod sequences;
pub mod tools;

pub use breakpoint::Breakpoint;
pub use cpu::CPU6502;
pub use edge_latch::EdgeLatch;
pub use memory::Addressable;
pub use micro_op::MicroOp;
pub use registers::Registers;
