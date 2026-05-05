use crate::{
    addressable::Addressable,
    cpu::{
        instruction_executor::stack_push,
        instructions::InstructionInfo,
        interrupt_handler::{Interrupt, InterruptHandler},
        registers::{BREAK_FLAG_BITMASK, INTERRUPT_FLAG_BITMASK, Registers, UNUSED_FLAG_BITMASK},
    },
};
use log::debug;

#[derive(Clone, Copy, Default)]
pub struct InstructionTracking {
    pub current_instruction_info: Option<InstructionInfo>,
    pub interrupt_requested: Option<Interrupt>,
}
impl InterruptHandler for InstructionTracking {
    fn handle_interrupt(&mut self, registers: &mut Registers, memory: &mut dyn Addressable, interrupt: Interrupt) {
        if interrupt == Interrupt::IRQ && registers.is_flag_set(INTERRUPT_FLAG_BITMASK) {
            return;
        }
        if interrupt == Interrupt::BRK || self.current_instruction_info.is_none() {
            self.current_instruction_info = None;
            self.do_interrupt(registers, memory, interrupt);
        } else {
            debug!(
                "Interrupt requested during instruction execution at PC=0x{:04X}, will be handled after current instruction completes",
                registers.pc
            );
            self.interrupt_requested = Some(interrupt);
        }
    }
}

impl InstructionTracking {
    pub fn do_interrupt(&mut self, registers: &mut Registers, memory: &mut dyn Addressable, interrupt: Interrupt) {
        let is_break = interrupt == Interrupt::BRK;
        debug!("Handling interrupt (is_break={is_break}) at PC=0x{:04X}", registers.pc);
        let return_address = if is_break {
            registers.pc.wrapping_add(2)
        } else {
            registers.pc
        };
        let status_to_push = if is_break {
            registers.status | UNUSED_FLAG_BITMASK | BREAK_FLAG_BITMASK
        } else {
            (registers.status | UNUSED_FLAG_BITMASK) & !BREAK_FLAG_BITMASK
        };
        stack_push(registers, memory, (return_address >> 8) as u8);
        stack_push(registers, memory, return_address as u8);
        stack_push(registers, memory, status_to_push);
        let interrupt_vector = match interrupt {
            Interrupt::NMI => 0xFFFAu16,
            Interrupt::IRQ | Interrupt::BRK => 0xFFFEu16,
        };
        registers.pc = memory.read_word(interrupt_vector);
        registers.set_flag(INTERRUPT_FLAG_BITMASK, true);
        registers.set_flag(BREAK_FLAG_BITMASK, is_break);
        self.interrupt_requested = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::instructions::LDA_IMMEDIATE;
    use rstest::{fixture, rstest};

    #[fixture]
    fn memory() -> [u8; 65536] {
        [0; 65536]
    }

    #[rstest]
    fn test_nmi_ignores_interrupt_flag(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.set_flag(INTERRUPT_FLAG_BITMASK, true);
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFA] = 0x34;
        memory[0xFFFB] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::NMI);

        assert_eq!(registers.pc, 0x1234, "NMI should be processed despite I flag being set");
    }

    #[rstest]
    fn test_nmi_uses_vector_fffa(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0xA000;
        registers.sp = 0xFF;
        memory[0xFFFA] = 0xCD;
        memory[0xFFFB] = 0xAB;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::NMI);

        assert_eq!(registers.pc, 0xABCD, "NMI should jump to address stored at 0xFFFA");
    }

    #[rstest]
    fn test_nmi_pushes_return_address_equal_to_pc(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFA] = 0x00;
        memory[0xFFFB] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::NMI);

        assert_eq!(
            memory[0x01FF], 0x80,
            "NMI should push high byte of PC as return address"
        );
        assert_eq!(memory[0x01FE], 0x00, "NMI should push low byte of PC as return address");
        assert_eq!(registers.sp, 0xFC, "SP should be decremented by 3");
    }

    #[rstest]
    fn test_nmi_does_not_set_break_flag_in_pushed_status(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFA] = 0x00;
        memory[0xFFFB] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::NMI);

        let pushed_status = memory[0x01FD];
        assert_eq!(
            pushed_status & BREAK_FLAG_BITMASK,
            0,
            "NMI should not set BREAK flag in pushed status"
        );
    }

    #[rstest]
    fn test_nmi_is_deferred_mid_instruction(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        tracking.current_instruction_info = Some(LDA_IMMEDIATE);

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::NMI);

        assert_eq!(registers.pc, 0x8000, "PC should not change when NMI is deferred");
        assert!(
            tracking.interrupt_requested == Some(Interrupt::NMI),
            "NMI should be queued as interrupt_requested"
        );
    }

    #[rstest]
    fn test_nmi_sets_interrupt_flag_after_handling(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFA] = 0x00;
        memory[0xFFFB] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::NMI);

        assert!(
            registers.is_flag_set(INTERRUPT_FLAG_BITMASK),
            "NMI should set interrupt flag after handling"
        );
    }

    #[rstest]
    fn test_brk_pushes_pc_plus_2_as_return_address(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0x34;
        memory[0xFFFF] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::BRK);

        assert_eq!(
            memory[0x01FF], 0x80,
            "BRK should push high byte of PC+2 as return address"
        );
        assert_eq!(
            memory[0x01FE], 0x02,
            "BRK should push low byte of PC+2 as return address"
        );
    }

    #[rstest]
    fn test_brk_sets_break_flag_in_pushed_status(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0x00;
        memory[0xFFFF] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::BRK);

        let pushed_status = memory[0x01FD];
        assert!(
            pushed_status & BREAK_FLAG_BITMASK != 0,
            "BRK should set BREAK flag in pushed status"
        );
    }

    #[rstest]
    fn test_brk_uses_vector_fffe(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0xCD;
        memory[0xFFFF] = 0xAB;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::BRK);

        assert_eq!(registers.pc, 0xABCD, "BRK should jump to address stored at 0xFFFE");
    }

    #[rstest]
    fn test_brk_works_when_interrupt_flag_set(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.set_flag(INTERRUPT_FLAG_BITMASK, true);
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0x34;
        memory[0xFFFF] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::BRK);

        assert_eq!(registers.pc, 0x1234, "BRK should be processed despite I flag being set");
    }

    #[rstest]
    fn test_brk_sets_interrupt_flag_after_handling(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0x00;
        memory[0xFFFF] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::BRK);

        assert!(
            registers.is_flag_set(INTERRUPT_FLAG_BITMASK),
            "BRK should set interrupt flag after handling"
        );
    }

    #[rstest]
    fn test_irq_is_masked_by_interrupt_flag(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.set_flag(INTERRUPT_FLAG_BITMASK, true);
        registers.pc = 0x8000;
        registers.sp = 0xFF;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::IRQ);

        assert_eq!(registers.pc, 0x8000, "IRQ should be ignored when I flag is set");
        assert_eq!(registers.sp, 0xFF, "SP should not change when IRQ is masked");
    }

    #[rstest]
    fn test_irq_uses_vector_fffe(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0xCD;
        memory[0xFFFF] = 0xAB;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::IRQ);

        assert_eq!(registers.pc, 0xABCD, "IRQ should jump to address stored at 0xFFFE");
    }

    #[rstest]
    fn test_irq_pushes_pc_as_return_address(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0x00;
        memory[0xFFFF] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::IRQ);

        assert_eq!(
            memory[0x01FF], 0x80,
            "IRQ should push high byte of PC as return address"
        );
        assert_eq!(memory[0x01FE], 0x00, "IRQ should push low byte of PC as return address");
    }

    #[rstest]
    fn test_irq_does_not_set_break_flag_in_pushed_status(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0x00;
        memory[0xFFFF] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::IRQ);

        let pushed_status = memory[0x01FD];
        assert_eq!(
            pushed_status & BREAK_FLAG_BITMASK,
            0,
            "IRQ should not set BREAK flag in pushed status"
        );
    }

    #[rstest]
    fn test_irq_sets_interrupt_flag_after_handling(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        registers.sp = 0xFF;
        memory[0xFFFE] = 0x00;
        memory[0xFFFF] = 0x12;

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::IRQ);

        assert!(
            registers.is_flag_set(INTERRUPT_FLAG_BITMASK),
            "IRQ should set interrupt flag after handling"
        );
    }

    #[rstest]
    fn test_irq_is_deferred_mid_instruction(mut memory: [u8; 65536]) {
        let mut registers = Registers::default();
        let mut tracking = InstructionTracking::default();
        registers.pc = 0x8000;
        tracking.current_instruction_info = Some(LDA_IMMEDIATE);

        tracking.handle_interrupt(&mut registers, &mut memory, Interrupt::IRQ);

        assert_eq!(registers.pc, 0x8000, "PC should not change when IRQ is deferred");
        assert!(
            tracking.interrupt_requested == Some(Interrupt::IRQ),
            "IRQ should be queued as interrupt_requested"
        );
    }
}
