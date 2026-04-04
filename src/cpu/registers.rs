#[derive(Default)]
pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,     // Stack Pointer
    pub pc: u16,    // Program Counter
    pub status: u8, // Processor Status
}

const ZERO_FLAG_BITMASK: u8 = 0x02;
const NEGATIVE_FLAG_BITMASK: u8 = 0x80;

impl Registers {
    pub fn set_accoumulator(&mut self, value: u8) {
        self.a = value;
        self.update_zero_and_negative(value);
    }

    fn update_zero_and_negative(&mut self, value: u8) {
        self.set_flag(ZERO_FLAG_BITMASK, value == 0);
        self.set_flag(NEGATIVE_FLAG_BITMASK, value & 0x80 != 0);
    }

    fn set_flag(&mut self, flag_bitmask: u8, condition: bool) {
        if condition {
            self.status |= flag_bitmask; // Set the flag
        } else {
            self.status &= !flag_bitmask; // Clear the flag
        }
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
        regs.set_accoumulator(0x42);
        assert_eq!(regs.a, 0x42);
        assert_eq!(
            regs.status & ZERO_FLAG_BITMASK,
            0,
            "zero flag should be clear"
        );
        assert_eq!(
            regs.status & NEGATIVE_FLAG_BITMASK,
            0,
            "negative flag should be clear"
        );
    }

    #[test]
    fn test_set_accumulator_zero_sets_zero_flag() {
        let mut regs = Registers::default();
        regs.set_accoumulator(0x00);
        assert_eq!(regs.a, 0x00);
        assert_ne!(
            regs.status & ZERO_FLAG_BITMASK,
            0,
            "zero flag should be set"
        );
        assert_eq!(
            regs.status & NEGATIVE_FLAG_BITMASK,
            0,
            "negative flag should be clear"
        );
    }

    #[test]
    fn test_set_accumulator_negative_sets_negative_flag() {
        let mut regs = Registers::default();
        regs.set_accoumulator(0x80);
        assert_eq!(regs.a, 0x80);
        assert_eq!(
            regs.status & ZERO_FLAG_BITMASK,
            0,
            "zero flag should be clear"
        );
        assert_ne!(
            regs.status & NEGATIVE_FLAG_BITMASK,
            0,
            "negative flag should be set"
        );
    }

    #[test]
    fn test_set_accumulator_clears_flags_on_positive_after_zero() {
        let mut regs = Registers::default();
        regs.set_accoumulator(0x00);
        regs.set_accoumulator(0x01);
        assert_eq!(
            regs.status & ZERO_FLAG_BITMASK,
            0,
            "zero flag should be cleared"
        );
    }

    #[test]
    fn test_set_accumulator_clears_negative_flag_on_positive_after_negative() {
        let mut regs = Registers::default();
        regs.set_accoumulator(0x80);
        regs.set_accoumulator(0x01);
        assert_eq!(
            regs.status & NEGATIVE_FLAG_BITMASK,
            0,
            "negative flag should be cleared"
        );
    }
}
