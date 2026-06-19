use crate::{
    cpu::{
        addressing_mode::OperandResolution,
        instruction_executor::execute_instruction,
        instruction_tracking::InstructionTracking,
        instructions::{Instruction, InstructionInfo, decode},
        interrupt_handler::Interrupt,
        registers::{DECIMAL_FLAG_BITMASK, INTERRUPT_FLAG_BITMASK, Registers},
    },
    hardware::{addressable::Addressable, edge_latch::EdgeLatch},
    tools::{
        debug::{Breakpoint, LoggingAddressBreakpoint},
        disassembler::disassemble_instruction,
    },
};
use log::{debug, log_enabled};
use std::{
    hint::{likely, unlikely},
    time::Instant,
};

const PERFORMANCE_LOG_INTERVAL_CYCLES: u64 = 1_000_000;

pub struct CPU6502 {
    pub registers: Registers,
    cycle_count_at_end_of_this_instruction: u8,
    cycle_count: u8,
    operands_index: usize,
    operands_buffer: [u8; 2],
    total_cycles: u64,
    last_performance_log_cycle: u64,
    last_performance_log_instant: Instant,
    breakpoints: Vec<Box<dyn Breakpoint>>,
    pub instruction_tracking: InstructionTracking,
    pub irq_line_low: bool,
    pub nmi_latch: EdgeLatch,
}

impl Default for CPU6502 {
    fn default() -> Self {
        let registers = Registers::default();
        Self {
            registers,
            cycle_count: 0,
            cycle_count_at_end_of_this_instruction: 0,
            operands_index: 0,
            operands_buffer: [0; 2],
            total_cycles: 0,
            last_performance_log_cycle: 0,
            last_performance_log_instant: Instant::now(),
            breakpoints: vec![],
            instruction_tracking: InstructionTracking::default(),
            irq_line_low: false,
            nmi_latch: EdgeLatch::new_falling(),
        }
    }
}

impl CPU6502 {
    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    pub fn reset(&mut self, reset_vector: u16) {
        let registers = &mut self.registers;
        registers.set_flag(DECIMAL_FLAG_BITMASK, false);
        registers.set_flag(INTERRUPT_FLAG_BITMASK, false);
        registers.sp = 0xFD;
        registers.pc = reset_vector;
        self.cycle_count = 0;
        self.cycle_count_at_end_of_this_instruction = 0;
        self.operands_index = 0;
        self.instruction_tracking = InstructionTracking::default();
        self.irq_line_low = false;
        self.nmi_latch.reset();
        self.nmi_latch.set_level(true); // NMI line idles HIGH
    }

    pub fn add_breakpoint_address(&mut self, address: u16) {
        self.add_breakpoint(Box::new(LoggingAddressBreakpoint::new(address)));
    }

    pub fn add_breakpoint(&mut self, breakpoint: Box<dyn Breakpoint>) {
        self.breakpoints.push(breakpoint);
    }

    pub fn step(&mut self, memory: &mut impl Addressable) {
        if self.instruction_tracking.current_instruction_info.is_none() {
            if unlikely(self.nmi_latch.take()) {
                self.instruction_tracking
                    .do_interrupt(&mut self.registers, memory, Interrupt::NMI);
                return;
            }
            let deferred = self.instruction_tracking.interrupt_requested.take();
            if unlikely(deferred.is_some()) {
                self.instruction_tracking
                    .do_interrupt(&mut self.registers, memory, deferred.unwrap());
                return;
            }
            if unlikely(self.irq_line_low && !self.registers.is_flag_set(INTERRUPT_FLAG_BITMASK)) {
                self.instruction_tracking
                    .do_interrupt(&mut self.registers, memory, Interrupt::IRQ);
                return;
            }
        }
        self.total_cycles += 1;
        if log_enabled!(log::Level::Debug)
            && self.total_cycles - self.last_performance_log_cycle >= PERFORMANCE_LOG_INTERVAL_CYCLES
        {
            let elapsed = self.last_performance_log_instant.elapsed();
            debug!(
                "Executed {} cycles in {:.3} ms",
                PERFORMANCE_LOG_INTERVAL_CYCLES,
                elapsed.as_secs_f64() * 1_000.0
            );
            self.last_performance_log_cycle = self.total_cycles;
            self.last_performance_log_instant = Instant::now();
        }
        self.cycle_count += 1;
        if self.instruction_tracking.current_instruction_info.is_none() {
            let opcode = memory.read_byte(self.registers.pc);
            let current_instruction_info = decode(opcode);
            self.instruction_tracking.current_instruction_info = Some(current_instruction_info);
            self.operands_index = 0;
            self.cycle_count_at_end_of_this_instruction = self.cycle_count + current_instruction_info.cycles - 1;
        } else {
            let Some(instruction_info) = self.instruction_tracking.current_instruction_info else {
                panic!("Expected current_instruction_info to be Some");
            };
            if likely(self.operands_index < instruction_info.mode.operand_count()) {
                self.operands_buffer[self.operands_index] =
                    memory.read_byte(self.registers.pc + 1 + self.operands_index as u16);
                self.operands_index += 1;
                if unlikely(
                    self.operands_index == instruction_info.mode.operand_count()
                        && instruction_info
                            .instruction
                            .has_page_cross_cycle_penalty(&instruction_info.mode)
                        && instruction_info
                            .mode
                            .crosses_page_boundary(&self.registers, memory, &self.operands_buffer),
                ) {
                    self.cycle_count_at_end_of_this_instruction += 1;
                }
            }
            if self.cycle_count == self.cycle_count_at_end_of_this_instruction {
                self.breakpoints.iter().for_each(|bp| bp.on_hit(self.registers.pc));
                let debug_log = if log_enabled!(log::Level::Info) {
                    Some(line_debug_log(
                        self.total_cycles,
                        &instruction_info,
                        &self.operands_buffer,
                        &self.registers,
                    ))
                } else {
                    None
                };
                let pc_before = self.registers.pc;
                let expected_next_pc = pc_before.wrapping_add(1 + instruction_info.mode.operand_count() as u16);
                let increment_pc = execute_instruction(
                    &mut self.registers,
                    memory,
                    instruction_info.instruction,
                    &instruction_info.mode,
                    &self.operands_buffer,
                    &mut self.instruction_tracking,
                );
                if increment_pc {
                    self.registers
                        .update_pc(self.registers.pc + 1 + instruction_info.mode.operand_count() as u16);
                }
                log_instruction_result(
                    debug_log,
                    &instruction_info.instruction,
                    pc_before,
                    self.registers.pc,
                    expected_next_pc,
                );
                self.instruction_tracking.current_instruction_info = None;
                self.cycle_count = 0;

                if unlikely(self.nmi_latch.take()) {
                    self.instruction_tracking
                        .do_interrupt(&mut self.registers, memory, Interrupt::NMI);
                    return;
                }
                let deferred = self.instruction_tracking.interrupt_requested.take();
                if unlikely(deferred.is_some()) {
                    self.instruction_tracking
                        .do_interrupt(&mut self.registers, memory, deferred.unwrap());
                    return;
                }
                if unlikely(self.irq_line_low && !self.registers.is_flag_set(INTERRUPT_FLAG_BITMASK)) {
                    self.instruction_tracking
                        .do_interrupt(&mut self.registers, memory, Interrupt::IRQ);
                }
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
    instruction: &Instruction,
    pc_before: u16,
    actual_pc: u16,
    expected_next_pc: u16,
) {
    if let Some(debug_log) = debug_log {
        let branch_taken = instruction.is_branch() && actual_pc != expected_next_pc;
        let branch_marker = if branch_taken { " (*)" } else { "" };
        debug!("@0x{:04X}: {}{}", pc_before, debug_log, branch_marker);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::cpu::instructions::{INX_IMPLIED, LDA_ABSOLUTE_X, LDA_IMMEDIATE, NOP_IMPLIED};
    use rstest::{fixture, rstest};

    #[fixture]
    fn memory() -> crate::hardware::memory::Memory {
        crate::hardware::memory::Memory::default()
    }

    #[fixture]
    fn cpu() -> CPU6502 {
        CPU6502::default()
    }

    #[rstest]
    fn test_inx_executes_after_two_steps(mut memory: crate::hardware::memory::Memory, mut cpu: CPU6502) {
        cpu.registers.pc = 0x8000;
        memory.data[0x8000] = INX_IMPLIED.opcode;

        cpu.step(&mut memory);
        assert_eq!(cpu.registers.x, 0x00, "INX should not execute on first cycle");
        assert_eq!(cpu.registers.pc, 0x8000, "PC should not advance on first cycle");

        cpu.step(&mut memory);
        assert_eq!(cpu.registers.x, 0x01, "INX should execute on second cycle");
        assert_eq!(cpu.registers.pc, 0x8001, "Program counter should advance by 1");
    }

    #[rstest]
    fn test_lda_immediate_executes_after_two_cycles(mut memory: crate::hardware::memory::Memory, mut cpu: CPU6502) {
        cpu.registers.pc = 0x8000;
        memory.data[0x8000] = LDA_IMMEDIATE.opcode;
        memory.data[0x8001] = 0x20; // LDA immediate operand

        cpu.step(&mut memory);
        assert_eq!(cpu.registers.a, 0x00, "LDA should not execute on first cycle");
        assert_eq!(cpu.registers.pc, 0x8000, "PC should not advance on first cycle");

        cpu.step(&mut memory);
        assert_eq!(cpu.registers.a, 0x20, "LDA immediate should load operand");
        assert_eq!(cpu.registers.pc, 0x8002, "Program counter should advance by 2");
    }

    #[rstest]
    fn test_lda_absolute_x_executes_after_four_cycles_without_page_crossing(
        mut memory: crate::hardware::memory::Memory,
        mut cpu: CPU6502,
    ) {
        cpu.registers.pc = 0x8000;
        cpu.registers.x = 0x01;
        memory.data[0x8000] = LDA_ABSOLUTE_X.opcode;
        memory.data[0x8001] = 0x10; // low byte
        memory.data[0x8002] = 0x20; // high byte
        memory.data[0x2011] = 0x42; // target value

        for cycle in 1..4 {
            cpu.step(&mut memory);
            assert_eq!(
                cpu.registers.a, 0x00,
                "LDA absolute,X should not execute on cycle {cycle}"
            );
            assert_eq!(cpu.registers.pc, 0x8000, "PC should not advance before execution");
        }

        cpu.step(&mut memory);
        assert_eq!(cpu.registers.a, 0x42, "LDA absolute,X should load on cycle 4");
        assert_eq!(cpu.registers.pc, 0x8003, "Program counter should advance by 3");
    }

    #[rstest]
    fn test_lda_absolute_x_executes_after_five_cycles_when_crossing_page_boundary(
        mut memory: crate::hardware::memory::Memory,
        mut cpu: CPU6502,
    ) {
        cpu.registers.pc = 0x8000;
        cpu.registers.x = 0x01;
        memory.data[0x8000] = LDA_ABSOLUTE_X.opcode;
        memory.data[0x8001] = 0xFF; // low byte
        memory.data[0x8002] = 0x20; // high byte
        memory.data[0x2100] = 0x99; // target value after page crossing

        for cycle in 1..5 {
            cpu.step(&mut memory);
            assert_eq!(
                cpu.registers.a, 0x00,
                "LDA absolute,X should not execute on cycle {cycle}"
            );
            assert_eq!(cpu.registers.pc, 0x8000, "PC should not advance before execution");
        }

        cpu.step(&mut memory);
        assert_eq!(
            cpu.registers.a, 0x99,
            "LDA absolute,X should load on cycle 5 when crossing a page"
        );
        assert_eq!(cpu.registers.pc, 0x8003, "Program counter should advance by 3");
    }

    #[rstest]
    fn test_nmi_fires_on_falling_edge(mut memory: crate::hardware::memory::Memory, mut cpu: CPU6502) {
        cpu.registers.pc = 0x8000;
        cpu.registers.sp = 0xFF;
        memory.data[0x8000] = NOP_IMPLIED.opcode;
        memory.data[0xFFFA] = 0x34;
        memory.data[0xFFFB] = 0x12;

        cpu.nmi_latch.set_level(true);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x8000, "PC unchanged when NMI line stays HIGH");

        cpu.nmi_latch.set_level(false);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x1234, "NMI should fire on falling edge");
    }

    #[rstest]
    fn test_nmi_does_not_refire_when_line_stays_low(mut memory: crate::hardware::memory::Memory, mut cpu: CPU6502) {
        cpu.registers.pc = 0x8000;
        cpu.registers.sp = 0xFF;
        memory.data[0x8000] = NOP_IMPLIED.opcode;
        memory.data[0xFFFA] = 0x34;
        memory.data[0xFFFB] = 0x12;
        memory.data[0x1234] = NOP_IMPLIED.opcode;

        cpu.nmi_latch.set_level(true);
        cpu.nmi_latch.set_level(false);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x1234, "First NMI should fire");

        cpu.nmi_latch.set_level(false);
        cpu.step(&mut memory);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x1235, "PC should advance normally, no second NMI");
    }

    #[rstest]
    fn test_nmi_fires_again_on_new_edge(mut memory: crate::hardware::memory::Memory, mut cpu: CPU6502) {
        cpu.registers.pc = 0x8000;
        cpu.registers.sp = 0xFF;
        memory.data[0x8000] = NOP_IMPLIED.opcode;
        memory.data[0xFFFA] = 0x34;
        memory.data[0xFFFB] = 0x12;
        memory.data[0x1234] = NOP_IMPLIED.opcode;

        cpu.nmi_latch.set_level(true);
        cpu.nmi_latch.set_level(false);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x1234, "First NMI fires");

        cpu.nmi_latch.set_level(true);
        cpu.nmi_latch.set_level(false);

        cpu.step(&mut memory);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x1234, "Second NMI should fire on new edge");
    }

    #[rstest]
    fn test_nmi_latches_mid_instruction_and_fires_after(mut memory: crate::hardware::memory::Memory, mut cpu: CPU6502) {
        cpu.registers.pc = 0x8000;
        cpu.registers.sp = 0xFF;
        memory.data[0x8000] = LDA_IMMEDIATE.opcode;
        memory.data[0x8001] = 0x42;
        memory.data[0xFFFA] = 0x34;
        memory.data[0xFFFB] = 0x12;

        cpu.nmi_latch.set_level(true);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x8000, "LDA not yet executed");

        cpu.nmi_latch.set_level(false);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x1234, "NMI should fire after instruction completes");
    }

    #[rstest]
    fn test_reset_clears_nmi_state(mut cpu: CPU6502) {
        cpu.nmi_latch.set_level(true);
        cpu.nmi_latch.set_level(false);
        assert!(cpu.nmi_latch.is_latched(), "latch should be set before reset");
        cpu.reset(0x8000);
        assert!(!cpu.nmi_latch.take(), "latch should be cleared on reset");
    }

    #[rstest]
    fn test_nmi_fires_when_only_restore_asserts(mut memory: crate::hardware::memory::Memory, mut cpu: CPU6502) {
        cpu.registers.pc = 0x8000;
        cpu.registers.sp = 0xFF;
        memory.data[0x8000] = NOP_IMPLIED.opcode;
        memory.data[0xFFFA] = 0x34;
        memory.data[0xFFFB] = 0x12;

        cpu.nmi_latch.set_level(true);
        cpu.nmi_latch.set_level(false);
        cpu.step(&mut memory);
        assert_eq!(cpu.registers.pc, 0x1234, "NMI should fire from RESTORE alone");
    }
}
