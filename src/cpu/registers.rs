#[derive(Default)]
pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,     // Stack Pointer
    pub pc: u16,    // Program Counter
    pub status: u8, // Processor Status
}

pub const CARRY_FLAG_BITMASK: u8 = 0x01; // Bit 0 - C
pub const ZERO_FLAG_BITMASK: u8 = 0x02; // Bit 1 - Z
pub const INTERRUPT_FLAG_BITMASK: u8 = 0x04; // Bit 2 - I
pub const DECIMAL_FLAG_BITMASK: u8 = 0x08; // Bit 3 - D
pub const BREAK_FLAG_BITMASK: u8 = 0x10; // Bit 4 - B
pub const OVERFLOW_FLAG_BITMASK: u8 = 0x40; // Bit 6 - V
pub const NEGATIVE_FLAG_BITMASK: u8 = 0x80; // Bit 7 - N

impl Registers {
    pub fn set_flag(&mut self, flag_bitmask: u8, condition: bool) {
        if condition {
            self.status |= flag_bitmask; // Set the flag
        } else {
            self.status &= !flag_bitmask; // Clear the flag
        }
    }

    pub fn is_flag_set(&self, flag_bitmask: u8) -> bool {
        self.status & flag_bitmask != 0
    }

    pub fn set_accumulator(&mut self, value: u8) {
        self.a = value;
        self.update_zero_and_negative(value);
    }

    pub fn set_x(&mut self, value: u8) {
        self.x = value;
        self.update_zero_and_negative(value);
    }

    pub fn set_y(&mut self, value: u8) {
        self.y = value;
        self.update_zero_and_negative(value);
    }

    pub fn update_carry_flag(&mut self, condition: bool) {
        self.set_flag(CARRY_FLAG_BITMASK, condition);
    }

    pub fn update_decimal_flag(&mut self, condition: bool) {
        self.set_flag(DECIMAL_FLAG_BITMASK, condition);
    }

    pub fn update_interrupt_flag(&mut self, condition: bool) {
        self.set_flag(INTERRUPT_FLAG_BITMASK, condition);
    }

    pub fn update_overflow_flag(&mut self, condition: bool) {
        self.set_flag(OVERFLOW_FLAG_BITMASK, condition);
    }

    pub fn update_zero_and_negative(&mut self, value: u8) {
        self.set_flag(ZERO_FLAG_BITMASK, value == 0);
        self.set_flag(NEGATIVE_FLAG_BITMASK, value & 0x80 != 0);
    }

    pub fn update_pc(&mut self, value: u16) {
        self.pc = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn registers() -> Registers {
        Registers::default()
    }

    #[rstest]
    fn test_default_registers(registers: Registers) {
        assert_eq!(registers.a, 0);
        assert_eq!(registers.x, 0);
        assert_eq!(registers.y, 0);
        assert_eq!(registers.sp, 0);
        assert_eq!(registers.pc, 0);
        assert_eq!(registers.status, 0);
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_set_accumulator(mut registers: Registers, #[case] value: u8, #[case] zero: bool, #[case] negative: bool) {
        registers.set_accumulator(value);
        assert_eq!(registers.a, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    fn test_set_accumulator_clears_flags(mut registers: Registers) {
        registers.set_accumulator(0x00);
        registers.set_accumulator(0x01);
        assert!(!registers.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be cleared");
        assert!(
            !registers.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be cleared"
        );
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_set_x(mut registers: Registers, #[case] value: u8, #[case] zero: bool, #[case] negative: bool) {
        registers.set_x(value);
        assert_eq!(registers.x, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    fn test_set_x_clears_flags(mut registers: Registers) {
        registers.set_x(0x00);
        registers.set_x(0x01);
        assert!(!registers.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be cleared");
        assert!(
            !registers.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be cleared"
        );
    }

    #[rstest]
    #[case(0x42, false, false)]
    #[case(0x00, true, false)]
    #[case(0x80, false, true)]
    fn test_set_y(mut registers: Registers, #[case] value: u8, #[case] zero: bool, #[case] negative: bool) {
        registers.set_y(value);
        assert_eq!(registers.y, value);
        assert_eq!(registers.is_flag_set(ZERO_FLAG_BITMASK), zero, "zero flag");
        assert_eq!(registers.is_flag_set(NEGATIVE_FLAG_BITMASK), negative, "negative flag");
    }

    #[rstest]
    fn test_set_y_clears_flags(mut registers: Registers) {
        registers.set_y(0x00);
        registers.set_y(0x01);
        assert!(!registers.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be cleared");
        assert!(
            !registers.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be cleared"
        );
    }

    #[rstest]
    fn test_set_and_is_break_flag(mut registers: Registers) {
        registers.set_flag(BREAK_FLAG_BITMASK, true);
        assert!(registers.is_flag_set(BREAK_FLAG_BITMASK), "break flag should be set");
        registers.set_flag(BREAK_FLAG_BITMASK, false);
        assert!(!registers.is_flag_set(BREAK_FLAG_BITMASK), "break flag should be clear");
    }
}
