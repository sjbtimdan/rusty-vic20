use crate::{
    alu,
    breakpoint::Breakpoint,
    edge_latch::EdgeLatch,
    memory::Addressable,
    micro_op::{BusOp, MicroOp},
    registers::Registers,
    sequences::{INTERRUPT_SEQ_IRQ, INTERRUPT_SEQ_NMI, INTERRUPT_SEQ_RESET, OPCODE_SEQUENCES},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interrupt {
    NMI,
    IRQ,
    BRK,
    Reset,
}

#[derive(Default)]
pub struct CPU6502 {
    pub registers: Registers,

    sequence: &'static [MicroOp],
    sequence_index: usize,

    operands: [u8; 2],
    addr: u16,
    data_latch: u8,

    branch_taken: bool,
    page_crossed: bool,

    pub irq_line_low: bool,
    pub nmi_latch: EdgeLatch,

    pub breakpoints: Vec<Box<dyn Breakpoint>>,

    pub halted: bool,

    pub total_cycles: u64,
}

fn page_cross(base: u16, index: u8) -> bool {
    let addr = base.wrapping_add(index as u16);
    (base & 0xFF00) != (addr & 0xFF00)
}

macro_rules! branch_op {
    ($name:ident, $cond:expr) => {
        pub fn $name(&mut self) {
            self.branch_if($cond);
        }
    };
}

impl CPU6502 {
    /// Returns `true` when the CPU has no pending micro-ops and is ready to
    /// fetch the next instruction. Useful for determining when a given value of
    /// `registers.pc` is a stable instruction boundary rather than an
    /// intermediate value during multi-cycle instruction execution.
    pub fn is_at_instruction_boundary(&self) -> bool {
        self.sequence_index >= self.sequence.len() || self.sequence.is_empty()
    }

    pub fn add_breakpoint(&mut self, bp: Box<dyn Breakpoint>) {
        self.breakpoints.push(bp);
    }

    pub fn reset(&mut self, _memory: &mut impl Addressable) {
        *self = Self::default();
        self.nmi_latch.set_level(true);
        self.enter_interrupt(Interrupt::Reset);
    }

    #[inline]
    pub fn cycle(&mut self, memory: &mut impl Addressable) {
        self.total_cycles += 1;
        if self.sequence_index < self.sequence.len() {
            let op = self.sequence[self.sequence_index];
            match op.bus {
                BusOp::ReadPC1 => {
                    let val = memory.read_byte(self.registers.pc);
                    self.operands[0] = val;
                    self.data_latch = val;
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                }
                BusOp::ReadPC2 => {
                    self.operands[1] = memory.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                }

                BusOp::ReadAddr => {
                    self.data_latch = memory.read_byte(self.addr);
                }
                BusOp::ReadDummyZpX => {
                    let zp = self.operands[0];
                    let _ = memory.read_zp_byte(zp);
                    self.addr = (zp.wrapping_add(self.registers.x)) as u16;
                }
                BusOp::ReadDummyZpY => {
                    let zp = self.operands[0];
                    let _ = memory.read_zp_byte(zp);
                    self.addr = (zp.wrapping_add(self.registers.y)) as u16;
                }
                BusOp::ReadAddrZp1 => {
                    let addr = (self.addr as u8).wrapping_add(1) as u16;
                    self.data_latch = memory.read_byte(addr);
                }
                BusOp::ReadDummyNext => {
                    let _ = memory.read_byte(self.registers.pc);
                }
                BusOp::ReadRTS => {
                    let _ = memory.read_byte(self.registers.pc.wrapping_sub(1));
                }
                BusOp::WriteAddrA => memory.write_byte(self.addr, self.registers.a),
                BusOp::WriteAddrX => memory.write_byte(self.addr, self.registers.x),
                BusOp::WriteAddrY => memory.write_byte(self.addr, self.registers.y),
                BusOp::WriteAddrAX => memory.write_byte(self.addr, self.registers.a & self.registers.x),
                BusOp::WriteAddrAHX => {
                    self.masked_write(memory, self.registers.a & self.registers.x);
                }
                BusOp::WriteAddrSHY => self.masked_write(memory, self.registers.y),
                BusOp::WriteAddrSHX => self.masked_write(memory, self.registers.x),
                BusOp::WriteDataLatch => memory.write_byte(self.addr, self.data_latch),
                BusOp::PushPCH => self.push_byte(memory, (self.registers.pc >> 8) as u8),
                BusOp::PushPCL => self.push_byte(memory, self.registers.pc as u8),
                BusOp::PushA => self.push_byte(memory, self.registers.a),
                BusOp::PushStatusB => {
                    self.push_byte(memory, self.registers.status | crate::registers::BREAK);
                }
                BusOp::PushStatus => {
                    self.push_byte(memory, self.registers.status);
                }
                BusOp::PopDummy => {
                    let _ = memory.read_byte(0x0100 + self.registers.sp as u16);
                }
                BusOp::Pop => self.data_latch = self.pop_byte(memory),
                BusOp::PopPCL => self.operands[0] = self.pop_byte(memory),
                BusOp::ReadVecLo(addr) => {
                    self.operands[0] = memory.read_byte(addr);
                }
                BusOp::ReadVecHi(addr) => {
                    let hi = memory.read_byte(addr);
                    self.registers.pc = (hi as u16) << 8 | self.operands[0] as u16;
                }
            }
            (op.internal)(self);
            self.sequence_index += 1;
        } else {
            if self.halted {
                return;
            }
            self.fetch_and_decode(memory);
        }
    }

    fn fetch_and_decode(&mut self, memory: &mut impl Addressable) {
        if self.nmi_latch.take() {
            self.enter_interrupt(Interrupt::NMI);
            return;
        }
        if self.irq_line_low && !self.registers.is_flag_set(crate::registers::INTERRUPT) {
            self.enter_interrupt(Interrupt::IRQ);
            return;
        }
        let opcode = memory.read_byte(self.registers.pc);
        if !self.breakpoints.is_empty() {
            for bp in &self.breakpoints {
                bp.on_hit(self.registers.pc);
            }
        }
        self.registers.pc = self.registers.pc.wrapping_add(1);
        self.sequence = OPCODE_SEQUENCES[opcode as usize];
        self.sequence_index = 0;
        self.reset_instruction_state();
    }

    // ── Internal operation implementations (called via function pointer) ──

    pub fn op_none(&mut self) {}

    // ── Address computation ──

    pub fn op_set_addr_zp(&mut self) {
        self.addr = self.operands[0] as u16;
    }
    pub fn op_set_addr_abs(&mut self) {
        self.addr = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
    }
    /// Add `index` to operand address, set page-wrapped `addr`, optionally skip on no-cross.
    fn set_addr_abs_indexed(&mut self, index: u8, skip_on_no_cross: bool) {
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        let full = base.wrapping_add(index as u16);
        self.page_crossed = (base & 0xFF00) != (full & 0xFF00);
        self.addr = (base & 0xFF00) | (full as u8 as u16);
        if skip_on_no_cross && !self.page_crossed {
            self.sequence_index += 1;
        }
    }
    pub fn op_set_addr_absx(&mut self) {
        self.set_addr_abs_indexed(self.registers.x, true);
    }
    pub fn op_set_addr_absy(&mut self) {
        self.set_addr_abs_indexed(self.registers.y, true);
    }
    /// Like abs,X but never skips (writes/RMW always need the dummy cycle).
    pub fn op_set_addr_absx_full(&mut self) {
        self.set_addr_abs_indexed(self.registers.x, false);
    }
    /// Like abs,Y but never skips (writes/RMW always need the dummy cycle).
    pub fn op_set_addr_absy_full(&mut self) {
        self.set_addr_abs_indexed(self.registers.y, false);
    }
    pub fn op_fix_addr_cross(&mut self) {
        if self.page_crossed {
            self.addr = self.addr.wrapping_add(0x100);
        }
    }
    /// Combine (indirect),Y pointer with Y, set page-wrapped `addr`.
    /// `save_base_hi` saves the page base high byte for masked writes (AHX).
    fn set_addr_indy_indexed(&mut self, skip_on_no_cross: bool, save_base_hi: bool) {
        let lo = self.operands[1] as u16;
        let hi = self.data_latch as u16;
        let ptr = (hi << 8) | lo;
        let full = ptr.wrapping_add(self.registers.y as u16);
        self.page_crossed = (ptr & 0xFF00) != (full & 0xFF00);
        self.addr = (ptr & 0xFF00) | (full as u8 as u16);
        if save_base_hi {
            self.operands[1] = hi as u8;
        }
        if skip_on_no_cross && !self.page_crossed {
            self.sequence_index += 1;
        }
    }
    pub fn op_compute_indy_addr(&mut self) {
        self.set_addr_indy_indexed(true, false);
    }
    pub fn op_compute_indy_addr_rmw(&mut self) {
        self.set_addr_indy_indexed(false, false);
    }
    pub fn op_compute_ahx_addr(&mut self) {
        self.set_addr_indy_indexed(false, true);
    }
    pub fn op_save_lo(&mut self) {
        self.operands[1] = self.data_latch;
    }
    pub fn op_compute_ind_addr(&mut self) {
        let lo = self.operands[1] as u16;
        let hi = self.data_latch as u16;
        self.addr = (hi << 8) | lo;
    }

    /// TAS/SHS (abs,Y): SP = A & X, page-wrapped addr, reuses WriteAddrAHX for the masked write.
    pub fn op_tas_setup_addr(&mut self) {
        self.registers.sp = self.registers.a & self.registers.x;
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.page_crossed = page_cross(base, self.registers.y);
        self.addr = (self.operands[1] as u16) << 8 | (self.operands[0].wrapping_add(self.registers.y)) as u16;
    }
    /// SHY (abs,X): page-wrapped addr, never skips (always 5 cycles).
    pub fn op_shy_setup_addr(&mut self) {
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.page_crossed = page_cross(base, self.registers.x);
        self.addr = (self.operands[1] as u16) << 8 | (self.operands[0].wrapping_add(self.registers.x)) as u16;
    }
    /// SHX (abs,Y): page-wrapped addr, never skips (always 5 cycles).
    pub fn op_shx_setup_addr(&mut self) {
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.page_crossed = page_cross(base, self.registers.y);
        self.addr = (self.operands[1] as u16) << 8 | (self.operands[0].wrapping_add(self.registers.y)) as u16;
    }

    // ── Register operations ──

    pub fn op_set_a(&mut self) {
        self.registers.set_accumulator(self.data_latch);
    }
    pub fn op_set_x(&mut self) {
        self.registers.set_x(self.data_latch);
    }
    pub fn op_set_y(&mut self) {
        self.registers.set_y(self.data_latch);
    }
    pub fn op_txa(&mut self) {
        self.registers.set_accumulator(self.registers.x);
    }
    pub fn op_tya(&mut self) {
        self.registers.set_accumulator(self.registers.y);
    }
    pub fn op_tax(&mut self) {
        self.registers.set_x(self.registers.a);
    }
    pub fn op_tay(&mut self) {
        self.registers.set_y(self.registers.a);
    }
    pub fn op_tsx(&mut self) {
        self.registers.set_x(self.registers.sp);
    }
    pub fn op_txs(&mut self) {
        self.registers.sp = self.registers.x;
    }
    pub fn op_inc_x(&mut self) {
        self.registers.set_x(self.registers.x.wrapping_add(1));
    }
    pub fn op_inc_y(&mut self) {
        self.registers.set_y(self.registers.y.wrapping_add(1));
    }
    pub fn op_dec_x(&mut self) {
        self.registers.set_x(self.registers.x.wrapping_sub(1));
    }
    pub fn op_dec_y(&mut self) {
        self.registers.set_y(self.registers.y.wrapping_sub(1));
    }

    // ── Flag operations ──

    pub fn op_set_c(&mut self) {
        self.registers.update_carry_flag(true);
    }
    pub fn op_clr_c(&mut self) {
        self.registers.update_carry_flag(false);
    }
    pub fn op_set_d(&mut self) {
        self.registers.update_decimal_flag(true);
    }
    pub fn op_clr_d(&mut self) {
        self.registers.update_decimal_flag(false);
    }
    pub fn op_set_i(&mut self) {
        self.registers.update_interrupt_flag(true);
    }
    pub fn op_clr_i(&mut self) {
        self.registers.update_interrupt_flag(false);
    }
    pub fn op_clr_v(&mut self) {
        self.registers.update_overflow_flag(false);
    }

    // ── ALU operations ──

    pub fn op_adc(&mut self) {
        alu::adc(&mut self.registers, self.data_latch);
    }
    pub fn op_sbc(&mut self) {
        alu::sbc(&mut self.registers, self.data_latch);
    }
    pub fn op_and(&mut self) {
        self.registers.set_accumulator(self.registers.a & self.data_latch);
    }
    pub fn op_ora(&mut self) {
        self.registers.set_accumulator(self.registers.a | self.data_latch);
    }
    pub fn op_eor(&mut self) {
        self.registers.set_accumulator(self.registers.a ^ self.data_latch);
    }
    pub fn op_cmp_a(&mut self) {
        let a = self.registers.a;
        alu::compare(&mut self.registers, a, self.data_latch);
    }
    pub fn op_cmp_x(&mut self) {
        let x = self.registers.x;
        alu::compare(&mut self.registers, x, self.data_latch);
    }
    pub fn op_cmp_y(&mut self) {
        let y = self.registers.y;
        alu::compare(&mut self.registers, y, self.data_latch);
    }
    pub fn op_bit(&mut self) {
        let v = self.data_latch;
        self.registers.update_zero_flag(self.registers.a & v == 0);
        self.registers.update_overflow_flag(v & 0x40 != 0);
        self.registers.update_negative_flag(v & 0x80 != 0);
    }

    // ── RMW memory operations ──

    pub fn op_asl(&mut self) {
        let c = self.data_latch & 0x80 != 0;
        self.data_latch <<= 1;
        self.registers.update_carry_flag(c);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub fn op_lsr(&mut self) {
        let c = self.data_latch & 0x01 != 0;
        self.data_latch >>= 1;
        self.registers.update_carry_flag(c);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub fn op_rol(&mut self) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.data_latch & 0x80 != 0;
        self.data_latch = (self.data_latch << 1) | old_c;
        self.registers.update_carry_flag(new_c);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub fn op_ror(&mut self) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.data_latch & 0x01 != 0;
        self.data_latch = (self.data_latch >> 1) | (old_c << 7);
        self.registers.update_carry_flag(new_c);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub fn op_inc(&mut self) {
        self.data_latch = self.data_latch.wrapping_add(1);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub fn op_dec(&mut self) {
        self.data_latch = self.data_latch.wrapping_sub(1);
        self.registers.update_zero_and_negative(self.data_latch);
    }

    // ── Accumulator shifts ──

    pub fn op_asl_a(&mut self) {
        let c = self.registers.a & 0x80 != 0;
        let result = self.registers.a << 1;
        self.registers.update_carry_flag(c);
        self.registers.set_accumulator(result);
    }
    pub fn op_lsr_a(&mut self) {
        let c = self.registers.a & 0x01 != 0;
        let result = self.registers.a >> 1;
        self.registers.update_carry_flag(c);
        self.registers.set_accumulator(result);
    }
    pub fn op_rol_a(&mut self) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.registers.a & 0x80 != 0;
        let result = (self.registers.a << 1) | old_c;
        self.registers.update_carry_flag(new_c);
        self.registers.set_accumulator(result);
    }
    pub fn op_ror_a(&mut self) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.registers.a & 0x01 != 0;
        let result = (self.registers.a >> 1) | (old_c << 7);
        self.registers.update_carry_flag(new_c);
        self.registers.set_accumulator(result);
    }

    // ── Undocumented load/store ──

    pub fn op_las(&mut self) {
        let val = self.registers.sp & self.data_latch;
        self.registers.set_accumulator(val);
        self.registers.set_x(val);
        self.registers.sp = val;
    }

    // ── Unofficial opcodes ──

    pub fn op_lax(&mut self) {
        self.registers.set_accumulator(self.data_latch);
        self.registers.set_x(self.data_latch);
    }
    pub fn op_lax_imm(&mut self) {
        let result = self.data_latch & (self.registers.a | 0xEE);
        self.registers.set_accumulator(result);
        self.registers.set_x(result);
    }
    pub fn op_slo(&mut self) {
        let c = self.data_latch & 0x80 != 0;
        self.data_latch <<= 1;
        self.registers.update_carry_flag(c);
        self.registers.update_zero_and_negative(self.data_latch);
        let a = self.registers.a | self.data_latch;
        self.registers.set_accumulator(a);
    }
    pub fn op_rla(&mut self) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.data_latch & 0x80 != 0;
        self.data_latch = (self.data_latch << 1) | old_c;
        self.registers.update_carry_flag(new_c);
        self.registers.update_zero_and_negative(self.data_latch);
        let a = self.registers.a & self.data_latch;
        self.registers.set_accumulator(a);
    }
    pub fn op_sre(&mut self) {
        let c = self.data_latch & 0x01 != 0;
        self.data_latch >>= 1;
        self.registers.update_carry_flag(c);
        self.registers.update_zero_and_negative(self.data_latch);
        let a = self.registers.a ^ self.data_latch;
        self.registers.set_accumulator(a);
    }
    pub fn op_rra(&mut self) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.data_latch & 0x01 != 0;
        self.data_latch = (self.data_latch >> 1) | (old_c << 7);
        self.registers.update_carry_flag(new_c);
        self.registers.update_zero_and_negative(self.data_latch);
        alu::adc(&mut self.registers, self.data_latch);
    }
    pub fn op_dcp(&mut self) {
        self.data_latch = self.data_latch.wrapping_sub(1);
        self.registers.update_zero_and_negative(self.data_latch);
        let a = self.registers.a;
        alu::compare(&mut self.registers, a, self.data_latch);
    }
    pub fn op_isc(&mut self) {
        self.data_latch = self.data_latch.wrapping_add(1);
        self.registers.update_zero_and_negative(self.data_latch);
        alu::sbc(&mut self.registers, self.data_latch);
    }
    pub fn op_anc(&mut self) {
        self.registers.set_accumulator(self.registers.a & self.data_latch);
        self.registers
            .update_carry_flag(self.registers.is_flag_set(crate::registers::NEGATIVE));
    }
    pub fn op_alr(&mut self) {
        let result = self.registers.a & self.data_latch;
        self.registers.update_carry_flag(result & 1 != 0);
        let shifted = result >> 1;
        self.registers.a = shifted;
        self.registers.update_zero_and_negative(shifted);
    }
    pub fn op_arr(&mut self) {
        let and = self.registers.a & self.data_latch;
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;

        if self.registers.is_flag_set(crate::registers::DECIMAL) {
            let ah = and >> 4;
            let al = and & 0x0F;

            let result = (and >> 1) | (old_c << 7);

            self.registers.update_negative_flag(old_c != 0);
            self.registers.update_zero_flag(result == 0);
            self.registers.update_overflow_flag((and ^ result) & 0x40 != 0);

            let mut a = result;

            if al + (al & 1) > 5 {
                a = (a & 0xF0) | ((a.wrapping_add(6)) & 0x0F);
            }

            let carry_set = ah + (ah & 1) > 5;
            self.registers.update_carry_flag(carry_set);
            if carry_set {
                a = a.wrapping_add(0x60);
            }

            self.registers.a = a;
        } else {
            let result = (and >> 1) | (old_c << 7);
            self.registers.a = result;
            self.registers.update_carry_flag(result & 0x40 != 0);
            self.registers.update_zero_and_negative(result);
            self.registers
                .update_overflow_flag(((result >> 6) ^ (result >> 5)) & 1 != 0);
        }
    }
    pub fn op_xaa(&mut self) {
        let val = (self.registers.a | 0xEE) & self.registers.x & self.data_latch;
        self.registers.set_accumulator(val);
    }
    pub fn op_sbx(&mut self) {
        let ax = self.registers.a & self.registers.x;
        let result = ax.wrapping_sub(self.data_latch);
        self.registers.update_carry_flag(ax >= self.data_latch);
        self.registers.update_zero_and_negative(result);
        self.registers.set_x(result);
    }

    /// Shared write helper for AHX/SHY/SHX: write `reg_val` masked by
    /// `(base_hi + 1)` at a page-cross-conditional address.  `self.addr`
    /// must be set to the C4 page-wrapped address and `operands[1]` must
    /// hold `base_hi`.
    fn masked_write(&mut self, memory: &mut dyn Addressable, reg_val: u8) {
        let lo = self.addr as u8;
        let base_hi = self.operands[1];
        let hi = if self.page_crossed {
            reg_val & base_hi.wrapping_add(1)
        } else {
            (self.addr >> 8) as u8
        };
        let val = if self.page_crossed {
            hi
        } else {
            reg_val & ((self.addr >> 8) as u8).wrapping_add(1)
        };
        memory.write_byte((hi as u16) << 8 | lo as u16, val);
    }

    fn push_byte(&mut self, memory: &mut dyn Addressable, value: u8) {
        memory.write_byte(0x0100 + self.registers.sp as u16, value);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
    }

    fn pop_byte(&mut self, memory: &mut dyn Addressable) -> u8 {
        self.registers.sp = self.registers.sp.wrapping_add(1);
        memory.read_byte(0x0100 + self.registers.sp as u16)
    }

    fn reset_instruction_state(&mut self) {
        self.operands = [0; 2];
        self.addr = 0;
        self.data_latch = 0;
        self.branch_taken = false;
        self.page_crossed = false;
    }

    // ── Control flow ──

    pub fn op_jam_set_addr_ffff(&mut self) {
        self.addr = 0xFFFF;
    }
    pub fn op_jam_set_addr_fffe(&mut self) {
        self.addr = 0xFFFE;
    }

    branch_op!(op_branch_cc, |r| !r.is_flag_set(crate::registers::CARRY));
    branch_op!(op_branch_cs, |r| r.is_flag_set(crate::registers::CARRY));
    branch_op!(op_branch_eq, |r| r.is_flag_set(crate::registers::ZERO));
    branch_op!(op_branch_ne, |r| !r.is_flag_set(crate::registers::ZERO));
    branch_op!(op_branch_mi, |r| r.is_flag_set(crate::registers::NEGATIVE));
    branch_op!(op_branch_pl, |r| !r.is_flag_set(crate::registers::NEGATIVE));
    branch_op!(op_branch_vc, |r| !r.is_flag_set(crate::registers::OVERFLOW));
    branch_op!(op_branch_vs, |r| r.is_flag_set(crate::registers::OVERFLOW));

    pub fn op_jmp_abs(&mut self) {
        self.addr = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.registers.pc = self.addr;
    }
    pub fn op_jump_ind_save_lo(&mut self) {
        self.operands[1] = self.data_latch;
        self.addr = (self.addr & 0xFF00) | ((self.addr as u8).wrapping_add(1) as u16);
    }
    pub fn op_jump_ind_hi(&mut self) {
        let lo = self.operands[1];
        let hi = self.data_latch;
        self.registers.pc = (hi as u16) << 8 | lo as u16;
    }
    pub fn op_jsr_c6(&mut self) {
        self.addr = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.registers.pc = self.addr;
    }
    pub fn op_rts_finish(&mut self) {
        let pc = ((self.data_latch as u16) << 8 | self.operands[0] as u16).wrapping_add(1);
        self.registers.pc = pc;
    }
    pub fn op_rti_finish(&mut self) {
        let pc = (self.data_latch as u16) << 8 | self.operands[0] as u16;
        self.registers.pc = pc;
    }
    pub fn op_set_status(&mut self) {
        self.registers.status = (self.data_latch | crate::registers::UNUSED) & !crate::registers::BREAK;
    }

    fn branch_if(&mut self, condition: fn(&Registers) -> bool) {
        let offset = self.data_latch as i8;

        if condition(&self.registers) {
            let base = self.registers.pc;
            let target = base.wrapping_add(offset as i16 as u16);
            self.branch_taken = true;
            self.page_crossed = (base & 0xFF00) != (target & 0xFF00);
            self.addr = base;
            self.registers.pc = target;
        } else {
            self.branch_taken = false;
            self.page_crossed = false;
            self.sequence_index += 2;
        }
    }

    pub fn op_branch_dummy(&mut self) {
        if !self.page_crossed {
            self.sequence_index += 1;
        } else {
            self.addr = (self.addr & 0xFF00) | (self.registers.pc as u8 as u16);
        }
    }

    fn enter_interrupt(&mut self, interrupt: Interrupt) {
        let seq = match interrupt {
            Interrupt::NMI => INTERRUPT_SEQ_NMI,
            Interrupt::IRQ | Interrupt::BRK => INTERRUPT_SEQ_IRQ,
            Interrupt::Reset => INTERRUPT_SEQ_RESET,
        };
        self.sequence = seq;
        self.sequence_index = 0;
        self.reset_instruction_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Ram;

    #[test]
    fn test_lda_immediate() {
        let mut cpu = CPU6502::default();
        let mut mem = Ram::new();
        cpu.registers.pc = 0x0200;
        mem.write_byte(0x0200, 0xA9); // LDA #
        mem.write_byte(0x0201, 0x42);

        let cycles_before = cpu.total_cycles;
        cpu.cycle(&mut mem); // C1: Fetch
        cpu.cycle(&mut mem); // C2: ReadPC1 + SetA + EndInstr

        assert_eq!(cpu.registers.a, 0x42);
        assert_eq!(cpu.registers.pc, 0x0202);
        assert_eq!(cpu.total_cycles - cycles_before, 2);
        assert!(!cpu.registers.is_flag_set(crate::registers::ZERO));
        assert!(!cpu.registers.is_flag_set(crate::registers::NEGATIVE));
    }

    #[test]
    fn test_lda_immediate_zero() {
        let mut cpu = CPU6502::default();
        let mut mem = Ram::new();
        cpu.registers.pc = 0x0200;
        mem.write_byte(0x0200, 0xA9);
        mem.write_byte(0x0201, 0x00);

        cpu.cycle(&mut mem);
        cpu.cycle(&mut mem);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.is_flag_set(crate::registers::ZERO));
    }

    #[test]
    fn test_inx() {
        let mut cpu = CPU6502::default();
        let mut mem = Ram::new();
        cpu.registers.pc = 0x0200;
        mem.write_byte(0x0200, 0xE8); // INX
        cpu.registers.x = 0x05;

        cpu.cycle(&mut mem); // Fetch
        cpu.cycle(&mut mem); // IncX + EndInstr

        assert_eq!(cpu.registers.x, 0x06);
        assert_eq!(cpu.registers.pc, 0x0201);
    }

    #[test]
    fn test_lda_zp() {
        let mut cpu = CPU6502::default();
        let mut mem = Ram::new();
        cpu.registers.pc = 0x0200;
        mem.write_byte(0x0200, 0xA5); // LDA zp
        mem.write_byte(0x0201, 0x10); // zp address
        mem.write_byte(0x0010, 0x99); // value at zp

        cpu.cycle(&mut mem); // Fetch
        cpu.cycle(&mut mem); // ReadPC1 + SetAddrZp
        cpu.cycle(&mut mem); // ReadAddr + SetA + EndInstr

        assert_eq!(cpu.registers.a, 0x99);
        assert_eq!(cpu.registers.pc, 0x0202);
    }
}
