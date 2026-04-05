#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    ADC,
    AND,
    ASL,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    JMP,
    JSR,
    LDA,
    LDX,
    LDY,
    LSR,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    ROL,
    ROR,
    RTI,
    RTS,
    SBC,
    SEC,
    SED,
    SEI,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
    Illegal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionInfo {
    pub opcode: u8,
    pub instruction: Instruction,
    pub mode: AddressingMode,
    pub cycles: u8,
}

const fn info(opcode: u8, instruction: Instruction, mode: AddressingMode, cycles: u8) -> InstructionInfo {
    InstructionInfo {
        opcode,
        instruction,
        mode,
        cycles,
    }
}

const fn illegal(opcode: u8) -> InstructionInfo {
    info(opcode, Instruction::Illegal, AddressingMode::Implied, 0)
}

pub const LDA_IMMEDIATE: InstructionInfo = info(0xA9, Instruction::LDA, AddressingMode::Immediate, 2);
pub const LDA_ZERO_PAGE: InstructionInfo = info(0xA5, Instruction::LDA, AddressingMode::ZeroPage, 3);
pub const LDA_ZERO_PAGE_X: InstructionInfo = info(0xB5, Instruction::LDA, AddressingMode::ZeroPageX, 4);
pub const LDA_ABSOLUTE: InstructionInfo = info(0xAD, Instruction::LDA, AddressingMode::Absolute, 4);
pub const LDA_ABSOLUTE_X: InstructionInfo = info(0xBD, Instruction::LDA, AddressingMode::AbsoluteX, 4);
pub const LDA_ABSOLUTE_Y: InstructionInfo = info(0xB9, Instruction::LDA, AddressingMode::AbsoluteY, 4);
pub const LDA_INDEXED_INDIRECT: InstructionInfo = info(0xA1, Instruction::LDA, AddressingMode::IndexedIndirect, 6);
pub const LDA_INDIRECT_INDEXED: InstructionInfo = info(0xB1, Instruction::LDA, AddressingMode::IndirectIndexed, 5);

pub const LDX_IMMEDIATE: InstructionInfo = info(0xA2, Instruction::LDX, AddressingMode::Immediate, 2);
pub const LDX_ZERO_PAGE: InstructionInfo = info(0xA6, Instruction::LDX, AddressingMode::ZeroPage, 3);
pub const LDX_ZERO_PAGE_Y: InstructionInfo = info(0xB6, Instruction::LDX, AddressingMode::ZeroPageY, 4);
pub const LDX_ABSOLUTE: InstructionInfo = info(0xAE, Instruction::LDX, AddressingMode::Absolute, 4);
pub const LDX_ABSOLUTE_Y: InstructionInfo = info(0xBE, Instruction::LDX, AddressingMode::AbsoluteY, 4);

pub const DEX_IMPLIED: InstructionInfo = info(0xCA, Instruction::DEX, AddressingMode::Implied, 2);
pub const DEY_IMPLIED: InstructionInfo = info(0x88, Instruction::DEY, AddressingMode::Implied, 2);
pub const INX_IMPLIED: InstructionInfo = info(0xE8, Instruction::INX, AddressingMode::Implied, 2);
pub const INY_IMPLIED: InstructionInfo = info(0xC8, Instruction::INY, AddressingMode::Implied, 2);

pub const INC_ZERO_PAGE: InstructionInfo = info(0xE6, Instruction::INC, AddressingMode::ZeroPage, 5);
pub const INC_ZERO_PAGE_X: InstructionInfo = info(0xF6, Instruction::INC, AddressingMode::ZeroPageX, 6);
pub const INC_ABSOLUTE: InstructionInfo = info(0xEE, Instruction::INC, AddressingMode::Absolute, 6);
pub const INC_ABSOLUTE_X: InstructionInfo = info(0xFE, Instruction::INC, AddressingMode::AbsoluteX, 7);

pub const DEC_ZERO_PAGE: InstructionInfo = info(0xC6, Instruction::DEC, AddressingMode::ZeroPage, 5);
pub const DEC_ZERO_PAGE_X: InstructionInfo = info(0xD6, Instruction::DEC, AddressingMode::ZeroPageX, 6);
pub const DEC_ABSOLUTE: InstructionInfo = info(0xCE, Instruction::DEC, AddressingMode::Absolute, 6);
pub const DEC_ABSOLUTE_X: InstructionInfo = info(0xDE, Instruction::DEC, AddressingMode::AbsoluteX, 7);

pub const LDY_IMMEDIATE: InstructionInfo = info(0xA0, Instruction::LDY, AddressingMode::Immediate, 2);
pub const LDY_ZERO_PAGE: InstructionInfo = info(0xA4, Instruction::LDY, AddressingMode::ZeroPage, 3);
pub const LDY_ZERO_PAGE_X: InstructionInfo = info(0xB4, Instruction::LDY, AddressingMode::ZeroPageX, 4);
pub const LDY_ABSOLUTE: InstructionInfo = info(0xAC, Instruction::LDY, AddressingMode::Absolute, 4);
pub const LDY_ABSOLUTE_X: InstructionInfo = info(0xBC, Instruction::LDY, AddressingMode::AbsoluteX, 4);

// Base cycle counts are provided here. Some instructions (branches/page crossings)
// can take extra cycles depending on runtime conditions.
pub const fn decode(opcode: u8) -> InstructionInfo {
    match opcode {
        0x00 => info(0x00, Instruction::BRK, AddressingMode::Implied, 7),
        0x01 => info(0x01, Instruction::ORA, AddressingMode::IndexedIndirect, 6),
        0x05 => info(0x05, Instruction::ORA, AddressingMode::ZeroPage, 3),
        0x06 => info(0x06, Instruction::ASL, AddressingMode::ZeroPage, 5),
        0x08 => info(0x08, Instruction::PHP, AddressingMode::Implied, 3),
        0x09 => info(0x09, Instruction::ORA, AddressingMode::Immediate, 2),
        0x0A => info(0x0A, Instruction::ASL, AddressingMode::Accumulator, 2),
        0x0D => info(0x0D, Instruction::ORA, AddressingMode::Absolute, 4),
        0x0E => info(0x0E, Instruction::ASL, AddressingMode::Absolute, 6),
        0x10 => info(0x10, Instruction::BPL, AddressingMode::Relative, 2),
        0x11 => info(0x11, Instruction::ORA, AddressingMode::IndirectIndexed, 5),
        0x15 => info(0x15, Instruction::ORA, AddressingMode::ZeroPageX, 4),
        0x16 => info(0x16, Instruction::ASL, AddressingMode::ZeroPageX, 6),
        0x18 => info(0x18, Instruction::CLC, AddressingMode::Implied, 2),
        0x19 => info(0x19, Instruction::ORA, AddressingMode::AbsoluteY, 4),
        0x1D => info(0x1D, Instruction::ORA, AddressingMode::AbsoluteX, 4),
        0x1E => info(0x1E, Instruction::ASL, AddressingMode::AbsoluteX, 7),
        0x20 => info(0x20, Instruction::JSR, AddressingMode::Absolute, 6),
        0x21 => info(0x21, Instruction::AND, AddressingMode::IndexedIndirect, 6),
        0x24 => info(0x24, Instruction::BIT, AddressingMode::ZeroPage, 3),
        0x25 => info(0x25, Instruction::AND, AddressingMode::ZeroPage, 3),
        0x26 => info(0x26, Instruction::ROL, AddressingMode::ZeroPage, 5),
        0x28 => info(0x28, Instruction::PLP, AddressingMode::Implied, 4),
        0x29 => info(0x29, Instruction::AND, AddressingMode::Immediate, 2),
        0x2A => info(0x2A, Instruction::ROL, AddressingMode::Accumulator, 2),
        0x2C => info(0x2C, Instruction::BIT, AddressingMode::Absolute, 4),
        0x2D => info(0x2D, Instruction::AND, AddressingMode::Absolute, 4),
        0x2E => info(0x2E, Instruction::ROL, AddressingMode::Absolute, 6),
        0x30 => info(0x30, Instruction::BMI, AddressingMode::Relative, 2),
        0x31 => info(0x31, Instruction::AND, AddressingMode::IndirectIndexed, 5),
        0x35 => info(0x35, Instruction::AND, AddressingMode::ZeroPageX, 4),
        0x36 => info(0x36, Instruction::ROL, AddressingMode::ZeroPageX, 6),
        0x38 => info(0x38, Instruction::SEC, AddressingMode::Implied, 2),
        0x39 => info(0x39, Instruction::AND, AddressingMode::AbsoluteY, 4),
        0x3D => info(0x3D, Instruction::AND, AddressingMode::AbsoluteX, 4),
        0x3E => info(0x3E, Instruction::ROL, AddressingMode::AbsoluteX, 7),
        0x40 => info(0x40, Instruction::RTI, AddressingMode::Implied, 6),
        0x41 => info(0x41, Instruction::EOR, AddressingMode::IndexedIndirect, 6),
        0x45 => info(0x45, Instruction::EOR, AddressingMode::ZeroPage, 3),
        0x46 => info(0x46, Instruction::LSR, AddressingMode::ZeroPage, 5),
        0x48 => info(0x48, Instruction::PHA, AddressingMode::Implied, 3),
        0x49 => info(0x49, Instruction::EOR, AddressingMode::Immediate, 2),
        0x4A => info(0x4A, Instruction::LSR, AddressingMode::Accumulator, 2),
        0x4C => info(0x4C, Instruction::JMP, AddressingMode::Absolute, 3),
        0x4D => info(0x4D, Instruction::EOR, AddressingMode::Absolute, 4),
        0x4E => info(0x4E, Instruction::LSR, AddressingMode::Absolute, 6),
        0x50 => info(0x50, Instruction::BVC, AddressingMode::Relative, 2),
        0x51 => info(0x51, Instruction::EOR, AddressingMode::IndirectIndexed, 5),
        0x55 => info(0x55, Instruction::EOR, AddressingMode::ZeroPageX, 4),
        0x56 => info(0x56, Instruction::LSR, AddressingMode::ZeroPageX, 6),
        0x58 => info(0x58, Instruction::CLI, AddressingMode::Implied, 2),
        0x59 => info(0x59, Instruction::EOR, AddressingMode::AbsoluteY, 4),
        0x5D => info(0x5D, Instruction::EOR, AddressingMode::AbsoluteX, 4),
        0x5E => info(0x5E, Instruction::LSR, AddressingMode::AbsoluteX, 7),
        0x60 => info(0x60, Instruction::RTS, AddressingMode::Implied, 6),
        0x61 => info(0x61, Instruction::ADC, AddressingMode::IndexedIndirect, 6),
        0x65 => info(0x65, Instruction::ADC, AddressingMode::ZeroPage, 3),
        0x66 => info(0x66, Instruction::ROR, AddressingMode::ZeroPage, 5),
        0x68 => info(0x68, Instruction::PLA, AddressingMode::Implied, 4),
        0x69 => info(0x69, Instruction::ADC, AddressingMode::Immediate, 2),
        0x6A => info(0x6A, Instruction::ROR, AddressingMode::Accumulator, 2),
        0x6C => info(0x6C, Instruction::JMP, AddressingMode::Indirect, 5),
        0x6D => info(0x6D, Instruction::ADC, AddressingMode::Absolute, 4),
        0x6E => info(0x6E, Instruction::ROR, AddressingMode::Absolute, 6),
        0x70 => info(0x70, Instruction::BVS, AddressingMode::Relative, 2),
        0x71 => info(0x71, Instruction::ADC, AddressingMode::IndirectIndexed, 5),
        0x75 => info(0x75, Instruction::ADC, AddressingMode::ZeroPageX, 4),
        0x76 => info(0x76, Instruction::ROR, AddressingMode::ZeroPageX, 6),
        0x78 => info(0x78, Instruction::SEI, AddressingMode::Implied, 2),
        0x79 => info(0x79, Instruction::ADC, AddressingMode::AbsoluteY, 4),
        0x7D => info(0x7D, Instruction::ADC, AddressingMode::AbsoluteX, 4),
        0x7E => info(0x7E, Instruction::ROR, AddressingMode::AbsoluteX, 7),
        0x81 => info(0x81, Instruction::STA, AddressingMode::IndexedIndirect, 6),
        0x84 => info(0x84, Instruction::STY, AddressingMode::ZeroPage, 3),
        0x85 => info(0x85, Instruction::STA, AddressingMode::ZeroPage, 3),
        0x86 => info(0x86, Instruction::STX, AddressingMode::ZeroPage, 3),
        0x88 => DEY_IMPLIED,
        0x8A => info(0x8A, Instruction::TXA, AddressingMode::Implied, 2),
        0x8C => info(0x8C, Instruction::STY, AddressingMode::Absolute, 4),
        0x8D => info(0x8D, Instruction::STA, AddressingMode::Absolute, 4),
        0x8E => info(0x8E, Instruction::STX, AddressingMode::Absolute, 4),
        0x90 => info(0x90, Instruction::BCC, AddressingMode::Relative, 2),
        0x91 => info(0x91, Instruction::STA, AddressingMode::IndirectIndexed, 6),
        0x94 => info(0x94, Instruction::STY, AddressingMode::ZeroPageX, 4),
        0x95 => info(0x95, Instruction::STA, AddressingMode::ZeroPageX, 4),
        0x96 => info(0x96, Instruction::STX, AddressingMode::ZeroPageY, 4),
        0x98 => info(0x98, Instruction::TYA, AddressingMode::Implied, 2),
        0x99 => info(0x99, Instruction::STA, AddressingMode::AbsoluteY, 5),
        0x9A => info(0x9A, Instruction::TXS, AddressingMode::Implied, 2),
        0x9D => info(0x9D, Instruction::STA, AddressingMode::AbsoluteX, 5),
        0xA0 => LDY_IMMEDIATE,
        0xA1 => LDA_INDEXED_INDIRECT,
        0xA2 => LDX_IMMEDIATE,
        0xA4 => LDY_ZERO_PAGE,
        0xA5 => LDA_ZERO_PAGE,
        0xA6 => LDX_ZERO_PAGE,
        0xA8 => info(0xA8, Instruction::TAY, AddressingMode::Implied, 2),
        0xA9 => LDA_IMMEDIATE,
        0xAA => info(0xAA, Instruction::TAX, AddressingMode::Implied, 2),
        0xAC => LDY_ABSOLUTE,
        0xAD => LDA_ABSOLUTE,
        0xAE => LDX_ABSOLUTE,
        0xB0 => info(0xB0, Instruction::BCS, AddressingMode::Relative, 2),
        0xB1 => info(0xB1, Instruction::LDA, AddressingMode::IndirectIndexed, 5),
        0xB4 => LDY_ZERO_PAGE_X,
        0xB5 => LDA_ZERO_PAGE_X,
        0xB6 => LDX_ZERO_PAGE_Y,
        0xB8 => info(0xB8, Instruction::CLV, AddressingMode::Implied, 2),
        0xB9 => LDA_ABSOLUTE_Y,
        0xBA => info(0xBA, Instruction::TSX, AddressingMode::Implied, 2),
        0xBC => LDY_ABSOLUTE_X,
        0xBD => LDA_ABSOLUTE_X,
        0xBE => LDX_ABSOLUTE_Y,
        0xC0 => info(0xC0, Instruction::CPY, AddressingMode::Immediate, 2),
        0xC1 => info(0xC1, Instruction::CMP, AddressingMode::IndexedIndirect, 6),
        0xC4 => info(0xC4, Instruction::CPY, AddressingMode::ZeroPage, 3),
        0xC5 => info(0xC5, Instruction::CMP, AddressingMode::ZeroPage, 3),
        0xC6 => DEC_ZERO_PAGE,
        0xC8 => INY_IMPLIED,
        0xC9 => info(0xC9, Instruction::CMP, AddressingMode::Immediate, 2),
        0xCA => DEX_IMPLIED,
        0xCC => info(0xCC, Instruction::CPY, AddressingMode::Absolute, 4),
        0xCD => info(0xCD, Instruction::CMP, AddressingMode::Absolute, 4),
        0xCE => DEC_ABSOLUTE,
        0xD0 => info(0xD0, Instruction::BNE, AddressingMode::Relative, 2),
        0xD1 => info(0xD1, Instruction::CMP, AddressingMode::IndirectIndexed, 5),
        0xD5 => info(0xD5, Instruction::CMP, AddressingMode::ZeroPageX, 4),
        0xD6 => DEC_ZERO_PAGE_X,
        0xD8 => info(0xD8, Instruction::CLD, AddressingMode::Implied, 2),
        0xD9 => info(0xD9, Instruction::CMP, AddressingMode::AbsoluteY, 4),
        0xDD => info(0xDD, Instruction::CMP, AddressingMode::AbsoluteX, 4),
        0xDE => DEC_ABSOLUTE_X,
        0xE0 => info(0xE0, Instruction::CPX, AddressingMode::Immediate, 2),
        0xE1 => info(0xE1, Instruction::SBC, AddressingMode::IndexedIndirect, 6),
        0xE4 => info(0xE4, Instruction::CPX, AddressingMode::ZeroPage, 3),
        0xE5 => info(0xE5, Instruction::SBC, AddressingMode::ZeroPage, 3),
        0xE6 => INC_ZERO_PAGE,
        0xE8 => INX_IMPLIED,
        0xE9 => info(0xE9, Instruction::SBC, AddressingMode::Immediate, 2),
        0xEA => info(0xEA, Instruction::NOP, AddressingMode::Implied, 2),
        0xEC => info(0xEC, Instruction::CPX, AddressingMode::Absolute, 4),
        0xED => info(0xED, Instruction::SBC, AddressingMode::Absolute, 4),
        0xEE => INC_ABSOLUTE,
        0xF0 => info(0xF0, Instruction::BEQ, AddressingMode::Relative, 2),
        0xF1 => info(0xF1, Instruction::SBC, AddressingMode::IndirectIndexed, 5),
        0xF5 => info(0xF5, Instruction::SBC, AddressingMode::ZeroPageX, 4),
        0xF6 => INC_ZERO_PAGE_X,
        0xF8 => info(0xF8, Instruction::SED, AddressingMode::Implied, 2),
        0xF9 => info(0xF9, Instruction::SBC, AddressingMode::AbsoluteY, 4),
        0xFD => info(0xFD, Instruction::SBC, AddressingMode::AbsoluteX, 4),
        0xFE => INC_ABSOLUTE_X,
        _ => illegal(opcode),
    }
}

pub const fn cycles_for(opcode: u8) -> u8 {
    decode(opcode).cycles
}

pub const fn length_for(mode: AddressingMode) -> usize {
    return match mode {
        AddressingMode::Implied | AddressingMode::Accumulator => 1,
        AddressingMode::Immediate
        | AddressingMode::ZeroPage
        | AddressingMode::ZeroPageX
        | AddressingMode::ZeroPageY
        | AddressingMode::Relative
        | AddressingMode::IndexedIndirect
        | AddressingMode::IndirectIndexed => 2,
        AddressingMode::Absolute | AddressingMode::AbsoluteX | AddressingMode::AbsoluteY | AddressingMode::Indirect => {
            3
        }
    };
}
