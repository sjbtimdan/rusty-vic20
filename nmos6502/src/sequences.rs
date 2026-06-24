//! Static instruction sequences — one per opcode, indexed by opcode byte.
//!
//! Each sequence represents cycles 2+ of an instruction. Cycle 1 (opcode
//! fetch) is handled by `CPU6502::step()` when no sequence is active.
//!
//! Naming: `BusOp::None` = no bus access. `InternalOp::None` = no internal work.

use crate::micro_op::{BusOp, InternalOp, MicroOp};

// Shorthand
use BusOp as B;
use InternalOp as I;

const fn m(bus: B, internal: I) -> MicroOp {
    MicroOp { bus, internal }
}

const fn i(internal: I) -> MicroOp {
    MicroOp { bus: BN, internal }
}

const fn b(bus: B) -> MicroOp {
    MicroOp { bus, internal: N }
}

// Common constants
const N: I = I::None;
const BN: B = B::None;
const NONE: MicroOp = m(BN, N);

// ── Commmon sub-sequences ──

const R_ZP: MicroOp = m(B::ReadPC1, I::SetAddrZp);

const C_ABSX: MicroOp = m(B::ReadPC2, I::SetAddrAbsX);
const C_ABSY: MicroOp = m(B::ReadPC2, I::SetAddrAbsY);

const X_DUMMY: MicroOp = m(B::ReadDummy, N);
const S_NC1: MicroOp = m(BN, I::SkipIfNotCrossed(1));

// ── Sequences ──

// Implied/register
static S_NOP: &[MicroOp] = &[NONE];
static S_INX: &[MicroOp] = &[i(I::IncX)];
static S_INY: &[MicroOp] = &[i(I::IncY)];
static S_DEX: &[MicroOp] = &[i(I::DecX)];
static S_DEY: &[MicroOp] = &[i(I::DecY)];
static S_TXA: &[MicroOp] = &[i(I::Txa)];
static S_TYA: &[MicroOp] = &[i(I::Tya)];
static S_TAX: &[MicroOp] = &[i(I::Tax)];
static S_TAY: &[MicroOp] = &[i(I::Tay)];
static S_TSX: &[MicroOp] = &[i(I::Tsx)];
static S_TXS: &[MicroOp] = &[i(I::Txs)];
static S_CLC: &[MicroOp] = &[i(I::ClrC)];
static S_SEC: &[MicroOp] = &[i(I::SetC)];
static S_CLD: &[MicroOp] = &[i(I::ClrD)];
static S_SED: &[MicroOp] = &[i(I::SetD)];
static S_CLI: &[MicroOp] = &[i(I::ClrI)];
static S_SEI: &[MicroOp] = &[i(I::SetI)];
static S_CLV: &[MicroOp] = &[i(I::ClrV)];

// Stack
static S_PHA: &[MicroOp] = &[NONE, m(B::PushA, N)];
static S_PHP: &[MicroOp] = &[NONE, m(B::PushStatusB, N)];
static S_PLA: &[MicroOp] = &[NONE, m(B::PopDummy, N), m(B::Pop, I::SetA)];
static S_PLP: &[MicroOp] = &[NONE, m(B::PopDummy, N), m(B::Pop, I::SetStatus)];

// Immediate
static S_LDA_IMM: &[MicroOp] = &[m(B::ReadPC1, I::SetA)];
static S_LDX_IMM: &[MicroOp] = &[m(B::ReadPC1, I::SetX)];
static S_LDY_IMM: &[MicroOp] = &[m(B::ReadPC1, I::SetY)];
static S_ADC_IMM: &[MicroOp] = &[m(B::ReadPC1, I::Adc)];
static S_SBC_IMM: &[MicroOp] = &[m(B::ReadPC1, I::Sbc)];
static S_AND_IMM: &[MicroOp] = &[m(B::ReadPC1, I::And)];
static S_ORA_IMM: &[MicroOp] = &[m(B::ReadPC1, I::Ora)];
static S_EOR_IMM: &[MicroOp] = &[m(B::ReadPC1, I::Eor)];
static S_CMP_IMM: &[MicroOp] = &[m(B::ReadPC1, I::CmpA)];
static S_CPX_IMM: &[MicroOp] = &[m(B::ReadPC1, I::CmpX)];
static S_CPY_IMM: &[MicroOp] = &[m(B::ReadPC1, I::CmpY)];

// Zero page read
static S_LDA_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::SetA)];
static S_LDX_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::SetX)];
static S_LDY_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::SetY)];
static S_ADC_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::Adc)];
static S_SBC_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::Sbc)];
static S_AND_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::And)];
static S_ORA_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::Ora)];
static S_EOR_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::Eor)];
static S_CMP_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::CmpA)];
static S_CPX_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::CmpX)];
static S_CPY_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::CmpY)];
static S_BIT_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::Bit)];

// Zero page write
static S_STA_ZP: &[MicroOp] = &[R_ZP, b(B::WriteAddrA)];
static S_STX_ZP: &[MicroOp] = &[R_ZP, b(B::WriteAddrX)];
static S_STY_ZP: &[MicroOp] = &[R_ZP, b(B::WriteAddrY)];

// Zero page indexed X read
static S_LDA_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::ReadAddr, I::SetA)];
static S_LDY_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::ReadAddr, I::SetY)];
static S_ADC_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::ReadAddr, I::Adc)];
static S_SBC_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::ReadAddr, I::Sbc)];
static S_AND_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::ReadAddr, I::And)];
static S_ORA_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::ReadAddr, I::Ora)];
static S_EOR_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::ReadAddr, I::Eor)];
static S_CMP_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::ReadAddr, I::CmpA)];

// Zero page indexed X write
static S_STA_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::WriteAddrA, N)];
static S_STY_ZPX: &[MicroOp] = &[m(B::ReadPC1, N), b(B::ReadDummyZpX), m(B::WriteAddrY, N)];

// Zero page indexed Y
static S_LDX_ZPY: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadDummyZpY, N), m(B::ReadAddr, I::SetX)];
static S_STX_ZPY: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadDummyZpY, N), m(B::WriteAddrX, N)];

// Absolute read
static S_LDA_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::SetA)];
static S_LDX_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::SetX)];
static S_LDY_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::SetY)];
static S_ADC_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::Adc)];
static S_SBC_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::Sbc)];
static S_AND_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::And)];
static S_ORA_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::Ora)];
static S_EOR_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::Eor)];
static S_CMP_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::CmpA)];
static S_CPX_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::CmpX)];
static S_CPY_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::CmpY)];
static S_BIT_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::Bit)];

// Absolute write
static S_STA_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::WriteAddrA, N)];
static S_STX_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::WriteAddrX, N)];
static S_STY_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::WriteAddrY, N)];

// JMP
static S_JMP_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), i(I::JumpAbs)];
static S_JMP_IND: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), i(I::JumpInd), NONE];

// Absolute indexed X read (with page-cross handling)
static S_LDA_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, S_NC1, X_DUMMY, m(B::ReadAddr, I::SetA)];
static S_LDY_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, S_NC1, X_DUMMY, m(B::ReadAddr, I::SetY)];
static S_ADC_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, S_NC1, X_DUMMY, m(B::ReadAddr, I::Adc)];
static S_SBC_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, S_NC1, X_DUMMY, m(B::ReadAddr, I::Sbc)];
static S_AND_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, S_NC1, X_DUMMY, m(B::ReadAddr, I::And)];
static S_ORA_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, S_NC1, X_DUMMY, m(B::ReadAddr, I::Ora)];
static S_EOR_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, S_NC1, X_DUMMY, m(B::ReadAddr, I::Eor)];
static S_CMP_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, S_NC1, X_DUMMY, m(B::ReadAddr, I::CmpA)];

// Absolute indexed Y read
static S_LDA_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::SetA)];
static S_LDX_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::SetX)];
static S_ADC_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::Adc)];
static S_SBC_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::Sbc)];
static S_AND_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::And)];
static S_ORA_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::Ora)];
static S_EOR_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::Eor)];
static S_CMP_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::CmpA)];

// Absolute indexed write (always 5 cycles)
static S_STA_ABSX: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSX, X_DUMMY, m(B::WriteAddrA, N)];
static S_STA_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, X_DUMMY, m(B::WriteAddrA, N)];

// RMW zero page
static S_ASL_ZP: &[MicroOp] = &[
    R_ZP,
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Asl),
    m(B::WriteAddrDL, N),
];
static S_LSR_ZP: &[MicroOp] = &[
    R_ZP,
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Lsr),
    m(B::WriteAddrDL, N),
];
static S_ROL_ZP: &[MicroOp] = &[
    R_ZP,
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Rol),
    m(B::WriteAddrDL, N),
];
static S_ROR_ZP: &[MicroOp] = &[
    R_ZP,
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Ror),
    m(B::WriteAddrDL, N),
];
static S_INC_ZP: &[MicroOp] = &[
    R_ZP,
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Inc),
    m(B::WriteAddrDL, N),
];
static S_DEC_ZP: &[MicroOp] = &[
    R_ZP,
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Dec),
    m(B::WriteAddrDL, N),
];

// RMW accumulator
static S_ASL_A: &[MicroOp] = &[i(I::AslA)];
static S_LSR_A: &[MicroOp] = &[i(I::LsrA)];
static S_ROL_A: &[MicroOp] = &[i(I::RolA)];
static S_ROR_A: &[MicroOp] = &[i(I::RorA)];

// RMW absolute
static S_ASL_ABS: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(B::ReadPC2, I::SetAddrAbs),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Asl),
    m(B::WriteAddrDL, N),
];
static S_LSR_ABS: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(B::ReadPC2, I::SetAddrAbs),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Lsr),
    m(B::WriteAddrDL, N),
];
static S_ROL_ABS: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(B::ReadPC2, I::SetAddrAbs),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Rol),
    m(B::WriteAddrDL, N),
];
static S_ROR_ABS: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(B::ReadPC2, I::SetAddrAbs),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Ror),
    m(B::WriteAddrDL, N),
];
static S_INC_ABS: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(B::ReadPC2, I::SetAddrAbs),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Inc),
    m(B::WriteAddrDL, N),
];
static S_DEC_ABS: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(B::ReadPC2, I::SetAddrAbs),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Dec),
    m(B::WriteAddrDL, N),
];

// RMW absolute indexed X (always 7 cycles)
static S_ASL_ABSX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    C_ABSX,
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Asl),
    m(B::WriteAddrDL, N),
];
static S_LSR_ABSX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    C_ABSX,
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Lsr),
    m(B::WriteAddrDL, N),
];
static S_ROL_ABSX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    C_ABSX,
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Rol),
    m(B::WriteAddrDL, N),
];
static S_ROR_ABSX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    C_ABSX,
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Ror),
    m(B::WriteAddrDL, N),
];
static S_INC_ABSX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    C_ABSX,
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Inc),
    m(B::WriteAddrDL, N),
];
static S_DEC_ABSX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    C_ABSX,
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Dec),
    m(B::WriteAddrDL, N),
];

// RMW zero page indexed X (6 cycles)
static S_ASL_ZPX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Asl),
    m(B::WriteAddrDL, N),
];
static S_LSR_ZPX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Lsr),
    m(B::WriteAddrDL, N),
];
static S_ROL_ZPX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Rol),
    m(B::WriteAddrDL, N),
];
static S_ROR_ZPX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Ror),
    m(B::WriteAddrDL, N),
];
static S_INC_ZPX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Inc),
    m(B::WriteAddrDL, N),
];
static S_DEC_ZPX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(B::ReadAddr, N),
    m(B::WriteDummy, N),
    m(BN, I::Dec),
    m(B::WriteAddrDL, N),
];

// Indexed indirect (zp,X) read
static S_ORA_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::ReadAddr, I::Ora),
];
static S_AND_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::ReadAddr, I::And),
];
static S_EOR_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::ReadAddr, I::Eor),
];
static S_ADC_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::ReadAddr, I::Adc),
];
static S_SBC_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::ReadAddr, I::Sbc),
];
static S_CMP_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::ReadAddr, I::CmpA),
];
static S_LDA_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::ReadAddr, I::SetA),
];
static S_STA_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::WriteAddrA, N),
];

// Branches
macro_rules! branch_seq {
    ($cond:ident) => {
        &[
            m(BN, I::$cond),
            m(BN, I::SkipIfNotTaken(2)),
            m(BN, I::SkipIfNotCrossed(1)),
            m(B::ReadDummy, N),
        ]
    };
}
static S_BCC: &[MicroOp] = branch_seq!(BranchCC);
static S_BCS: &[MicroOp] = branch_seq!(BranchCS);
static S_BEQ: &[MicroOp] = branch_seq!(BranchEQ);
static S_BNE: &[MicroOp] = branch_seq!(BranchNE);
static S_BMI: &[MicroOp] = branch_seq!(BranchMI);
static S_BPL: &[MicroOp] = branch_seq!(BranchPL);
static S_BVC: &[MicroOp] = branch_seq!(BranchVC);
static S_BVS: &[MicroOp] = branch_seq!(BranchVS);

// JSR / RTS / RTI / BRK
static S_JSR: &[MicroOp] = &[
    m(B::ReadPC1, N),
    NONE,
    m(B::PushReturnHi, N),
    m(B::PushReturnLo, N),
    m(B::ReadPC2, I::JsrC6),
];
static S_RTS: &[MicroOp] = &[NONE, m(B::PopDummy, N), m(B::PopPCL, N), m(B::PopPCH, I::RtsFinish)];
static S_RTI: &[MicroOp] = &[
    NONE,
    m(B::PopDummy, N),
    m(B::Pop, I::SetStatus),
    m(B::PopPCL, N),
    m(B::PopPCH, I::RtiFinish),
];
static S_BRK: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(B::PushReturnHi, N),
    m(B::PushReturnLo, N),
    m(B::PushStatusB, I::SetI),
    m(B::ReadVecLo(0xFFFE), N),
    m(B::ReadVecHi(0xFFFF), N),
];

// Indirect indexed (zp),Y read — 5 cycles, +1 if page cross
// Note: SetAddrIndY performs both zp pointer reads internally (simplification).
static S_ORA_INDY: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(BN, I::SetAddrIndY),
    S_NC1,
    X_DUMMY,
    m(B::ReadAddr, I::Ora),
];
static S_AND_INDY: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(BN, I::SetAddrIndY),
    S_NC1,
    X_DUMMY,
    m(B::ReadAddr, I::And),
];
static S_EOR_INDY: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(BN, I::SetAddrIndY),
    S_NC1,
    X_DUMMY,
    m(B::ReadAddr, I::Eor),
];
static S_ADC_INDY: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(BN, I::SetAddrIndY),
    S_NC1,
    X_DUMMY,
    m(B::ReadAddr, I::Adc),
];
static S_SBC_INDY: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(BN, I::SetAddrIndY),
    S_NC1,
    X_DUMMY,
    m(B::ReadAddr, I::Sbc),
];
static S_CMP_INDY: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(BN, I::SetAddrIndY),
    S_NC1,
    X_DUMMY,
    m(B::ReadAddr, I::CmpA),
];
static S_LDA_INDY: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(BN, I::SetAddrIndY),
    S_NC1,
    X_DUMMY,
    m(B::ReadAddr, I::SetA),
];

// Indirect indexed (zp),Y write — 6 cycles (always, extra dummy read)
static S_STA_INDY: &[MicroOp] = &[m(B::ReadPC1, N), m(BN, I::SetAddrIndY), X_DUMMY, m(B::WriteAddrA, N)];

// ── Unofficial opcodes ──

// RMW+ALU helpers (read-modify-write then combine with A)
macro_rules! rmw_zp {
    ($alu:ident) => {
        &[
            R_ZP,
            m(B::ReadAddr, N),
            m(B::WriteDummy, N),
            m(BN, I::$alu),
            m(B::WriteAddrDL, N),
        ]
    };
}
macro_rules! rmw_zpx {
    ($alu:ident) => {
        &[
            m(B::ReadPC1, N),
            b(B::ReadDummyZpX),
            m(B::ReadAddr, N),
            m(B::WriteDummy, N),
            m(BN, I::$alu),
            m(B::WriteAddrDL, N),
        ]
    };
}
macro_rules! rmw_abs {
    ($alu:ident) => {
        &[
            m(B::ReadPC1, N),
            m(B::ReadPC2, I::SetAddrAbs),
            m(B::ReadAddr, N),
            m(B::WriteDummy, N),
            m(BN, I::$alu),
            m(B::WriteAddrDL, N),
        ]
    };
}
macro_rules! rmw_absx {
    ($alu:ident) => {
        &[
            m(B::ReadPC1, N),
            C_ABSX,
            m(B::ReadAddr, N),
            m(B::ReadAddr, N),
            m(B::WriteDummy, N),
            m(BN, I::$alu),
            m(B::WriteAddrDL, N),
        ]
    };
}
macro_rules! rmw_absy {
    ($alu:ident) => {
        &[
            m(B::ReadPC1, N),
            C_ABSY,
            m(B::ReadAddr, N),
            m(B::ReadAddr, N),
            m(B::WriteDummy, N),
            m(BN, I::$alu),
            m(B::WriteAddrDL, N),
        ]
    };
}
macro_rules! rmw_indx {
    ($alu:ident) => {
        &[
            m(B::ReadPC1, N),
            b(B::ReadDummyZpX),
            m(BN, I::SetAddrIndX),
            m(B::ReadAddr, N),
            m(B::ReadAddr, N),
            m(B::WriteDummy, N),
            m(BN, I::$alu),
            m(B::WriteAddrDL, N),
        ]
    };
}
macro_rules! rmw_indy {
    ($alu:ident) => {
        &[
            m(B::ReadPC1, N),
            m(BN, I::SetAddrIndY),
            m(B::ReadAddr, N),
            m(B::ReadAddr, N),
            m(B::WriteDummy, N),
            m(BN, I::$alu),
            m(B::WriteAddrDL, N),
        ]
    };
}

// SLO (ASL + ORA)
static S_SLO_ZP: &[MicroOp] = rmw_zp!(Slo);
static S_SLO_ZPX: &[MicroOp] = rmw_zpx!(Slo);
static S_SLO_ABS: &[MicroOp] = rmw_abs!(Slo);
static S_SLO_ABSX: &[MicroOp] = rmw_absx!(Slo);
static S_SLO_ABSY: &[MicroOp] = rmw_absy!(Slo);
static S_SLO_INDX: &[MicroOp] = rmw_indx!(Slo);
static S_SLO_INDY: &[MicroOp] = rmw_indy!(Slo);

// RLA (ROL + AND)
static S_RLA_ZP: &[MicroOp] = rmw_zp!(Rla);
static S_RLA_ZPX: &[MicroOp] = rmw_zpx!(Rla);
static S_RLA_ABS: &[MicroOp] = rmw_abs!(Rla);
static S_RLA_ABSX: &[MicroOp] = rmw_absx!(Rla);
static S_RLA_ABSY: &[MicroOp] = rmw_absy!(Rla);
static S_RLA_INDX: &[MicroOp] = rmw_indx!(Rla);
static S_RLA_INDY: &[MicroOp] = rmw_indy!(Rla);

// SRE (LSR + EOR)
static S_SRE_ZP: &[MicroOp] = rmw_zp!(Sre);
static S_SRE_ZPX: &[MicroOp] = rmw_zpx!(Sre);
static S_SRE_ABS: &[MicroOp] = rmw_abs!(Sre);
static S_SRE_ABSX: &[MicroOp] = rmw_absx!(Sre);
static S_SRE_ABSY: &[MicroOp] = rmw_absy!(Sre);
static S_SRE_INDX: &[MicroOp] = rmw_indx!(Sre);
static S_SRE_INDY: &[MicroOp] = rmw_indy!(Sre);

// RRA (ROR + ADC)
static S_RRA_ZP: &[MicroOp] = rmw_zp!(Rra);
static S_RRA_ZPX: &[MicroOp] = rmw_zpx!(Rra);
static S_RRA_ABS: &[MicroOp] = rmw_abs!(Rra);
static S_RRA_ABSX: &[MicroOp] = rmw_absx!(Rra);
static S_RRA_ABSY: &[MicroOp] = rmw_absy!(Rra);
static S_RRA_INDX: &[MicroOp] = rmw_indx!(Rra);
static S_RRA_INDY: &[MicroOp] = rmw_indy!(Rra);

// DCP (DEC + CMP)
static S_DCP_ZP: &[MicroOp] = rmw_zp!(Dcp);
static S_DCP_ZPX: &[MicroOp] = rmw_zpx!(Dcp);
static S_DCP_ABS: &[MicroOp] = rmw_abs!(Dcp);
static S_DCP_ABSX: &[MicroOp] = rmw_absx!(Dcp);
static S_DCP_ABSY: &[MicroOp] = rmw_absy!(Dcp);
static S_DCP_INDX: &[MicroOp] = rmw_indx!(Dcp);
static S_DCP_INDY: &[MicroOp] = rmw_indy!(Dcp);

// ISC (INC + SBC)
static S_ISC_ZP: &[MicroOp] = rmw_zp!(Isc);
static S_ISC_ZPX: &[MicroOp] = rmw_zpx!(Isc);
static S_ISC_ABS: &[MicroOp] = rmw_abs!(Isc);
static S_ISC_ABSX: &[MicroOp] = rmw_absx!(Isc);
static S_ISC_ABSY: &[MicroOp] = rmw_absy!(Isc);
static S_ISC_INDX: &[MicroOp] = rmw_indx!(Isc);
static S_ISC_INDY: &[MicroOp] = rmw_indy!(Isc);

// LAX (LDA + LDX)
static S_LAX_ZP: &[MicroOp] = &[R_ZP, m(B::ReadAddr, I::Lax)];
static S_LAX_ZPY: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadDummyZpY, N), m(B::ReadAddr, I::Lax)];
static S_LAX_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::ReadAddr, I::Lax)];
static S_LAX_ABSY: &[MicroOp] = &[m(B::ReadPC1, N), C_ABSY, S_NC1, X_DUMMY, m(B::ReadAddr, I::Lax)];
static S_LAX_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::ReadAddr, I::Lax),
];
static S_LAX_INDY: &[MicroOp] = &[
    m(B::ReadPC1, N),
    m(BN, I::SetAddrIndY),
    S_NC1,
    X_DUMMY,
    m(B::ReadAddr, I::Lax),
];

// SAX (STA & STX — stores A & X)
static S_SAX_ZP: &[MicroOp] = &[R_ZP, m(B::WriteAddrAX, N)];
static S_SAX_ZPY: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadDummyZpY, N), m(B::WriteAddrAX, N)];
static S_SAX_ABS: &[MicroOp] = &[m(B::ReadPC1, N), m(B::ReadPC2, I::SetAddrAbs), m(B::WriteAddrAX, N)];
static S_SAX_INDX: &[MicroOp] = &[
    m(B::ReadPC1, N),
    b(B::ReadDummyZpX),
    m(BN, I::SetAddrIndX),
    m(B::ReadAddr, N),
    m(B::ReadAddr, N),
    m(B::WriteAddrAX, N),
];

// ── Full opcode table ──

const EMPTY: &[MicroOp] = &[];

pub static OPCODE_SEQUENCES: [&[MicroOp]; 256] = [
    // 0x00-0x0F
    S_BRK, S_ORA_INDX, EMPTY, S_SLO_INDX, EMPTY, S_ORA_ZP, S_ASL_ZP, S_SLO_ZP, S_PHP, S_ORA_IMM, S_ASL_A, EMPTY, EMPTY,
    S_ORA_ABS, S_ASL_ABS, S_SLO_ABS, // 0x10-0x1F
    S_BPL, S_ORA_INDY, EMPTY, S_SLO_INDY, EMPTY, S_ORA_ZPX, S_ASL_ZPX, S_SLO_ZPX, S_CLC, S_ORA_ABSY, EMPTY, S_SLO_ABSY,
    EMPTY, S_ORA_ABSX, S_ASL_ABSX, S_SLO_ABSX, // 0x20-0x2F
    S_JSR, S_AND_INDX, EMPTY, S_RLA_INDX, S_BIT_ZP, S_AND_ZP, S_ROL_ZP, S_RLA_ZP, S_PLP, S_AND_IMM, S_ROL_A, EMPTY,
    S_BIT_ABS, S_AND_ABS, S_ROL_ABS, S_RLA_ABS, // 0x30-0x3F
    S_BMI, S_AND_INDY, EMPTY, S_RLA_INDY, EMPTY, S_AND_ZPX, S_ROL_ZPX, S_RLA_ZPX, S_SEC, S_AND_ABSY, EMPTY, S_RLA_ABSY,
    EMPTY, S_AND_ABSX, S_ROL_ABSX, S_RLA_ABSX, // 0x40-0x4F
    S_RTI, S_EOR_INDX, EMPTY, S_SRE_INDX, EMPTY, S_EOR_ZP, S_LSR_ZP, S_SRE_ZP, S_PHA, S_EOR_IMM, S_LSR_A, EMPTY,
    S_JMP_ABS, S_EOR_ABS, S_LSR_ABS, S_SRE_ABS, // 0x50-0x5F
    S_BVC, S_EOR_INDY, EMPTY, S_SRE_INDY, EMPTY, S_EOR_ZPX, S_LSR_ZPX, S_SRE_ZPX, S_CLI, S_EOR_ABSY, EMPTY, S_SRE_ABSY,
    EMPTY, S_EOR_ABSX, S_LSR_ABSX, S_SRE_ABSX, // 0x60-0x6F
    S_RTS, S_ADC_INDX, EMPTY, S_RRA_INDX, EMPTY, S_ADC_ZP, S_ROR_ZP, S_RRA_ZP, S_PLA, S_ADC_IMM, S_ROR_A, EMPTY,
    S_JMP_IND, S_ADC_ABS, S_ROR_ABS, S_RRA_ABS, // 0x70-0x7F
    S_BVS, S_ADC_INDY, EMPTY, S_RRA_INDY, EMPTY, S_ADC_ZPX, S_ROR_ZPX, S_RRA_ZPX, S_SEI, S_ADC_ABSY, EMPTY, S_RRA_ABSY,
    EMPTY, S_ADC_ABSX, S_ROR_ABSX, S_RRA_ABSX, // 0x80-0x8F
    EMPTY, S_STA_INDX, EMPTY, S_SAX_INDX, S_STY_ZP, S_STA_ZP, S_STX_ZP, S_SAX_ZP, S_DEY, EMPTY, S_TXA, EMPTY,
    S_STY_ABS, S_STA_ABS, S_STX_ABS, S_SAX_ABS, // 0x90-0x9F
    S_BCC, S_STA_INDY, EMPTY, EMPTY, S_STY_ZPX, S_STA_ZPX, S_STX_ZPY, S_SAX_ZPY, S_TYA, S_STA_ABSY, S_TXS, EMPTY,
    EMPTY, S_STA_ABSX, EMPTY, EMPTY, // 0xA0-0xAF
    S_LDY_IMM, S_LDA_INDX, S_LDX_IMM, S_LAX_INDX, S_LDY_ZP, S_LDA_ZP, S_LDX_ZP, S_LAX_ZP, S_TAY, S_LDA_IMM, S_TAX,
    EMPTY, S_LDY_ABS, S_LDA_ABS, S_LDX_ABS, S_LAX_ABS, // 0xB0-0xBF
    S_BCS, S_LDA_INDY, EMPTY, S_LAX_INDY, S_LDY_ZPX, S_LDA_ZPX, S_LDX_ZPY, S_LAX_ZPY, S_CLV, S_LDA_ABSY, S_TSX, EMPTY,
    S_LDY_ABSX, S_LDA_ABSX, S_LDX_ABSY, S_LAX_ABSY, // 0xC0-0xCF
    S_CPY_IMM, S_CMP_INDX, EMPTY, S_DCP_INDX, S_CPY_ZP, S_CMP_ZP, S_DEC_ZP, S_DCP_ZP, S_INY, S_CMP_IMM, S_DEX, EMPTY,
    S_CPY_ABS, S_CMP_ABS, S_DEC_ABS, S_DCP_ABS, // 0xD0-0xDF
    S_BNE, S_CMP_INDY, EMPTY, S_DCP_INDY, EMPTY, S_CMP_ZPX, S_DEC_ZPX, S_DCP_ZPX, S_CLD, S_CMP_ABSY, EMPTY, S_DCP_ABSY,
    EMPTY, S_CMP_ABSX, S_DEC_ABSX, S_DCP_ABSX, // 0xE0-0xEF
    S_CPX_IMM, S_SBC_INDX, EMPTY, S_ISC_INDX, S_CPX_ZP, S_SBC_ZP, S_INC_ZP, S_ISC_ZP, S_INX, S_SBC_IMM, S_NOP, EMPTY,
    S_CPX_ABS, S_SBC_ABS, S_INC_ABS, S_ISC_ABS, // 0xF0-0xFF
    S_BEQ, S_SBC_INDY, EMPTY, S_ISC_INDY, EMPTY, S_SBC_ZPX, S_INC_ZPX, S_ISC_ZPX, S_SED, S_SBC_ABSY, EMPTY, S_ISC_ABSY,
    EMPTY, S_SBC_ABSX, S_INC_ABSX, S_ISC_ABSX,
];

// ── Interrupt sequences ──

pub static INTERRUPT_SEQ_NMI: &[MicroOp] = &[
    NONE,
    NONE,
    m(B::PushPCH, N),
    m(B::PushPCL, N),
    m(B::PushStatus, I::SetI),
    m(B::ReadVecLo(0xFFFA), N),
    m(B::ReadVecHi(0xFFFB), N),
];

pub static INTERRUPT_SEQ_IRQ: &[MicroOp] = &[
    NONE,
    NONE,
    m(B::PushPCH, N),
    m(B::PushPCL, N),
    m(B::PushStatus, I::SetI),
    m(B::ReadVecLo(0xFFFE), N),
    m(B::ReadVecHi(0xFFFF), N),
];

pub static INTERRUPT_SEQ_RESET: &[MicroOp] = &[
    NONE,
    NONE,
    NONE,
    m(B::PopDummy, N),
    m(B::PopDummy, N),
    m(B::ReadVecLo(0xFFFC), N),
    m(B::ReadVecHi(0xFFFD), N),
];
