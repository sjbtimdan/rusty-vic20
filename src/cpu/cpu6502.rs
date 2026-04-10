use crate::cpu::{
    addressing_mode::AddressingMode,
    instructions::{InstructionInfo, decode},
    interrupt_handler::{DefaultInterruptHandler, InterruptHandler},
    registers::Registers,
};

pub struct CPU6502 {
    pub registers: Registers,
    current_instruction_info: Option<InstructionInfo>,
    cycle_count: u8,
    operands_index: usize,
    operands_buffer: [u8; 2],
    interrupt_handler: Box<dyn InterruptHandler>,
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
            interrupt_handler: Box::new(DefaultInterruptHandler),
        }
    }
}

impl CPU6502 {
    pub fn cycle(
        &mut self,
        memory: &mut [u8; 65536],
        mut execute_instruction: impl FnMut(&mut Registers, &mut [u8; 65536], &AddressingMode, &[u8], &dyn InterruptHandler),
    ) {
        self.cycle_count += 1;
        if self.current_instruction_info.is_none() {
            let opcode = memory[self.registers.pc as usize];
            self.current_instruction_info = Some(decode(opcode));
            self.operands_index = 0;
        } else {
            let Some(instruction_info) = &self.current_instruction_info else {
                panic!("Expected current_instruction_info to be Some");
            };
            if self.operands_index < instruction_info.mode.operand_count() {
                self.operands_buffer[self.operands_index] =
                    memory[(self.registers.pc + 1 + self.operands_index as u16) as usize];
                self.operands_index += 1;
            }
            if self.cycle_count == instruction_info.cycles {
                execute_instruction(
                    &mut self.registers,
                    memory,
                    &instruction_info.mode,
                    &self.operands_buffer,
                    self.interrupt_handler.as_ref(),
                );
                self.registers
                    .update_pc(self.registers.pc + 1 + instruction_info.mode.operand_count() as u16);
                self.current_instruction_info = None;
                self.cycle_count = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    fn memory() -> [u8; 65536] {
        [0; 65536]
    }

    #[rstest]
    fn test_inx_executes_after_two_cycles(mut memory: [u8; 65536]) {
        let mut cpu = CPU6502::default();
        cpu.registers.pc = 0x8000;
        memory[0x8000] = 0xE8; // INX opcode

        cpu.cycle(&mut memory, |_, _, _, _, _| {
            panic!("Instruction should not execute on first cycle");
        });
        let mut called = false;
        cpu.cycle(&mut memory, |_, _, _, _, _| {
            called = true;
        });
        assert!(called, "INX should execute on second cycle");
        assert_eq!(0x8001, cpu.registers.pc, "Program counter should advance by 1");
    }

    #[rstest]
    fn test_lda_immediate_executes_after_two_cycles(mut memory: [u8; 65536]) {
        let mut cpu = CPU6502::default();
        cpu.registers.pc = 0x8000;
        memory[0x8000] = 0xA9; // LDA immediate opcode
        memory[0x8001] = 0x20; // LDA immediate operand

        cpu.cycle(&mut memory, |_, _, _, _, _| {
            panic!("Instruction should not execute on first cycle");
        });
        let mut called = false;
        cpu.cycle(&mut memory, |_, _, _, operand, _| {
            called = true;
            assert_eq!(operand, &[0x20, 0x00], "Operand should be passed to executor");
        });
        assert!(called, "LDA should execute on third cycle");
        assert_eq!(0x8002, cpu.registers.pc, "Program counter should advance by 2");
    }
}
