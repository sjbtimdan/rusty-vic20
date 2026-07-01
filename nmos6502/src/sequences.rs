//! Static instruction sequences — one per opcode, indexed by opcode byte.
//!
//! Each sequence represents cycles 2+ of an instruction. Cycle 1 (opcode
//! fetch) is handled by `CPU6502::step()` when no sequence is active.
//!
//! The CPU always drives the address bus every cycle — there are no idle bus
//! cycles. Even internal-only register operations perform a dummy read from a
//! (possibly stale) address.

use crate::{
    cpu::CPU6502,
    micro_op::{BusOp, InternalOp, MicroOp},
};

// Shorthand
use BusOp as B;
use InternalOp as I;

const fn m(bus: B, internal: I) -> MicroOp {
    MicroOp { bus, internal }
}

const fn b(bus: B) -> MicroOp {
    MicroOp {
        bus,
        internal: CPU6502::op_none,
    }
}

const R_ZP: MicroOp = m(B::ReadPC1, CPU6502::op_set_addr_zp);

const C_ABSX: MicroOp = m(B::ReadPC2, CPU6502::op_set_addr_absx);
const C_ABSY: MicroOp = m(B::ReadPC2, CPU6502::op_set_addr_absy);

const X_DUMMY: MicroOp = m(B::ReadDummy, CPU6502::op_fix_addr_cross);

// ── Addressing-mode family macros ──

const fn seq_imm(op: InternalOp) -> [MicroOp; 1] {
    [m(B::ReadPC1, op)]
}
const fn seq_zp(op: InternalOp) -> [MicroOp; 2] {
    [R_ZP, m(B::ReadAddr, op)]
}
const fn seq_zpx(op: InternalOp) -> [MicroOp; 3] {
    [b(B::ReadPC1), b(B::ReadDummyZpX), m(B::ReadAddr, op)]
}
const fn seq_zpy(op: InternalOp) -> [MicroOp; 3] {
    [b(B::ReadPC1), b(B::ReadDummyZpY), m(B::ReadAddr, op)]
}
const fn seq_abs(op: InternalOp) -> [MicroOp; 3] {
    [
        b(B::ReadPC1),
        m(B::ReadPC2, CPU6502::op_set_addr_abs),
        m(B::ReadAddr, op),
    ]
}
const fn seq_absx(op: InternalOp) -> [MicroOp; 4] {
    [b(B::ReadPC1), C_ABSX, X_DUMMY, m(B::ReadAddr, op)]
}
const fn seq_absy(op: InternalOp) -> [MicroOp; 4] {
    [b(B::ReadPC1), C_ABSY, X_DUMMY, m(B::ReadAddr, op)]
}
const fn seq_indx(op: InternalOp) -> [MicroOp; 5] {
    [
        b(B::ReadPC1),
        b(B::ReadDummyZpX),
        m(B::ReadAddr, CPU6502::op_save_lo),
        m(B::ReadAddrZp1, CPU6502::op_compute_ind_addr),
        m(B::ReadAddr, op),
    ]
}
const fn seq_indy(op: InternalOp) -> [MicroOp; 5] {
    [
        m(B::ReadPC1, CPU6502::op_set_addr_zp),
        m(B::ReadAddr, CPU6502::op_save_lo),
        m(B::ReadAddrZp1, CPU6502::op_compute_indy_addr),
        X_DUMMY,
        m(B::ReadAddr, op),
    ]
}

const fn seq_implied(op: InternalOp) -> [MicroOp; 1] {
    [m(B::ReadDummyNext, op)]
}

macro_rules! interrupt_seq {
    ($n:ident, $lo:expr, $hi:expr) => {
        pub static $n: &[MicroOp] = &[
            b(B::ReadDummy),
            b(B::ReadDummy),
            b(B::PushPCH),
            b(B::PushPCL),
            m(B::PushStatus, CPU6502::op_set_i),
            b(B::ReadVecLo($lo)),
            b(B::ReadVecHi($hi)),
        ];
    };
}

const fn rmw_zp(op: InternalOp) -> [MicroOp; 4] {
    [R_ZP, b(B::ReadAddr), m(B::WriteDummy, op), b(B::WriteAddrDL)]
}
const fn rmw_zpx(op: InternalOp) -> [MicroOp; 5] {
    [
        b(B::ReadPC1),
        b(B::ReadDummyZpX),
        b(B::ReadAddr),
        m(B::WriteDummy, op),
        b(B::WriteAddrDL),
    ]
}
const fn rmw_abs(op: InternalOp) -> [MicroOp; 5] {
    [
        b(B::ReadPC1),
        m(B::ReadPC2, CPU6502::op_set_addr_abs),
        b(B::ReadAddr),
        m(B::WriteDummy, op),
        b(B::WriteAddrDL),
    ]
}
const fn rmw_absx(op: InternalOp) -> [MicroOp; 6] {
    [
        b(B::ReadPC1),
        m(B::ReadPC2, CPU6502::op_set_addr_absx_full),
        X_DUMMY,
        b(B::ReadAddr),
        m(B::WriteDummy, op),
        b(B::WriteAddrDL),
    ]
}
const fn rmw_absy(op: InternalOp) -> [MicroOp; 6] {
    [
        b(B::ReadPC1),
        m(B::ReadPC2, CPU6502::op_set_addr_absy_full),
        X_DUMMY,
        b(B::ReadAddr),
        m(B::WriteDummy, op),
        b(B::WriteAddrDL),
    ]
}
const fn rmw_indx(op: InternalOp) -> [MicroOp; 7] {
    [
        b(B::ReadPC1),
        b(B::ReadDummyZpX),
        m(B::ReadAddr, CPU6502::op_save_lo),
        m(B::ReadAddrZp1, CPU6502::op_compute_ind_addr),
        b(B::ReadAddr),
        m(B::WriteDummy, op),
        b(B::WriteAddrDL),
    ]
}
const fn rmw_indy(op: InternalOp) -> [MicroOp; 7] {
    [
        m(B::ReadPC1, CPU6502::op_set_addr_zp),
        m(B::ReadAddr, CPU6502::op_save_lo),
        m(B::ReadAddrZp1, CPU6502::op_compute_indy_addr_rmw),
        X_DUMMY,
        b(B::ReadAddr),
        m(B::WriteDummy, op),
        b(B::WriteAddrDL),
    ]
}
const fn branch_seq(op: InternalOp) -> [MicroOp; 3] {
    [
        m(B::ReadPC1, op),
        m(B::ReadDummy, CPU6502::op_branch_dummy),
        b(B::ReadDummy),
    ]
}

// ── Sequences ──

// Implied/register
static S_JAM: &[MicroOp] = &[
    m(B::ReadDummyNext, CPU6502::op_jam_set_addr_ffff),
    m(B::ReadDummy, CPU6502::op_jam_set_addr_fffe),
    b(B::ReadDummy),
    m(B::ReadDummy, CPU6502::op_jam_set_addr_ffff),
    b(B::ReadDummy),
    b(B::ReadDummy),
    b(B::ReadDummy),
    b(B::ReadDummy),
    b(B::ReadDummy),
    b(B::ReadDummy),
];
static S_NOP: &[MicroOp] = &seq_implied(CPU6502::op_none);
static S_INX: &[MicroOp] = &seq_implied(CPU6502::op_inc_x);
static S_INY: &[MicroOp] = &seq_implied(CPU6502::op_inc_y);
static S_DEX: &[MicroOp] = &seq_implied(CPU6502::op_dec_x);
static S_DEY: &[MicroOp] = &seq_implied(CPU6502::op_dec_y);
static S_TXA: &[MicroOp] = &seq_implied(CPU6502::op_txa);
static S_TYA: &[MicroOp] = &seq_implied(CPU6502::op_tya);
static S_TAX: &[MicroOp] = &seq_implied(CPU6502::op_tax);
static S_TAY: &[MicroOp] = &seq_implied(CPU6502::op_tay);
static S_TSX: &[MicroOp] = &seq_implied(CPU6502::op_tsx);
static S_TXS: &[MicroOp] = &seq_implied(CPU6502::op_txs);
static S_CLC: &[MicroOp] = &seq_implied(CPU6502::op_clr_c);
static S_SEC: &[MicroOp] = &seq_implied(CPU6502::op_set_c);
static S_CLD: &[MicroOp] = &seq_implied(CPU6502::op_clr_d);
static S_SED: &[MicroOp] = &seq_implied(CPU6502::op_set_d);
static S_CLI: &[MicroOp] = &seq_implied(CPU6502::op_clr_i);
static S_SEI: &[MicroOp] = &seq_implied(CPU6502::op_set_i);
static S_CLV: &[MicroOp] = &seq_implied(CPU6502::op_clr_v);

// Stack
static S_PHA: &[MicroOp] = &[b(B::ReadDummyNext), b(B::PushA)];
static S_PHP: &[MicroOp] = &[b(B::ReadDummyNext), b(B::PushStatusB)];
static S_PLA: &[MicroOp] = &[b(B::ReadDummyNext), b(B::PopDummy), m(B::Pop, CPU6502::op_set_a)];
static S_PLP: &[MicroOp] = &[b(B::ReadDummyNext), b(B::PopDummy), m(B::Pop, CPU6502::op_set_status)];

// Immediate
static S_LDA_IMM: &[MicroOp] = &seq_imm(CPU6502::op_set_a);
static S_LDX_IMM: &[MicroOp] = &seq_imm(CPU6502::op_set_x);
static S_LDY_IMM: &[MicroOp] = &seq_imm(CPU6502::op_set_y);
static S_ADC_IMM: &[MicroOp] = &seq_imm(CPU6502::op_adc);
static S_SBC_IMM: &[MicroOp] = &seq_imm(CPU6502::op_sbc);
static S_AND_IMM: &[MicroOp] = &seq_imm(CPU6502::op_and);
static S_ORA_IMM: &[MicroOp] = &seq_imm(CPU6502::op_ora);
static S_EOR_IMM: &[MicroOp] = &seq_imm(CPU6502::op_eor);
static S_CMP_IMM: &[MicroOp] = &seq_imm(CPU6502::op_cmp_a);
static S_CPX_IMM: &[MicroOp] = &seq_imm(CPU6502::op_cmp_x);
static S_CPY_IMM: &[MicroOp] = &seq_imm(CPU6502::op_cmp_y);

// Zero page read
static S_LDA_ZP: &[MicroOp] = &seq_zp(CPU6502::op_set_a);
static S_LDX_ZP: &[MicroOp] = &seq_zp(CPU6502::op_set_x);
static S_LDY_ZP: &[MicroOp] = &seq_zp(CPU6502::op_set_y);
static S_ADC_ZP: &[MicroOp] = &seq_zp(CPU6502::op_adc);
static S_SBC_ZP: &[MicroOp] = &seq_zp(CPU6502::op_sbc);
static S_AND_ZP: &[MicroOp] = &seq_zp(CPU6502::op_and);
static S_NOP_ZP: &[MicroOp] = &[R_ZP, b(B::ReadDummy)];
static S_ORA_ZP: &[MicroOp] = &seq_zp(CPU6502::op_ora);
static S_EOR_ZP: &[MicroOp] = &seq_zp(CPU6502::op_eor);
static S_CMP_ZP: &[MicroOp] = &seq_zp(CPU6502::op_cmp_a);
static S_CPX_ZP: &[MicroOp] = &seq_zp(CPU6502::op_cmp_x);
static S_CPY_ZP: &[MicroOp] = &seq_zp(CPU6502::op_cmp_y);
static S_BIT_ZP: &[MicroOp] = &seq_zp(CPU6502::op_bit);

// Zero page write
static S_STA_ZP: &[MicroOp] = &[R_ZP, b(B::WriteAddrA)];
static S_STX_ZP: &[MicroOp] = &[R_ZP, b(B::WriteAddrX)];
static S_STY_ZP: &[MicroOp] = &[R_ZP, b(B::WriteAddrY)];

// Zero page indexed X read
static S_LDA_ZPX: &[MicroOp] = &seq_zpx(CPU6502::op_set_a);
static S_LDY_ZPX: &[MicroOp] = &seq_zpx(CPU6502::op_set_y);
static S_ADC_ZPX: &[MicroOp] = &seq_zpx(CPU6502::op_adc);
static S_SBC_ZPX: &[MicroOp] = &seq_zpx(CPU6502::op_sbc);
static S_AND_ZPX: &[MicroOp] = &seq_zpx(CPU6502::op_and);
static S_ORA_ZPX: &[MicroOp] = &seq_zpx(CPU6502::op_ora);
static S_EOR_ZPX: &[MicroOp] = &seq_zpx(CPU6502::op_eor);
static S_CMP_ZPX: &[MicroOp] = &seq_zpx(CPU6502::op_cmp_a);

// Zero page indexed X write
static S_STA_ZPX: &[MicroOp] = &[b(B::ReadPC1), b(B::ReadDummyZpX), b(B::WriteAddrA)];
static S_STY_ZPX: &[MicroOp] = &[b(B::ReadPC1), b(B::ReadDummyZpX), b(B::WriteAddrY)];

// Zero page indexed Y
static S_LDX_ZPY: &[MicroOp] = &seq_zpy(CPU6502::op_set_x);
static S_STX_ZPY: &[MicroOp] = &[b(B::ReadPC1), b(B::ReadDummyZpY), b(B::WriteAddrX)];

// Absolute read
static S_LDA_ABS: &[MicroOp] = &seq_abs(CPU6502::op_set_a);
static S_LDX_ABS: &[MicroOp] = &seq_abs(CPU6502::op_set_x);
static S_LDY_ABS: &[MicroOp] = &seq_abs(CPU6502::op_set_y);
static S_ADC_ABS: &[MicroOp] = &seq_abs(CPU6502::op_adc);
static S_SBC_ABS: &[MicroOp] = &seq_abs(CPU6502::op_sbc);
static S_AND_ABS: &[MicroOp] = &seq_abs(CPU6502::op_and);
static S_ORA_ABS: &[MicroOp] = &seq_abs(CPU6502::op_ora);
static S_EOR_ABS: &[MicroOp] = &seq_abs(CPU6502::op_eor);
static S_CMP_ABS: &[MicroOp] = &seq_abs(CPU6502::op_cmp_a);
static S_CPX_ABS: &[MicroOp] = &seq_abs(CPU6502::op_cmp_x);
static S_CPY_ABS: &[MicroOp] = &seq_abs(CPU6502::op_cmp_y);
static S_BIT_ABS: &[MicroOp] = &seq_abs(CPU6502::op_bit);
static S_NOP_ABS: &[MicroOp] = &[b(B::ReadPC1), m(B::ReadPC2, CPU6502::op_set_addr_abs), b(B::ReadDummy)];

// Absolute write
static S_STA_ABS: &[MicroOp] = &[b(B::ReadPC1), m(B::ReadPC2, CPU6502::op_set_addr_abs), b(B::WriteAddrA)];
static S_STX_ABS: &[MicroOp] = &[b(B::ReadPC1), m(B::ReadPC2, CPU6502::op_set_addr_abs), b(B::WriteAddrX)];
static S_STY_ABS: &[MicroOp] = &[b(B::ReadPC1), m(B::ReadPC2, CPU6502::op_set_addr_abs), b(B::WriteAddrY)];

// JMP
static S_JMP_ABS: &[MicroOp] = &[b(B::ReadPC1), m(B::ReadPC2, CPU6502::op_jmp_abs)];
static S_JMP_IND: &[MicroOp] = &[
    b(B::ReadPC1),
    m(B::ReadPC2, CPU6502::op_set_addr_abs),
    m(B::ReadAddr, CPU6502::op_jump_ind_save_lo),
    m(B::ReadAddr, CPU6502::op_jump_ind_hi),
];

// Absolute indexed X read (with page-cross handling)
static S_LDA_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_set_a);
static S_LDY_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_set_y);
static S_ADC_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_adc);
static S_SBC_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_sbc);
static S_AND_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_and);
static S_ORA_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_ora);
static S_EOR_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_eor);
static S_CMP_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_cmp_a);

// Absolute indexed Y read
static S_LDA_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_set_a);
static S_LDX_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_set_x);
static S_ADC_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_adc);
static S_SBC_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_sbc);
static S_AND_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_and);
static S_ORA_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_ora);
static S_EOR_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_eor);
static S_CMP_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_cmp_a);

// Absolute indexed write (always 5 cycles)
static S_STA_ABSX: &[MicroOp] = &[
    b(B::ReadPC1),
    m(B::ReadPC2, CPU6502::op_set_addr_absx_full),
    X_DUMMY,
    b(B::WriteAddrA),
];

static S_STA_ABSY: &[MicroOp] = &[
    b(B::ReadPC1),
    m(B::ReadPC2, CPU6502::op_set_addr_absy_full),
    X_DUMMY,
    b(B::WriteAddrA),
];
// RMW zero page
static S_ASL_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_asl);
static S_LSR_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_lsr);
static S_ROL_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_rol);
static S_ROR_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_ror);
static S_INC_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_inc);
static S_DEC_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_dec);

// RMW accumulator
static S_ASL_A: &[MicroOp] = &seq_implied(CPU6502::op_asl_a);
static S_LSR_A: &[MicroOp] = &seq_implied(CPU6502::op_lsr_a);
static S_ROL_A: &[MicroOp] = &seq_implied(CPU6502::op_rol_a);
static S_ROR_A: &[MicroOp] = &seq_implied(CPU6502::op_ror_a);

static S_NOP_ABSX: &[MicroOp] = &seq_absx(CPU6502::op_none);

static S_NOP_ZPX: &[MicroOp] = &[b(B::ReadPC1), b(B::ReadDummyZpX), b(B::ReadDummy)];

static S_NOP_IMM: &[MicroOp] = &seq_imm(CPU6502::op_none);
static S_ANC_A: &[MicroOp] = &seq_imm(CPU6502::op_anc);
static S_ALR_IMM: &[MicroOp] = &seq_imm(CPU6502::op_alr);
static S_ARR_IMM: &[MicroOp] = &seq_imm(CPU6502::op_arr);

// RMW absolute
static S_ASL_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_asl);
static S_LSR_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_lsr);
static S_ROL_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_rol);
static S_ROR_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_ror);
static S_INC_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_inc);
static S_DEC_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_dec);

// RMW absolute indexed X (always 7 cycles)
static S_ASL_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_asl);
static S_LSR_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_lsr);
static S_ROL_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_rol);
static S_ROR_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_ror);
static S_INC_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_inc);
static S_DEC_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_dec);

// RMW zero page indexed X (6 cycles)
static S_ASL_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_asl);
static S_LSR_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_lsr);
static S_ROL_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_rol);
static S_ROR_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_ror);
static S_INC_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_inc);
static S_DEC_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_dec);

// Indexed indirect (zp,X) read/write
static S_ORA_INDX: &[MicroOp] = &seq_indx(CPU6502::op_ora);
static S_AND_INDX: &[MicroOp] = &seq_indx(CPU6502::op_and);
static S_EOR_INDX: &[MicroOp] = &seq_indx(CPU6502::op_eor);
static S_ADC_INDX: &[MicroOp] = &seq_indx(CPU6502::op_adc);
static S_SBC_INDX: &[MicroOp] = &seq_indx(CPU6502::op_sbc);
static S_CMP_INDX: &[MicroOp] = &seq_indx(CPU6502::op_cmp_a);
static S_LDA_INDX: &[MicroOp] = &seq_indx(CPU6502::op_set_a);
static S_STA_INDX: &[MicroOp] = &[
    b(B::ReadPC1),
    b(B::ReadDummyZpX),
    m(B::ReadAddr, CPU6502::op_save_lo),
    m(B::ReadAddrZp1, CPU6502::op_compute_ind_addr),
    b(B::WriteAddrA),
];

// Branches
static S_BCC: &[MicroOp] = &branch_seq(CPU6502::op_branch_cc);
static S_BCS: &[MicroOp] = &branch_seq(CPU6502::op_branch_cs);
static S_BEQ: &[MicroOp] = &branch_seq(CPU6502::op_branch_eq);
static S_BNE: &[MicroOp] = &branch_seq(CPU6502::op_branch_ne);
static S_BMI: &[MicroOp] = &branch_seq(CPU6502::op_branch_mi);
static S_BPL: &[MicroOp] = &branch_seq(CPU6502::op_branch_pl);
static S_BVC: &[MicroOp] = &branch_seq(CPU6502::op_branch_vc);
static S_BVS: &[MicroOp] = &branch_seq(CPU6502::op_branch_vs);

// JSR / RTS / RTI / BRK
static S_JSR: &[MicroOp] = &[
    b(B::ReadPC1),
    b(B::PopDummy),
    b(B::PushReturnHi),
    b(B::PushReturnLo),
    m(B::ReadPC2, CPU6502::op_jsr_c6),
];
static S_RTS: &[MicroOp] = &[
    b(B::ReadDummyNext),
    b(B::PopDummy),
    b(B::PopPCL),
    m(B::PopPCH, CPU6502::op_rts_finish),
    b(B::ReadRTS),
];
static S_RTI: &[MicroOp] = &[
    b(B::ReadDummyNext),
    b(B::PopDummy),
    m(B::Pop, CPU6502::op_set_status),
    b(B::PopPCL),
    m(B::PopPCH, CPU6502::op_rti_finish),
];
static S_BRK: &[MicroOp] = &[
    b(B::ReadPC1),
    b(B::PushReturnHi),
    b(B::PushReturnLo),
    m(B::PushStatusB, CPU6502::op_set_i),
    b(B::ReadVecLo(0xFFFE)),
    b(B::ReadVecHi(0xFFFF)),
];

// Indirect indexed (zp),Y read — 5 cycles, +1 if page cross
// Note: SetAddrIndY performs both zp pointer reads internally (simplification).
static S_ORA_INDY: &[MicroOp] = &seq_indy(CPU6502::op_ora);
static S_AND_INDY: &[MicroOp] = &seq_indy(CPU6502::op_and);
static S_EOR_INDY: &[MicroOp] = &seq_indy(CPU6502::op_eor);
static S_ADC_INDY: &[MicroOp] = &seq_indy(CPU6502::op_adc);
static S_SBC_INDY: &[MicroOp] = &seq_indy(CPU6502::op_sbc);
static S_CMP_INDY: &[MicroOp] = &seq_indy(CPU6502::op_cmp_a);
static S_LDA_INDY: &[MicroOp] = &seq_indy(CPU6502::op_set_a);

// Indirect indexed (zp),Y write — 6 cycles (always, extra dummy read)
static S_STA_INDY: &[MicroOp] = &[
    m(B::ReadPC1, CPU6502::op_set_addr_zp),
    m(B::ReadAddr, CPU6502::op_save_lo),
    m(B::ReadAddrZp1, CPU6502::op_compute_indy_addr_rmw),
    X_DUMMY,
    b(B::WriteAddrA),
];

static S_AHX_INDY: &[MicroOp] = &[
    m(B::ReadPC1, CPU6502::op_set_addr_zp),
    m(B::ReadAddr, CPU6502::op_save_lo),
    m(B::ReadAddrZp1, CPU6502::op_compute_ahx_addr),
    X_DUMMY,
    b(B::WriteAddrAHX),
];

// SHY/SYA (abs,X) — store Y & (base_hi + 1) at abs,X with page-cross masking
// C_ABSX but with SHY setup: reads PC2 for high byte, computes page-wrapped addr
static S_SHY_ABSX: &[MicroOp] = &[
    b(B::ReadPC1),
    m(B::ReadPC2, CPU6502::op_shy_setup_addr),
    X_DUMMY,
    b(B::WriteAddrSHY),
];

// SHX/SXA (abs,Y) — store X & (base_hi + 1) at abs,Y with page-cross masking
// C_ABSY but with SHX setup: reads PC2 for high byte, computes page-wrapped addr
static S_SHX_ABSY: &[MicroOp] = &[
    b(B::ReadPC1),
    m(B::ReadPC2, CPU6502::op_shx_setup_addr),
    X_DUMMY,
    b(B::WriteAddrSHX),
];

// XAA/ANE (imm) — A = (A | $EE) & X & operand
static S_XAA_IMM: &[MicroOp] = &seq_imm(CPU6502::op_xaa);

// TAS/SHS (abs,Y) — SP = A & X, then store A & X & (base_hi + 1) at abs,Y
// Reuses WriteAddrAHX (same value/write-address formula as AHX but abs,Y).
static S_TAS_ABSY: &[MicroOp] = &[
    b(B::ReadPC1),
    m(B::ReadPC2, CPU6502::op_tas_setup_addr),
    X_DUMMY,
    b(B::WriteAddrAHX),
];

// AHX/SHA (abs,Y) — store A & X & (base_hi + 1) at abs,Y (same write as (indirect),Y)
// Reuses SHX's abs,Y setup — same page-wrapped address computation.
static S_AHX_ABSY: &[MicroOp] = &[
    b(B::ReadPC1),
    m(B::ReadPC2, CPU6502::op_shx_setup_addr),
    X_DUMMY,
    b(B::WriteAddrAHX),
];

// LAS/LAE (abs,Y) — A = X = SP & memory[abs,Y]
static S_LAS_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_las);

// SBX/AXS (imm) — X = (A & X) - operand; C = no_borrow; N, Z from result
static S_SBX_IMM: &[MicroOp] = &seq_imm(CPU6502::op_sbx);

// ── Unofficial opcodes ──

// SLO (ASL + ORA)
static S_SLO_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_slo);
static S_SLO_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_slo);
static S_SLO_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_slo);
static S_SLO_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_slo);
static S_SLO_ABSY: &[MicroOp] = &rmw_absy(CPU6502::op_slo);
static S_SLO_INDX: &[MicroOp] = &rmw_indx(CPU6502::op_slo);
static S_SLO_INDY: &[MicroOp] = &rmw_indy(CPU6502::op_slo);

// RLA (ROL + AND)
static S_RLA_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_rla);
static S_RLA_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_rla);
static S_RLA_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_rla);
static S_RLA_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_rla);
static S_RLA_ABSY: &[MicroOp] = &rmw_absy(CPU6502::op_rla);
static S_RLA_INDX: &[MicroOp] = &rmw_indx(CPU6502::op_rla);
static S_RLA_INDY: &[MicroOp] = &rmw_indy(CPU6502::op_rla);

// SRE (LSR + EOR)
static S_SRE_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_sre);
static S_SRE_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_sre);
static S_SRE_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_sre);
static S_SRE_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_sre);
static S_SRE_ABSY: &[MicroOp] = &rmw_absy(CPU6502::op_sre);
static S_SRE_INDX: &[MicroOp] = &rmw_indx(CPU6502::op_sre);
static S_SRE_INDY: &[MicroOp] = &rmw_indy(CPU6502::op_sre);

// RRA (ROR + ADC)
static S_RRA_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_rra);
static S_RRA_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_rra);
static S_RRA_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_rra);
static S_RRA_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_rra);
static S_RRA_ABSY: &[MicroOp] = &rmw_absy(CPU6502::op_rra);
static S_RRA_INDX: &[MicroOp] = &rmw_indx(CPU6502::op_rra);
static S_RRA_INDY: &[MicroOp] = &rmw_indy(CPU6502::op_rra);

// DCP (DEC + CMP)
static S_DCP_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_dcp);
static S_DCP_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_dcp);
static S_DCP_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_dcp);
static S_DCP_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_dcp);
static S_DCP_ABSY: &[MicroOp] = &rmw_absy(CPU6502::op_dcp);
static S_DCP_INDX: &[MicroOp] = &rmw_indx(CPU6502::op_dcp);
static S_DCP_INDY: &[MicroOp] = &rmw_indy(CPU6502::op_dcp);

// ISC (INC + SBC)
static S_ISC_ZP: &[MicroOp] = &rmw_zp(CPU6502::op_isc);
static S_ISC_ZPX: &[MicroOp] = &rmw_zpx(CPU6502::op_isc);
static S_ISC_ABS: &[MicroOp] = &rmw_abs(CPU6502::op_isc);
static S_ISC_ABSX: &[MicroOp] = &rmw_absx(CPU6502::op_isc);
static S_ISC_ABSY: &[MicroOp] = &rmw_absy(CPU6502::op_isc);
static S_ISC_INDX: &[MicroOp] = &rmw_indx(CPU6502::op_isc);
static S_ISC_INDY: &[MicroOp] = &rmw_indy(CPU6502::op_isc);

// LAX (LDA + LDX)
static S_LAX_ZP: &[MicroOp] = &seq_zp(CPU6502::op_lax);
static S_LAX_ZPY: &[MicroOp] = &seq_zpy(CPU6502::op_lax);
static S_LAX_ABS: &[MicroOp] = &seq_abs(CPU6502::op_lax);
static S_LAX_ABSY: &[MicroOp] = &seq_absy(CPU6502::op_lax);
static S_LAX_IMM: &[MicroOp] = &seq_imm(CPU6502::op_lax_imm);
static S_LAX_INDX: &[MicroOp] = &seq_indx(CPU6502::op_lax);
static S_LAX_INDY: &[MicroOp] = &[
    m(B::ReadPC1, CPU6502::op_set_addr_zp),
    m(B::ReadAddr, CPU6502::op_save_lo),
    m(B::ReadAddrZp1, CPU6502::op_compute_indy_addr),
    X_DUMMY,
    m(B::ReadAddr, CPU6502::op_lax),
];

// SAX (STA & STX — stores A & X)
static S_SAX_ZP: &[MicroOp] = &[R_ZP, b(B::WriteAddrAX)];
static S_SAX_ZPY: &[MicroOp] = &[b(B::ReadPC1), b(B::ReadDummyZpY), b(B::WriteAddrAX)];
static S_SAX_ABS: &[MicroOp] = &[
    b(B::ReadPC1),
    m(B::ReadPC2, CPU6502::op_set_addr_abs),
    b(B::WriteAddrAX),
];
static S_SAX_INDX: &[MicroOp] = &[
    b(B::ReadPC1),
    b(B::ReadDummyZpX),
    m(B::ReadAddr, CPU6502::op_save_lo),
    m(B::ReadAddrZp1, CPU6502::op_compute_ind_addr),
    b(B::WriteAddrAX),
];

// ── Full opcode table ──

#[rustfmt::skip]
pub static OPCODE_SEQUENCES: [&[MicroOp]; 256] = [
    // 0x00-0x0F
    S_BRK, S_ORA_INDX, S_JAM, S_SLO_INDX, S_NOP_ZP, S_ORA_ZP, S_ASL_ZP, S_SLO_ZP, S_PHP, S_ORA_IMM, S_ASL_A, S_ANC_A, S_NOP_ABS,
    S_ORA_ABS, S_ASL_ABS, S_SLO_ABS,
    // 0x10-0x1F
    S_BPL, S_ORA_INDY, S_JAM, S_SLO_INDY, S_NOP_ZPX, S_ORA_ZPX, S_ASL_ZPX, S_SLO_ZPX, S_CLC, S_ORA_ABSY, S_NOP, S_SLO_ABSY,
    S_NOP_ABSX, S_ORA_ABSX, S_ASL_ABSX, S_SLO_ABSX,
    // 0x20-0x2F
    S_JSR, S_AND_INDX, S_JAM, S_RLA_INDX, S_BIT_ZP, S_AND_ZP, S_ROL_ZP, S_RLA_ZP, S_PLP, S_AND_IMM, S_ROL_A, S_ANC_A,
    S_BIT_ABS, S_AND_ABS, S_ROL_ABS, S_RLA_ABS,
    // 0x30-0x3F
    S_BMI, S_AND_INDY, S_JAM, S_RLA_INDY, S_NOP_ZPX, S_AND_ZPX, S_ROL_ZPX, S_RLA_ZPX, S_SEC, S_AND_ABSY, S_NOP, S_RLA_ABSY,
    S_NOP_ABSX, S_AND_ABSX, S_ROL_ABSX, S_RLA_ABSX,
    // 0x40-0x4F
    S_RTI, S_EOR_INDX, S_JAM, S_SRE_INDX, S_NOP_ZP, S_EOR_ZP, S_LSR_ZP, S_SRE_ZP, S_PHA, S_EOR_IMM, S_LSR_A, S_ALR_IMM,
    S_JMP_ABS, S_EOR_ABS, S_LSR_ABS, S_SRE_ABS,
    // 0x50-0x5F
    S_BVC, S_EOR_INDY, S_JAM, S_SRE_INDY, S_NOP_ZPX, S_EOR_ZPX, S_LSR_ZPX, S_SRE_ZPX, S_CLI, S_EOR_ABSY, S_NOP, S_SRE_ABSY,
    S_NOP_ABSX, S_EOR_ABSX, S_LSR_ABSX, S_SRE_ABSX,
    // 0x60-0x6F
    S_RTS, S_ADC_INDX, S_JAM, S_RRA_INDX, S_NOP_ZP, S_ADC_ZP, S_ROR_ZP, S_RRA_ZP, S_PLA, S_ADC_IMM, S_ROR_A, S_ARR_IMM,
    S_JMP_IND, S_ADC_ABS, S_ROR_ABS, S_RRA_ABS,
    // 0x70-0x7F
    S_BVS, S_ADC_INDY, S_JAM, S_RRA_INDY, S_NOP_ZPX, S_ADC_ZPX, S_ROR_ZPX, S_RRA_ZPX, S_SEI, S_ADC_ABSY, S_NOP, S_RRA_ABSY,
    S_NOP_ABSX, S_ADC_ABSX, S_ROR_ABSX, S_RRA_ABSX,
    // 0x80-0x8F
    S_NOP_IMM, S_STA_INDX, S_NOP_IMM, S_SAX_INDX, S_STY_ZP, S_STA_ZP, S_STX_ZP, S_SAX_ZP, S_DEY, S_NOP_IMM, S_TXA, S_XAA_IMM,
    S_STY_ABS, S_STA_ABS, S_STX_ABS, S_SAX_ABS,
    // 0x90-0x9F
    S_BCC, S_STA_INDY, S_JAM, S_AHX_INDY, S_STY_ZPX, S_STA_ZPX, S_STX_ZPY, S_SAX_ZPY, S_TYA, S_STA_ABSY, S_TXS, S_TAS_ABSY,
    S_SHY_ABSX, S_STA_ABSX, S_SHX_ABSY, S_AHX_ABSY,
    // 0xA0-0xAF
    S_LDY_IMM, S_LDA_INDX, S_LDX_IMM, S_LAX_INDX, S_LDY_ZP, S_LDA_ZP, S_LDX_ZP, S_LAX_ZP, S_TAY, S_LDA_IMM, S_TAX,
    S_LAX_IMM, S_LDY_ABS, S_LDA_ABS, S_LDX_ABS, S_LAX_ABS,
    // 0xB0-0xBF
    S_BCS, S_LDA_INDY, S_JAM, S_LAX_INDY, S_LDY_ZPX, S_LDA_ZPX, S_LDX_ZPY, S_LAX_ZPY, S_CLV, S_LDA_ABSY, S_TSX, S_LAS_ABSY,
    S_LDY_ABSX, S_LDA_ABSX, S_LDX_ABSY, S_LAX_ABSY,
    // 0xC0-0xCF
    S_CPY_IMM, S_CMP_INDX, S_NOP_IMM, S_DCP_INDX, S_CPY_ZP, S_CMP_ZP, S_DEC_ZP, S_DCP_ZP, S_INY, S_CMP_IMM, S_DEX, S_SBX_IMM,
    S_CPY_ABS, S_CMP_ABS, S_DEC_ABS, S_DCP_ABS,
    // 0xD0-0xDF
    S_BNE, S_CMP_INDY, S_JAM, S_DCP_INDY, S_NOP_ZPX, S_CMP_ZPX, S_DEC_ZPX, S_DCP_ZPX, S_CLD, S_CMP_ABSY, S_NOP, S_DCP_ABSY,
    S_NOP_ABSX, S_CMP_ABSX, S_DEC_ABSX, S_DCP_ABSX,
    // 0xE0-0xEF
    S_CPX_IMM, S_SBC_INDX, S_NOP_IMM, S_ISC_INDX, S_CPX_ZP, S_SBC_ZP, S_INC_ZP, S_ISC_ZP, S_INX, S_SBC_IMM, S_NOP, S_SBC_IMM,
    S_CPX_ABS, S_SBC_ABS, S_INC_ABS, S_ISC_ABS,
    // 0xF0-0xFF
    S_BEQ, S_SBC_INDY, S_JAM, S_ISC_INDY, S_NOP_ZPX, S_SBC_ZPX, S_INC_ZPX, S_ISC_ZPX, S_SED, S_SBC_ABSY, S_NOP, S_ISC_ABSY,
    S_NOP_ABSX, S_SBC_ABSX, S_INC_ABSX, S_ISC_ABSX,
];

// ── Interrupt sequences ──

interrupt_seq!(INTERRUPT_SEQ_NMI, 0xFFFA, 0xFFFB);
interrupt_seq!(INTERRUPT_SEQ_IRQ, 0xFFFE, 0xFFFF);

pub static INTERRUPT_SEQ_RESET: &[MicroOp] = &[
    b(B::ReadDummy),
    b(B::ReadDummy),
    b(B::ReadDummy),
    b(B::PopDummy),
    b(B::PopDummy),
    b(B::ReadVecLo(0xFFFC)),
    b(B::ReadVecHi(0xFFFD)),
];
