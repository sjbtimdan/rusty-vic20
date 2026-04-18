use crate::{
    addressable::Addressable,
    cpu::{
        instruction_executor::{DefaultInstructionExecutor, InstructionExecutor},
        instructions::{Instruction, InstructionInfo, decode},
        interrupt_handler::InterruptHandler,
        registers::{DECIMAL_FLAG_BITMASK, INTERRUPT_FLAG_BITMASK, Registers},
    },
    tools::{
        debug::{Breakpoint, LoggingAddressBreakpoint},
        disassembler::disassemble_instruction,
    },
};
use log::{info, log_enabled};
use std::time::Instant;

const PERFORMANCE_LOG_INTERVAL_CYCLES: u64 = 1_000_000;

pub struct CPU6502 {
    registers: Registers,
    current_instruction_info: Option<InstructionInfo>,
    cycle_count: u8,
    operands_index: usize,
    operands_buffer: [u8; 2],
    instruction_executor: Box<dyn InstructionExecutor>,
    total_cycles: u64,
    last_performance_log_cycle: u64,
    last_performance_log_instant: Instant,
    breakpoints: Vec<Box<dyn Breakpoint>>,
}

impl Default for CPU6502 {
    fn default() -> Self {
        let registers = Registers::default();
        Self {
            registers,
            cycle_count: 0,
            current_instruction_info: None,
            operands_index: 0,
            operands_buffer: [0; 2],
            instruction_executor: Box::new(DefaultInstructionExecutor),
            total_cycles: 0,
            last_performance_log_cycle: 0,
            last_performance_log_instant: Instant::now(),
            breakpoints: vec![],
        }
    }
}

impl CPU6502 {
    pub fn reset(&mut self, reset_vector: u16) {
        let registers = &mut self.registers;
        registers.set_flag(DECIMAL_FLAG_BITMASK, false);
        registers.set_flag(INTERRUPT_FLAG_BITMASK, true);
        registers.sp = 0xFD;
        registers.pc = reset_vector;
    }

    pub fn add_breakpoint_address(&mut self, address: u16) {
        self.add_breakpoint(Box::new(LoggingAddressBreakpoint::new(address)));
    }

    pub fn add_breakpoint(&mut self, breakpoint: Box<dyn Breakpoint>) {
        self.breakpoints.push(breakpoint);
    }

    pub fn step(&mut self, memory: &mut impl Addressable, interrupt_handler: &impl InterruptHandler) {
        self.total_cycles += 1;
        if self.total_cycles - self.last_performance_log_cycle >= PERFORMANCE_LOG_INTERVAL_CYCLES {
            let elapsed = self.last_performance_log_instant.elapsed();
            info!(
                "Executed {} cycles in {:.3} ms",
                PERFORMANCE_LOG_INTERVAL_CYCLES,
                elapsed.as_secs_f64() * 1_000.0
            );
            self.last_performance_log_cycle = self.total_cycles;
            self.last_performance_log_instant = Instant::now();
        }
        self.cycle_count += 1;
        if self.current_instruction_info.is_none() {
            let opcode = memory.read_byte(self.registers.pc);
            self.current_instruction_info = Some(decode(opcode));
            self.operands_index = 0;
        } else {
            let Some(instruction_info) = &self.current_instruction_info else {
                panic!("Expected current_instruction_info to be Some");
            };
            if self.operands_index < instruction_info.mode.operand_count() {
                self.operands_buffer[self.operands_index] =
                    memory.read_byte(self.registers.pc + 1 + self.operands_index as u16);
                self.operands_index += 1;
            }
            if self.cycle_count == instruction_info.cycles {
                self.breakpoints.iter().for_each(|bp| bp.on_hit(self.registers.pc));
                let debug_log = if log_enabled!(log::Level::Info) {
                    Some(line_debug_log(
                        self.total_cycles,
                        instruction_info,
                        &self.operands_buffer,
                        &self.registers,
                    ))
                } else {
                    None
                };
                let pc_before = self.registers.pc;
                let expected_next_pc = pc_before.wrapping_add(1 + instruction_info.mode.operand_count() as u16);
                let increment_pc = self.instruction_executor.execute_instruction(
                    &mut self.registers,
                    memory,
                    instruction_info.instruction,
                    &instruction_info.mode,
                    &self.operands_buffer,
                    interrupt_handler,
                );
                if increment_pc {
                    self.registers
                        .update_pc(self.registers.pc + 1 + instruction_info.mode.operand_count() as u16);
                }
                log_instruction_result(
                    debug_log,
                    instruction_info.instruction,
                    pc_before,
                    self.registers.pc,
                    expected_next_pc,
                );
                self.current_instruction_info = None;
                self.cycle_count = 0;
            }
        }
    }
}

fn line_debug_log(
    total_cycles: u64,
    instruction_info: &InstructionInfo,
    operands_buffer: &[u8; 2],
    registers: &Registers,
) -> String {
    let code = disassemble_instruction(instruction_info, operands_buffer, registers.pc, " ");
    format!(
        "[{}]: 0x{:04X}: {:<20} [{}]",
        total_cycles, registers.pc, code, registers
    )
}

fn log_instruction_result(
    debug_log: Option<String>,
    instruction: Instruction,
    pc_before: u16,
    actual_pc: u16,
    expected_next_pc: u16,
) {
    if let Some(debug_log) = debug_log {
        let branch_taken = instruction.is_branch() && actual_pc != expected_next_pc;
        let branch_marker = if branch_taken { " (*)" } else { "" };
        info!("@0x{:04X}: {}{}", pc_before, debug_log, branch_marker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::interrupt_handler::DefaultInterruptHandler;
    use rstest::{fixture, rstest};

    #[fixture]
    fn memory() -> [u8; 65536] {
        [0; 65536]
    }

    #[rstest]
    fn test_inx_executes_after_two_steps(mut memory: [u8; 65536]) {
        let mut cpu = CPU6502::default();
        let interrupt_handler = DefaultInterruptHandler;
        cpu.registers.pc = 0x8000;
        memory[0x8000] = 0xE8; // INX opcode

        cpu.step(&mut memory, &interrupt_handler);
        assert_eq!(cpu.registers.x, 0x00, "INX should not execute on first cycle");
        assert_eq!(cpu.registers.pc, 0x8000, "PC should not advance on first cycle");

        cpu.step(&mut memory, &interrupt_handler);
        assert_eq!(cpu.registers.x, 0x01, "INX should execute on second cycle");
        assert_eq!(cpu.registers.pc, 0x8001, "Program counter should advance by 1");
    }

    #[rstest]
    fn test_lda_immediate_executes_after_two_cycles(mut memory: [u8; 65536]) {
        let mut cpu = CPU6502::default();
        let interrupt_handler = DefaultInterruptHandler;
        cpu.registers.pc = 0x8000;
        memory[0x8000] = 0xA9; // LDA immediate opcode
        memory[0x8001] = 0x20; // LDA immediate operand

        cpu.step(&mut memory, &interrupt_handler);
        assert_eq!(cpu.registers.a, 0x00, "LDA should not execute on first cycle");
        assert_eq!(cpu.registers.pc, 0x8000, "PC should not advance on first cycle");

        cpu.step(&mut memory, &interrupt_handler);
        assert_eq!(cpu.registers.a, 0x20, "LDA immediate should load operand");
        assert_eq!(cpu.registers.pc, 0x8002, "Program counter should advance by 2");
    }
}
