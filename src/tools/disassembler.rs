use crate::cpu::{addressing_mode::AddressingMode, instructions::InstructionInfo};

pub trait Disassembler {
    fn disassemble_instruction(&self, instruction_info: &InstructionInfo, operands: &[u8]) -> String;
}

pub struct DefaultDisassembler {
    separator: String,
}

impl DefaultDisassembler {
    pub fn new(separator: String) -> Self {
        Self { separator }
    }
}

impl Disassembler for DefaultDisassembler {
    fn disassemble_instruction(&self, instruction_info: &InstructionInfo, operands: &[u8]) -> String {
        let name = format!("{:?}", instruction_info.instruction);
        let operand_str = match instruction_info.mode {
            AddressingMode::Implied => return name,
            AddressingMode::Accumulator => "A".to_string(),
            AddressingMode::Immediate => format!("#${:02X}", operands[0]),
            AddressingMode::ZeroPage => format!("${:02X}", operands[0]),
            AddressingMode::ZeroPageX => format!("${:02X},X", operands[0]),
            AddressingMode::ZeroPageY => format!("${:02X},Y", operands[0]),
            AddressingMode::Relative => format!("${:02X}", operands[0]),
            AddressingMode::Absolute => format!("${:02X}{:02X}", operands[1], operands[0]),
            AddressingMode::AbsoluteX => format!("${:02X}{:02X},X", operands[1], operands[0]),
            AddressingMode::AbsoluteY => format!("${:02X}{:02X},Y", operands[1], operands[0]),
            AddressingMode::Indirect => format!("(${:02X}{:02X})", operands[1], operands[0]),
            AddressingMode::IndexedIndirect => format!("(${:02X},X)", operands[0]),
            AddressingMode::IndirectIndexed => format!("(${:02X}),Y", operands[0]),
        };
        format!("{}{}{}", name, self.separator, operand_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::instructions::{BRK_IMPLIED, LDA_IMMEDIATE};
    use rstest::{fixture, rstest};

    #[fixture]
    fn disassembler() -> DefaultDisassembler {
        DefaultDisassembler {
            separator: "\t".to_string(),
        }
    }

    #[rstest]
    #[case(&LDA_IMMEDIATE, &[0x45], "LDA\t#$45")]
    #[case(&BRK_IMPLIED, &[], "BRK")]
    fn test_disassemble(
        disassembler: DefaultDisassembler,
        #[case] instruction_info: &InstructionInfo,
        #[case] operands: &[u8],
        #[case] expected: &str,
    ) {
        assert_eq!(
            disassembler.disassemble_instruction(instruction_info, operands),
            expected
        );
    }
}
