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

    fn update_zero_and_negative(&mut self, value: u8) {
        self.set_flag(ZERO_FLAG_BITMASK, value == 0);
        self.set_flag(NEGATIVE_FLAG_BITMASK, value & 0x80 != 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registers() {
        let regs = Registers::default();
        assert_eq!(regs.a, 0);
        assert_eq!(regs.x, 0);
        assert_eq!(regs.y, 0);
        assert_eq!(regs.sp, 0);
        assert_eq!(regs.pc, 0);
        assert_eq!(regs.status, 0);
    }

    #[test]
    fn test_set_accumulator_nonzero() {
        let mut regs = Registers::default();
        regs.set_accumulator(0x42);
        assert_eq!(regs.a, 0x42);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be clear");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be clear"
        );
    }

    #[test]
    fn test_set_accumulator_zero_sets_zero_flag() {
        let mut regs = Registers::default();
        regs.set_accumulator(0x00);
        assert_eq!(regs.a, 0x00);
        assert!(regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be set");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be clear"
        );
    }

    #[test]
    fn test_set_accumulator_negative_sets_negative_flag() {
        let mut regs = Registers::default();
        regs.set_accumulator(0x80);
        assert_eq!(regs.a, 0x80);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be clear");
        assert!(regs.is_flag_set(NEGATIVE_FLAG_BITMASK), "negative flag should be set");
    }

    #[test]
    fn test_set_accumulator_clears_flags_on_positive_after_zero() {
        let mut regs = Registers::default();
        regs.set_accumulator(0x00);
        regs.set_accumulator(0x01);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be cleared");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be cleared"
        );
    }

    #[test]
    fn test_set_and_is_break_flag() {
        let mut regs = Registers::default();
        regs.set_flag(BREAK_FLAG_BITMASK, true);
        assert!(regs.is_flag_set(BREAK_FLAG_BITMASK), "break flag should be set");
        regs.set_flag(BREAK_FLAG_BITMASK, false);
        assert!(!regs.is_flag_set(BREAK_FLAG_BITMASK), "break flag should be clear");
    }

    #[test]
    fn test_set_x_nonzero() {
        let mut regs = Registers::default();
        regs.set_x(0x42);
        assert_eq!(regs.x, 0x42);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be clear");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be clear"
        );
    }

    #[test]
    fn test_set_x_zero_sets_zero_flag() {
        let mut regs = Registers::default();
        regs.set_x(0x00);
        assert_eq!(regs.x, 0x00);
        assert!(regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be set");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be clear"
        );
    }

    #[test]
    fn test_set_x_negative_sets_negative_flag() {
        let mut regs = Registers::default();
        regs.set_x(0x80);
        assert_eq!(regs.x, 0x80);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be clear");
        assert!(regs.is_flag_set(NEGATIVE_FLAG_BITMASK), "negative flag should be set");
    }

    #[test]
    fn test_set_x_clears_flags_on_positive_after_zero() {
        let mut regs = Registers::default();
        regs.set_x(0x00);
        regs.set_x(0x01);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be cleared");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be cleared"
        );
    }

    #[test]
    fn test_set_y_nonzero() {
        let mut regs = Registers::default();
        regs.set_y(0x42);
        assert_eq!(regs.y, 0x42);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be clear");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be clear"
        );
    }

    #[test]
    fn test_set_y_zero_sets_zero_flag() {
        let mut regs = Registers::default();
        regs.set_y(0x00);
        assert_eq!(regs.y, 0x00);
        assert!(regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be set");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be clear"
        );
    }

    #[test]
    fn test_set_y_negative_sets_negative_flag() {
        let mut regs = Registers::default();
        regs.set_y(0x80);
        assert_eq!(regs.y, 0x80);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be clear");
        assert!(regs.is_flag_set(NEGATIVE_FLAG_BITMASK), "negative flag should be set");
    }

    #[test]
    fn test_set_y_clears_flags_on_positive_after_zero() {
        let mut regs = Registers::default();
        regs.set_y(0x00);
        regs.set_y(0x01);
        assert!(!regs.is_flag_set(ZERO_FLAG_BITMASK), "zero flag should be cleared");
        assert!(
            !regs.is_flag_set(NEGATIVE_FLAG_BITMASK),
            "negative flag should be cleared"
        );
    }
}
