use crate::{
    alu,
    breakpoint::Breakpoint,
    edge_latch::EdgeLatch,
    memory::Addressable,
    micro_op::{BusOp, InternalOp, MicroOp},
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

pub struct CPU6502 {
    pub registers: Registers,

    sequence: &'static [MicroOp],
    sequence_index: usize,

    operands: [u8; 2],
    addr: u16,
    data_latch: u8,
    instruction_length: u8,

    branch_taken: bool,
    page_crossed: bool,

    pub irq_line_low: bool,
    pub nmi_latch: EdgeLatch,

    pub breakpoints: Vec<Box<dyn Breakpoint>>,

    pub halted: bool,

    pub total_cycles: u64,
}

impl CPU6502 {
    pub fn new() -> Self {
        Self {
            registers: Registers::default(),
            sequence: &[],
            sequence_index: 0,
            operands: [0; 2],
            addr: 0,
            data_latch: 0,
            instruction_length: 0,
            branch_taken: false,
            page_crossed: false,
            irq_line_low: false,
            nmi_latch: EdgeLatch::default(),
            breakpoints: Vec::new(),
            halted: false,
            total_cycles: 0,
        }
    }

    pub fn add_breakpoint(&mut self, bp: Box<dyn Breakpoint>) {
        self.breakpoints.push(bp);
    }

    pub fn reset(&mut self, memory: &mut impl Addressable) {
        self.registers = Registers::default();
        self.sequence = &[];
        self.sequence_index = 0;
        self.operands = [0; 2];
        self.addr = 0;
        self.data_latch = 0;
        self.instruction_length = 0;
        self.branch_taken = false;
        self.page_crossed = false;
        self.irq_line_low = false;
        self.nmi_latch.reset();
        self.nmi_latch.set_level(true);
        self.total_cycles = 0;
        self.halted = false;
        self.enter_interrupt(Interrupt::Reset, memory);
    }

    /// Execute one CPU clock cycle.
    pub fn cycle(&mut self, memory: &mut impl Addressable) {
        self.total_cycles += 1;

        if self.halted {
            return;
        }

        let op = if self.sequence_index < self.sequence.len() {
            self.sequence[self.sequence_index]
        } else if self.sequence.is_empty() {
            // No active instruction — fetch next
            self.fetch_and_decode(memory);
            return;
        } else {
            // Sequence exhausted — end current instruction, then fetch next
            self.end_instruction(memory);
            self.fetch_and_decode(memory);
            return;
        };

        match op.bus {
            BusOp::Fetch => {
                self.fetch_and_decode(memory);
                return; // Fetch resets sequence_index; don't increment
            }

            BusOp::ReadPC1 => {
                let val = memory.read_byte(self.registers.pc.wrapping_add(1));
                self.operands[0] = val;
                self.data_latch = val;
            }
            BusOp::ReadPC2 => {
                self.operands[1] = memory.read_byte(self.registers.pc.wrapping_add(2));
            }

            BusOp::ReadAddr => {
                self.data_latch = memory.read_byte(self.addr);
            }
            BusOp::ReadDummy => {
                let _ = memory.read_byte(self.addr);
            }
            BusOp::ReadDummyZpX => {
                let zp = self.operands[0];
                let _ = memory.read_zp_byte(zp);
                self.addr = (zp.wrapping_add(self.registers.x) & 0xFF) as u16;
            }
            BusOp::ReadDummyZpY => {
                let zp = self.operands[0];
                let _ = memory.read_zp_byte(zp);
                self.addr = (zp.wrapping_add(self.registers.y) & 0xFF) as u16;
            }
            BusOp::ReadAddrZp1 => {
                let addr = (self.addr as u8).wrapping_add(1) as u16;
                self.data_latch = memory.read_byte(addr);
            }
            BusOp::ReadDummyNext => {
                let _ = memory.read_byte(self.registers.pc.wrapping_add(1));
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
            BusOp::WriteAddrDL => memory.write_byte(self.addr, self.data_latch),
            BusOp::WriteDummy => memory.write_byte(self.addr, self.data_latch),

            BusOp::PushPCH => {
                memory.write_byte(0x0100 + self.registers.sp as u16, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            BusOp::PushPCL => {
                memory.write_byte(0x0100 + self.registers.sp as u16, self.registers.pc as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            BusOp::PushReturnHi => {
                let ret = self.registers.pc.wrapping_add(2);
                memory.write_byte(0x0100 + self.registers.sp as u16, (ret >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            BusOp::PushReturnLo => {
                let ret = self.registers.pc.wrapping_add(2);
                memory.write_byte(0x0100 + self.registers.sp as u16, ret as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            BusOp::PushA => {
                memory.write_byte(0x0100 + self.registers.sp as u16, self.registers.a);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            BusOp::PushStatusB => {
                let val = self.registers.status | crate::registers::UNUSED | crate::registers::BREAK;
                memory.write_byte(0x0100 + self.registers.sp as u16, val);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            BusOp::PushStatus => {
                let val = self.registers.status | crate::registers::UNUSED;
                memory.write_byte(0x0100 + self.registers.sp as u16, val);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }

            BusOp::PopDummy => {
                let _ = memory.read_byte(0x0100 + self.registers.sp as u16);
            }
            BusOp::Pop => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.data_latch = memory.read_byte(0x0100 + self.registers.sp as u16);
            }
            BusOp::PopPCL => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.operands[0] = memory.read_byte(0x0100 + self.registers.sp as u16);
            }
            BusOp::PopPCH => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.data_latch = memory.read_byte(0x0100 + self.registers.sp as u16);
            }

            BusOp::ReadVecLo(addr) => {
                self.operands[0] = memory.read_byte(addr);
            }
            BusOp::ReadVecHi(addr) => {
                let hi = memory.read_byte(addr);
                self.registers.pc = (hi as u16) << 8 | self.operands[0] as u16;
                self.instruction_length = 0;
            }

            BusOp::None => {} // internal-only cycle
        }

        // Execute internal operation
        self.execute_internal(op.internal, memory);

        self.sequence_index += 1;

        // If sequence exhausted, end instruction immediately (same cycle)
        if self.sequence_index >= self.sequence.len() && !self.sequence.is_empty() {
            self.end_instruction(memory);
        }
    }

    fn end_instruction(&mut self, _memory: &mut impl Addressable) {
        // instruction_length == 0 means PC was set explicitly by a control-flow
        // op (JMP, branch taken, interrupt vector). Do NOT advance PC.
        if self.instruction_length > 0 {
            self.registers.pc = self.registers.pc.wrapping_add(self.instruction_length as u16);
        }
        self.sequence = &[];
        self.sequence_index = 0;
    }

    fn fetch_and_decode(&mut self, memory: &mut impl Addressable) {
        // Check for pending interrupts before fetching
        if self.nmi_latch.take() {
            self.enter_interrupt(Interrupt::NMI, memory);
            return;
        }
        if self.irq_line_low && !self.registers.is_flag_set(crate::registers::INTERRUPT) {
            self.enter_interrupt(Interrupt::IRQ, memory);
            return;
        }

        let opcode = memory.read_byte(self.registers.pc);

        // Fire breakpoints
        for bp in &self.breakpoints {
            bp.on_hit(self.registers.pc);
        }

        self.sequence = OPCODE_SEQUENCES[opcode as usize];
        self.sequence_index = 0;
        self.operands = [0; 2];
        self.addr = 0;
        self.data_latch = 0;
        self.branch_taken = false;
        self.page_crossed = false;

        // Determine instruction length from opcode
        self.instruction_length = crate::opcode::decode(opcode).bytes;
    }

    /// Dispatch an `InternalOp` to its implementation.
    ///
    /// The `Fn` variant calls the stored function pointer (the common case for
    /// all non-parameterized operations). The three `Skip*` variants are handled
    /// inline since they carry runtime data (`u8` skip counts) that function
    /// pointers cannot express.
    fn execute_internal(&mut self, op: InternalOp, memory: &mut dyn Addressable) {
        match op {
            InternalOp::Fn(f) => f(self, memory),
            InternalOp::SkipIfCrossed(n) => {
                if self.page_crossed {
                    self.sequence_index += n as usize;
                }
            }
            InternalOp::SkipIfNotCrossed(n) => {
                if !self.page_crossed {
                    self.sequence_index += n as usize;
                }
            }
            InternalOp::SkipIfNotTaken(n) => {
                if !self.branch_taken {
                    self.sequence_index += n as usize;
                }
            }
        }
    }

    // ── Internal operation implementations (called via function pointer) ──

    pub(crate) fn op_none(&mut self, _memory: &mut dyn Addressable) {}

    // ── Address computation ──

    pub(crate) fn op_set_addr_zp(&mut self, _memory: &mut dyn Addressable) {
        self.addr = self.operands[0] as u16;
    }
    pub(crate) fn op_set_addr_abs(&mut self, _memory: &mut dyn Addressable) {
        self.addr = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
    }
    pub(crate) fn op_set_addr_absx(&mut self, _memory: &mut dyn Addressable) {
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.addr = base.wrapping_add(self.registers.x as u16);
        self.page_crossed = (base & 0xFF00) != (self.addr & 0xFF00);
        if !self.page_crossed {
            self.sequence_index += 1;
        }
    }
    pub(crate) fn op_set_addr_absy(&mut self, _memory: &mut dyn Addressable) {
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.addr = base.wrapping_add(self.registers.y as u16);
        self.page_crossed = (base & 0xFF00) != (self.addr & 0xFF00);
        if !self.page_crossed {
            self.sequence_index += 1;
        }
    }
    /// Save `data_latch` → `operands[1]` (stores lo byte for later addr compute).
    pub(crate) fn op_save_lo(&mut self, _memory: &mut dyn Addressable) {
        self.operands[1] = self.data_latch;
    }
    /// Combine `operands[1]` (lo) with `data_latch` (hi) → `self.addr`.
    pub(crate) fn op_compute_ind_addr(&mut self, _memory: &mut dyn Addressable) {
        let lo = self.operands[1] as u16;
        let hi = self.data_latch as u16;
        self.addr = (hi << 8) | lo;
    }
    pub(crate) fn op_set_addr_indy(&mut self, memory: &mut dyn Addressable) {
        let ptr = self.operands[0];
        let lo = memory.read_zp_byte(ptr);
        let hi = memory.read_zp_byte(ptr.wrapping_add(1));
        let base = (hi as u16) << 8 | lo as u16;
        self.addr = base.wrapping_add(self.registers.y as u16);
        self.page_crossed = (base & 0xFF00) != (self.addr & 0xFF00);
    }
    /// AHX/SHA (indirect),Y setup: like `op_set_addr_indy` but stores the
    /// page-wrapped address (C5 dummy-read target) in `self.addr` and saves
    /// `base_hi` in `operands[1]` for later use by `WriteAddrAHX`.
    pub(crate) fn op_ahx_setup_addr(&mut self, memory: &mut dyn Addressable) {
        let ptr = self.operands[0];
        let lo = memory.read_zp_byte(ptr);
        let hi = memory.read_zp_byte(ptr.wrapping_add(1));
        let base = (hi as u16) << 8 | lo as u16;
        let final_addr = base.wrapping_add(self.registers.y as u16);
        self.page_crossed = (base & 0xFF00) != (final_addr & 0xFF00);
        self.operands[1] = hi; // save base_hi for the write-address computation
                               // C5 dummy-read address: base page + low byte of (base + Y)
        self.addr = (hi as u16) << 8 | (lo.wrapping_add(self.registers.y)) as u16;
    }
    /// TAS/SHS (abs,Y) setup: sets SP = A & X, then computes page-wrapped address
    /// for C4 ReadDummy (like AHX but abs,Y). Reuses WriteAddrAHX for the write.
    pub(crate) fn op_tas_setup_addr(&mut self, _memory: &mut dyn Addressable) {
        self.registers.sp = self.registers.a & self.registers.x;
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        let final_addr = base.wrapping_add(self.registers.y as u16);
        self.page_crossed = (base & 0xFF00) != (final_addr & 0xFF00);
        // operands[1] already holds base_hi from ReadPC2
        // Set addr to page-wrapped address for C4 ReadDummy
        self.addr = (self.operands[1] as u16) << 8 | (self.operands[0].wrapping_add(self.registers.y)) as u16;
    }
    /// SHY (abs,X) setup: like `op_set_addr_absx` but stores the page-wrapped
    /// address (C4 dummy-read target) in `self.addr` and does NOT skip the
    /// ReadDummy cycle (always 5 cycles for SHY, unlike LDA abs,X).
    pub(crate) fn op_shy_setup_addr(&mut self, _memory: &mut dyn Addressable) {
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        let final_addr = base.wrapping_add(self.registers.x as u16);
        self.page_crossed = (base & 0xFF00) != (final_addr & 0xFF00);
        // operands[1] already holds base_hi from ReadPC2
        // Set addr to page-wrapped address for C4 ReadDummy
        self.addr = (self.operands[1] as u16) << 8 | (self.operands[0].wrapping_add(self.registers.x)) as u16;
    }
    /// SHX (abs,Y) setup: like `op_set_addr_absy` but stores the page-wrapped
    /// address (C4 dummy-read target) in `self.addr` and does NOT skip the
    /// ReadDummy cycle (always 5 cycles for SHX, unlike LDA abs,Y).
    pub(crate) fn op_shx_setup_addr(&mut self, _memory: &mut dyn Addressable) {
        let base = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        let final_addr = base.wrapping_add(self.registers.y as u16);
        self.page_crossed = (base & 0xFF00) != (final_addr & 0xFF00);
        // operands[1] already holds base_hi from ReadPC2
        // Set addr to page-wrapped address for C4 ReadDummy
        self.addr = (self.operands[1] as u16) << 8 | (self.operands[0].wrapping_add(self.registers.y)) as u16;
    }

    // ── Register operations ──

    pub(crate) fn op_set_a(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_accumulator(self.data_latch);
    }
    pub(crate) fn op_set_x(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_x(self.data_latch);
    }
    pub(crate) fn op_set_y(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_y(self.data_latch);
    }
    pub(crate) fn op_txa(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_accumulator(self.registers.x);
    }
    pub(crate) fn op_tya(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_accumulator(self.registers.y);
    }
    pub(crate) fn op_tax(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_x(self.registers.a);
    }
    pub(crate) fn op_tay(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_y(self.registers.a);
    }
    pub(crate) fn op_tsx(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_x(self.registers.sp);
    }
    pub(crate) fn op_txs(&mut self, _memory: &mut dyn Addressable) {
        self.registers.sp = self.registers.x;
    }
    pub(crate) fn op_inc_x(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_x(self.registers.x.wrapping_add(1));
    }
    pub(crate) fn op_inc_y(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_y(self.registers.y.wrapping_add(1));
    }
    pub(crate) fn op_dec_x(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_x(self.registers.x.wrapping_sub(1));
    }
    pub(crate) fn op_dec_y(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_y(self.registers.y.wrapping_sub(1));
    }

    // ── Flag operations ──

    pub(crate) fn op_set_c(&mut self, _memory: &mut dyn Addressable) {
        self.registers.update_carry_flag(true);
    }
    pub(crate) fn op_clr_c(&mut self, _memory: &mut dyn Addressable) {
        self.registers.update_carry_flag(false);
    }
    pub(crate) fn op_set_d(&mut self, _memory: &mut dyn Addressable) {
        self.registers.update_decimal_flag(true);
    }
    pub(crate) fn op_clr_d(&mut self, _memory: &mut dyn Addressable) {
        self.registers.update_decimal_flag(false);
    }
    pub(crate) fn op_set_i(&mut self, _memory: &mut dyn Addressable) {
        self.registers.update_interrupt_flag(true);
    }
    pub(crate) fn op_clr_i(&mut self, _memory: &mut dyn Addressable) {
        self.registers.update_interrupt_flag(false);
    }
    pub(crate) fn op_clr_v(&mut self, _memory: &mut dyn Addressable) {
        self.registers.update_overflow_flag(false);
    }

    // ── ALU operations ──

    pub(crate) fn op_adc(&mut self, _memory: &mut dyn Addressable) {
        alu::adc(&mut self.registers, self.data_latch);
    }
    pub(crate) fn op_sbc(&mut self, _memory: &mut dyn Addressable) {
        alu::sbc(&mut self.registers, self.data_latch);
    }
    pub(crate) fn op_and(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_accumulator(self.registers.a & self.data_latch);
    }
    pub(crate) fn op_ora(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_accumulator(self.registers.a | self.data_latch);
    }
    pub(crate) fn op_eor(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_accumulator(self.registers.a ^ self.data_latch);
    }
    pub(crate) fn op_cmp_a(&mut self, _memory: &mut dyn Addressable) {
        let a = self.registers.a;
        alu::compare(&mut self.registers, a, self.data_latch);
    }
    pub(crate) fn op_cmp_x(&mut self, _memory: &mut dyn Addressable) {
        let x = self.registers.x;
        alu::compare(&mut self.registers, x, self.data_latch);
    }
    pub(crate) fn op_cmp_y(&mut self, _memory: &mut dyn Addressable) {
        let y = self.registers.y;
        alu::compare(&mut self.registers, y, self.data_latch);
    }
    pub(crate) fn op_bit(&mut self, _memory: &mut dyn Addressable) {
        let v = self.data_latch;
        self.registers.update_zero_flag(self.registers.a & v == 0);
        self.registers.update_overflow_flag(v & 0x40 != 0);
        self.registers.update_negative_flag(v & 0x80 != 0);
    }

    // ── RMW memory operations ──

    pub(crate) fn op_asl(&mut self, _memory: &mut dyn Addressable) {
        let c = self.data_latch & 0x80 != 0;
        self.data_latch <<= 1;
        self.registers.update_carry_flag(c);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub(crate) fn op_lsr(&mut self, _memory: &mut dyn Addressable) {
        let c = self.data_latch & 0x01 != 0;
        self.data_latch >>= 1;
        self.registers.update_carry_flag(c);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub(crate) fn op_rol(&mut self, _memory: &mut dyn Addressable) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.data_latch & 0x80 != 0;
        self.data_latch = (self.data_latch << 1) | old_c;
        self.registers.update_carry_flag(new_c);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub(crate) fn op_ror(&mut self, _memory: &mut dyn Addressable) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.data_latch & 0x01 != 0;
        self.data_latch = (self.data_latch >> 1) | (old_c << 7);
        self.registers.update_carry_flag(new_c);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub(crate) fn op_inc(&mut self, _memory: &mut dyn Addressable) {
        self.data_latch = self.data_latch.wrapping_add(1);
        self.registers.update_zero_and_negative(self.data_latch);
    }
    pub(crate) fn op_dec(&mut self, _memory: &mut dyn Addressable) {
        self.data_latch = self.data_latch.wrapping_sub(1);
        self.registers.update_zero_and_negative(self.data_latch);
    }

    // ── Accumulator shifts ──

    pub(crate) fn op_asl_a(&mut self, _memory: &mut dyn Addressable) {
        let c = self.registers.a & 0x80 != 0;
        let result = self.registers.a << 1;
        self.registers.update_carry_flag(c);
        self.registers.set_accumulator(result);
    }
    pub(crate) fn op_lsr_a(&mut self, _memory: &mut dyn Addressable) {
        let c = self.registers.a & 0x01 != 0;
        let result = self.registers.a >> 1;
        self.registers.update_carry_flag(c);
        self.registers.set_accumulator(result);
    }
    pub(crate) fn op_rol_a(&mut self, _memory: &mut dyn Addressable) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.registers.a & 0x80 != 0;
        let result = (self.registers.a << 1) | old_c;
        self.registers.update_carry_flag(new_c);
        self.registers.set_accumulator(result);
    }
    pub(crate) fn op_ror_a(&mut self, _memory: &mut dyn Addressable) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.registers.a & 0x01 != 0;
        let result = (self.registers.a >> 1) | (old_c << 7);
        self.registers.update_carry_flag(new_c);
        self.registers.set_accumulator(result);
    }

    // ── Undocumented load/store ──

    pub(crate) fn op_las(&mut self, _memory: &mut dyn Addressable) {
        let val = self.registers.sp & self.data_latch;
        self.registers.set_accumulator(val);
        self.registers.set_x(val);
        self.registers.sp = val;
    }

    // ── Unofficial opcodes ──

    pub(crate) fn op_lax(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_accumulator(self.data_latch);
        self.registers.set_x(self.data_latch);
    }
    pub(crate) fn op_lax_imm(&mut self, _memory: &mut dyn Addressable) {
        let result = self.data_latch & (self.registers.a | 0xEE);
        self.registers.set_accumulator(result);
        self.registers.set_x(result);
    }
    pub(crate) fn op_slo(&mut self, _memory: &mut dyn Addressable) {
        let c = self.data_latch & 0x80 != 0;
        self.data_latch <<= 1;
        self.registers.update_carry_flag(c);
        self.registers.update_zero_and_negative(self.data_latch);
        let a = self.registers.a | self.data_latch;
        self.registers.set_accumulator(a);
    }
    pub(crate) fn op_rla(&mut self, _memory: &mut dyn Addressable) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.data_latch & 0x80 != 0;
        self.data_latch = (self.data_latch << 1) | old_c;
        self.registers.update_carry_flag(new_c);
        self.registers.update_zero_and_negative(self.data_latch);
        let a = self.registers.a & self.data_latch;
        self.registers.set_accumulator(a);
    }
    pub(crate) fn op_sre(&mut self, _memory: &mut dyn Addressable) {
        let c = self.data_latch & 0x01 != 0;
        self.data_latch >>= 1;
        self.registers.update_carry_flag(c);
        self.registers.update_zero_and_negative(self.data_latch);
        let a = self.registers.a ^ self.data_latch;
        self.registers.set_accumulator(a);
    }
    pub(crate) fn op_rra(&mut self, _memory: &mut dyn Addressable) {
        let old_c = self.registers.is_flag_set(crate::registers::CARRY) as u8;
        let new_c = self.data_latch & 0x01 != 0;
        self.data_latch = (self.data_latch >> 1) | (old_c << 7);
        self.registers.update_carry_flag(new_c);
        self.registers.update_zero_and_negative(self.data_latch);
        alu::adc(&mut self.registers, self.data_latch);
    }
    pub(crate) fn op_dcp(&mut self, _memory: &mut dyn Addressable) {
        self.data_latch = self.data_latch.wrapping_sub(1);
        self.registers.update_zero_and_negative(self.data_latch);
        let a = self.registers.a;
        alu::compare(&mut self.registers, a, self.data_latch);
    }
    pub(crate) fn op_isc(&mut self, _memory: &mut dyn Addressable) {
        self.data_latch = self.data_latch.wrapping_add(1);
        self.registers.update_zero_and_negative(self.data_latch);
        alu::sbc(&mut self.registers, self.data_latch);
    }
    pub(crate) fn op_anc(&mut self, _memory: &mut dyn Addressable) {
        self.registers.set_accumulator(self.registers.a & self.data_latch);
        self.registers
            .update_carry_flag(self.registers.is_flag_set(crate::registers::NEGATIVE));
    }
    pub(crate) fn op_alr(&mut self, _memory: &mut dyn Addressable) {
        let result = self.registers.a & self.data_latch;
        self.registers.update_carry_flag(result & 1 != 0);
        let shifted = result >> 1;
        self.registers.a = shifted;
        self.registers.update_zero_and_negative(shifted);
    }
    pub(crate) fn op_arr(&mut self, _memory: &mut dyn Addressable) {
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
    pub(crate) fn op_xaa(&mut self, _memory: &mut dyn Addressable) {
        let val = (self.registers.a | 0xEE) & self.registers.x & self.data_latch;
        self.registers.set_accumulator(val);
    }
    pub(crate) fn op_sbx(&mut self, _memory: &mut dyn Addressable) {
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
        let val = reg_val & ((self.addr >> 8) as u8).wrapping_add(1);
        memory.write_byte((hi as u16) << 8 | lo as u16, val);
    }

    // ── Control flow ──

    pub(crate) fn op_jam_halt(&mut self, _memory: &mut dyn Addressable) {
        self.halted = true;
    }

    pub(crate) fn op_branch_cc(&mut self, _memory: &mut dyn Addressable) {
        self.branch_if(|r| !r.is_flag_set(crate::registers::CARRY))
    }
    pub(crate) fn op_branch_cs(&mut self, _memory: &mut dyn Addressable) {
        self.branch_if(|r| r.is_flag_set(crate::registers::CARRY))
    }
    pub(crate) fn op_branch_eq(&mut self, _memory: &mut dyn Addressable) {
        self.branch_if(|r| r.is_flag_set(crate::registers::ZERO))
    }
    pub(crate) fn op_branch_ne(&mut self, _memory: &mut dyn Addressable) {
        self.branch_if(|r| !r.is_flag_set(crate::registers::ZERO))
    }
    pub(crate) fn op_branch_mi(&mut self, _memory: &mut dyn Addressable) {
        self.branch_if(|r| r.is_flag_set(crate::registers::NEGATIVE))
    }
    pub(crate) fn op_branch_pl(&mut self, _memory: &mut dyn Addressable) {
        self.branch_if(|r| !r.is_flag_set(crate::registers::NEGATIVE))
    }
    pub(crate) fn op_branch_vc(&mut self, _memory: &mut dyn Addressable) {
        self.branch_if(|r| !r.is_flag_set(crate::registers::OVERFLOW))
    }
    pub(crate) fn op_branch_vs(&mut self, _memory: &mut dyn Addressable) {
        self.branch_if(|r| r.is_flag_set(crate::registers::OVERFLOW))
    }

    pub(crate) fn op_jmp_abs(&mut self, _memory: &mut dyn Addressable) {
        self.addr = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.registers.pc = self.addr;
        self.instruction_length = 0;
    }
    pub(crate) fn op_jump_ind(&mut self, memory: &mut dyn Addressable) {
        let lo = memory.read_byte(self.addr);
        // NMOS 6502 bug: page wrap on indirect JMP
        let hi_addr = (self.addr & 0xFF00) | ((self.addr as u8).wrapping_add(1) as u16);
        let hi = memory.read_byte(hi_addr);
        self.registers.pc = (hi as u16) << 8 | lo as u16;
        self.instruction_length = 0;
    }
    pub(crate) fn op_jsr_c6(&mut self, memory: &mut dyn Addressable) {
        self.operands[1] = memory.read_byte(self.registers.pc.wrapping_add(2));
        self.addr = (self.operands[1] as u16) << 8 | self.operands[0] as u16;
        self.registers.pc = self.addr;
        self.instruction_length = 0;
    }
    pub(crate) fn op_rts_finish(&mut self, _memory: &mut dyn Addressable) {
        let pc = ((self.data_latch as u16) << 8 | self.operands[0] as u16).wrapping_add(1);
        self.registers.pc = pc;
        self.instruction_length = 0;
    }
    pub(crate) fn op_rti_finish(&mut self, _memory: &mut dyn Addressable) {
        let pc = (self.data_latch as u16) << 8 | self.operands[0] as u16;
        self.registers.pc = pc;
        self.instruction_length = 0;
    }
    pub(crate) fn op_set_status(&mut self, _memory: &mut dyn Addressable) {
        self.registers.status = (self.data_latch | crate::registers::UNUSED) & !crate::registers::BREAK;
    }

    fn branch_if(&mut self, condition: fn(&Registers) -> bool) {
        let offset = self.data_latch as i8;

        if condition(&self.registers) {
            let base = self.registers.pc.wrapping_add(2);
            let target = base.wrapping_add(offset as i16 as u16);
            self.branch_taken = true;
            self.page_crossed = (base & 0xFF00) != (target & 0xFF00);
            self.addr = target;
            self.registers.pc = target;
            self.instruction_length = 0;
        } else {
            self.branch_taken = false;
            self.page_crossed = false;
            self.sequence_index += 2;
        }
    }

    fn enter_interrupt(&mut self, interrupt: Interrupt, _memory: &mut dyn Addressable) {
        let seq = match interrupt {
            Interrupt::NMI => INTERRUPT_SEQ_NMI,
            Interrupt::IRQ => INTERRUPT_SEQ_IRQ,
            Interrupt::Reset => INTERRUPT_SEQ_RESET,
            Interrupt::BRK => INTERRUPT_SEQ_IRQ, // BRK uses IRQ vector; handled via S_BRK sequence, not this path
        };
        self.sequence = seq;
        self.sequence_index = 0;
        self.operands = [0; 2];
        self.addr = 0;
        self.data_latch = 0;
        self.branch_taken = false;
        self.page_crossed = false;
        match interrupt {
            Interrupt::NMI | Interrupt::IRQ => self.instruction_length = 0,
            Interrupt::Reset => self.instruction_length = 0,
            Interrupt::BRK => self.instruction_length = 0,
        }
    }
}

impl Default for CPU6502 {
    fn default() -> Self {
        Self::new()
    }
}

/// Return the instruction length in bytes for a given opcode.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Ram;

    #[test]
    fn test_lda_immediate() {
        let mut cpu = CPU6502::new();
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
        let mut cpu = CPU6502::new();
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
        let mut cpu = CPU6502::new();
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
        let mut cpu = CPU6502::new();
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
