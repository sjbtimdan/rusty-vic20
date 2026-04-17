use crate::cpu::{
    addressing_mode::AddressingMode,
    instructions::{InstructionInfo, decode},
};

#[cfg_attr(test, unimock::unimock(api=DisassemblerMock))]
pub trait Disassembler {
    fn parse_instruction(&self, data: &[u8]) -> (usize, InstructionInfo);
    fn disassemble_instruction(
        &self,
        instruction_info: &InstructionInfo,
        operands: &[u8],
        instruction_address: u16,
    ) -> String;
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
    fn parse_instruction(&self, data: &[u8]) -> (usize, InstructionInfo) {
        let operand = data[0];
        let instruction_info = decode(operand);
        (1 + instruction_info.mode.operand_count(), instruction_info)
    }

    fn disassemble_instruction(
        &self,
        instruction_info: &InstructionInfo,
        operands: &[u8],
        instruction_address: u16,
    ) -> String {
        disassemble_instruction(instruction_info, operands, instruction_address, &self.separator)
    }
}

pub fn disassemble_instruction(
    instruction_info: &InstructionInfo,
    operands: &[u8],
    instruction_address: u16,
    separator: &str,
) -> String {
    let name = format!("{:?}", instruction_info.instruction);
    let operand_str = match instruction_info.mode {
        AddressingMode::Implied => return name,
        AddressingMode::Accumulator => "A".to_string(),
        AddressingMode::Immediate => format!("#${:02X}", operands[0]),
        AddressingMode::ZeroPage => format!("${:02X}", operands[0]),
        AddressingMode::ZeroPageX => format!("${:02X},X", operands[0]),
        AddressingMode::ZeroPageY => format!("${:02X},Y", operands[0]),
        AddressingMode::Relative => {
            let offset = operands[0] as i8;
            let next_pc = instruction_address.wrapping_add(2);
            let target = next_pc.wrapping_add_signed(offset as i16);
            format!("${:04X}", target)
        }
        AddressingMode::Absolute => format!("${:02X}{:02X}", operands[1], operands[0]),
        AddressingMode::AbsoluteX => format!("${:02X}{:02X},X", operands[1], operands[0]),
        AddressingMode::AbsoluteY => format!("${:02X}{:02X},Y", operands[1], operands[0]),
        AddressingMode::Indirect => format!("(${:02X}{:02X})", operands[1], operands[0]),
        AddressingMode::IndexedIndirect => format!("(${:02X},X)", operands[0]),
        AddressingMode::IndirectIndexed => format!("(${:02X}),Y", operands[0]),
    };
    format!("{}{}{}", name, separator, operand_str)
}

pub fn disassemble_bytes(
    data: &[u8],
    disassembler: &impl Disassembler,
    base_address: u16,
    start_address: u16,
) -> Vec<String> {
    assert!(
        start_address >= base_address,
        "Start address: 0x{:04X} must be greater than or equal to base address: 0x{:04X}",
        start_address,
        base_address
    );
    let mut result = Vec::new();
    let mut index = (start_address - base_address) as usize;
    let data_len = data.len();
    while index < data_len {
        let (instruction_length, instruction_info) = disassembler.parse_instruction(&data[index..]);
        let end_next_instruction = index + instruction_length;
        if end_next_instruction > data_len {
            break;
        }
        let instruction_address = base_address + index as u16;
        let instruction_str = disassembler.disassemble_instruction(
            &instruction_info,
            &data[index + 1..end_next_instruction],
            instruction_address,
        );
        let address_str = format!("0x{:04X}:", instruction_address);
        result.push(format!("{}\t{}", address_str, instruction_str));
        index += instruction_length;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::instructions::{BNE_RELATIVE, BRK_IMPLIED, LDA_IMMEDIATE};
    use rstest::{fixture, rstest};

    #[fixture]
    fn disassembler() -> DefaultDisassembler {
        DefaultDisassembler {
            separator: "\t".to_string(),
        }
    }

    #[test]
    fn test_disassemble_bytes() {
        use unimock::{MockFn, Unimock, matching};

        let mock = Unimock::new((
            DisassemblerMock::parse_instruction
                .each_call(matching!(_))
                .returns((2_usize, LDA_IMMEDIATE)),
            DisassemblerMock::disassemble_instruction
                .each_call(matching!(_, _, _))
                .returns("LDA\t#$45".to_string()),
        ));

        let result = disassemble_bytes(&[0xA9, 0x45], &mock, 0x1000, 0x1000);

        assert_eq!(result, vec!["0x1000:\tLDA\t#$45"]);
    }

    #[rstest]
    #[case(&LDA_IMMEDIATE, &[0x45], "LDA\t#$45")]
    #[case(&BRK_IMPLIED, &[], "BRK")]
    #[case(&BNE_RELATIVE, &[0x05], "BNE\t$1007")]
    fn test_disassemble(
        disassembler: DefaultDisassembler,
        #[case] instruction_info: &InstructionInfo,
        #[case] operands: &[u8],
        #[case] expected: &str,
    ) {
        assert_eq!(
            disassembler.disassemble_instruction(instruction_info, operands, 0x1000),
            expected
        );
    }
}
