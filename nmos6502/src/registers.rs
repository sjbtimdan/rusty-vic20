use std::fmt;

#[derive(Debug, Clone)]
pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub status: u8,
}

pub const CARRY: u8 = 0x01;
pub const ZERO: u8 = 0x02;
pub const INTERRUPT: u8 = 0x04;
pub const DECIMAL: u8 = 0x08;
pub const BREAK: u8 = 0x10;
pub const UNUSED: u8 = 0x20;
pub const OVERFLOW: u8 = 0x40;
pub const NEGATIVE: u8 = 0x80;

impl Default for Registers {
    fn default() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            status: UNUSED,
        }
    }
}

impl Registers {
    #[inline]
    pub fn is_flag_set(&self, flag: u8) -> bool {
        self.status & flag != 0
    }

    #[inline]
    pub fn update_carry_flag(&mut self, on: bool) {
        self.set_flag(CARRY, on);
    }

    #[inline]
    pub fn update_zero_flag(&mut self, on: bool) {
        self.set_flag(ZERO, on);
    }

    #[inline]
    pub fn update_negative_flag(&mut self, on: bool) {
        self.set_flag(NEGATIVE, on);
    }

    #[inline]
    pub fn update_overflow_flag(&mut self, on: bool) {
        self.set_flag(OVERFLOW, on);
    }

    #[inline]
    pub fn update_decimal_flag(&mut self, on: bool) {
        self.set_flag(DECIMAL, on);
    }

    #[inline]
    pub fn update_interrupt_flag(&mut self, on: bool) {
        self.set_flag(INTERRUPT, on);
    }

    #[inline]
    pub fn update_zero_and_negative(&mut self, value: u8) {
        self.update_zero_flag(value == 0);
        self.update_negative_flag(value & 0x80 != 0);
    }

    #[inline]
    pub fn set_accumulator(&mut self, value: u8) {
        self.a = value;
        self.update_zero_and_negative(value);
    }

    #[inline]
    pub fn set_x(&mut self, value: u8) {
        self.x = value;
        self.update_zero_and_negative(value);
    }

    #[inline]
    pub fn set_y(&mut self, value: u8) {
        self.y = value;
        self.update_zero_and_negative(value);
    }

    #[inline]
    fn set_flag(&mut self, flag: u8, on: bool) {
        if on {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
    }
}

impl fmt::Display for Registers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A:{:02X} X:{:02X} Y:{:02X} SP:{:02X} PC:{:04X} SR:{:02X}[{}{}-{}{}{}{}{}]",
            self.a,
            self.x,
            self.y,
            self.sp,
            self.pc,
            self.status,
            if self.is_flag_set(NEGATIVE) { 'N' } else { '-' },
            if self.is_flag_set(OVERFLOW) { 'V' } else { '-' },
            if self.is_flag_set(BREAK) { 'B' } else { '-' },
            if self.is_flag_set(DECIMAL) { 'D' } else { '-' },
            if self.is_flag_set(INTERRUPT) { 'I' } else { '-' },
            if self.is_flag_set(ZERO) { 'Z' } else { '-' },
            if self.is_flag_set(CARRY) { 'C' } else { '-' },
        )
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
        assert_eq!(registers.sp, 0xFD);
        assert_eq!(registers.status, UNUSED);
        assert!(registers.is_flag_set(UNUSED));
    }

    #[rstest]
    fn test_set_accumulator_updates_flags(registers: Registers) {
        test_updates_flags(registers, |r, v| r.set_accumulator(v))
    }

    #[rstest]
    fn test_set_x_updates_flags(registers: Registers) {
        test_updates_flags(registers, |r, v| r.set_x(v))
    }

    #[rstest]
    fn test_set_y_updates_flags(registers: Registers) {
        test_updates_flags(registers, |r, v| r.set_y(v))
    }

    #[rstest]
    fn test_carry_flag(mut registers: Registers) {
        registers.update_carry_flag(true);
        assert!(registers.is_flag_set(CARRY));
        registers.update_carry_flag(false);
        assert!(!registers.is_flag_set(CARRY));
    }

    #[rstest]
    fn test_zero_flag(mut registers: Registers) {
        registers.update_zero_flag(true);
        assert!(registers.is_flag_set(ZERO));
        registers.update_zero_flag(false);
        assert!(!registers.is_flag_set(ZERO));
    }

    #[rstest]
    fn test_decimal_flag(mut registers: Registers) {
        registers.update_decimal_flag(true);
        assert!(registers.is_flag_set(DECIMAL));
        registers.update_decimal_flag(false);
        assert!(!registers.is_flag_set(DECIMAL));
    }

    #[rstest]
    fn test_interrupt_flag(mut registers: Registers) {
        registers.update_interrupt_flag(true);
        assert!(registers.is_flag_set(INTERRUPT));
        registers.update_interrupt_flag(false);
        assert!(!registers.is_flag_set(INTERRUPT));
    }

    #[rstest]
    fn test_negative_flag(mut registers: Registers) {
        registers.update_negative_flag(true);
        assert!(registers.is_flag_set(NEGATIVE));
        registers.update_negative_flag(false);
        assert!(!registers.is_flag_set(NEGATIVE));
    }

    #[rstest]
    fn test_overflow_flag(mut registers: Registers) {
        registers.update_overflow_flag(true);
        assert!(registers.is_flag_set(OVERFLOW));
        registers.update_overflow_flag(false);
        assert!(!registers.is_flag_set(OVERFLOW));
    }

    fn test_updates_flags(mut registers: Registers, set_register: fn(&mut Registers, u8)) {
        set_register(&mut registers, 0x00);
        assert!(registers.is_flag_set(ZERO));
        assert!(!registers.is_flag_set(NEGATIVE));

        set_register(&mut registers, 0x80);
        assert!(!registers.is_flag_set(ZERO));
        assert!(registers.is_flag_set(NEGATIVE));

        set_register(&mut registers, 0x42);
        assert!(!registers.is_flag_set(ZERO));
        assert!(!registers.is_flag_set(NEGATIVE));
    }
}
