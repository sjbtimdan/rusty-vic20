use crate::registers::{Registers, CARRY, DECIMAL};

/// Binary and BCD ADC: A = A + operand + C.
/// Sets C, Z, N, V based on binary result (NMOS 6502 quirk: flags from binary even in decimal mode).
pub fn adc(registers: &mut Registers, operand: u8) {
    let carry_in = registers.is_flag_set(CARRY) as u8;
    if registers.is_flag_set(DECIMAL) {
        let bin = (registers.a as u16) + (operand as u16) + (carry_in as u16);
        let bin_byte = bin as u8;
        let overflow = (!(registers.a ^ operand) & (registers.a ^ bin_byte) & 0x80) != 0;
        registers.update_overflow_flag(overflow);
        registers.update_zero_and_negative(bin_byte);

        let lo = (registers.a & 0x0F) + (operand & 0x0F) + carry_in;
        let lo_carry = if lo >= 10 { 1 } else { 0 };
        let lo = if lo >= 10 { lo + 6 } else { lo };
        let hi = (registers.a >> 4) + (operand >> 4) + lo_carry;
        let carry_out = hi >= 10;
        let hi = if hi >= 10 { hi + 6 } else { hi };

        registers.update_carry_flag(carry_out);
        registers.a = ((hi & 0x0F) << 4) | (lo & 0x0F);
    } else {
        let result = (registers.a as u16) + (operand as u16) + (carry_in as u16);
        let result_byte = result as u8;
        let overflow = (!(registers.a ^ operand) & (registers.a ^ result_byte) & 0x80) != 0;
        registers.update_overflow_flag(overflow);
        registers.update_carry_flag(result > 0xFF);
        registers.set_accumulator(result_byte);
    }
}

/// Binary and BCD SBC: A = A - operand - (1-C).
/// Sets C, Z, N, V based on binary result.
pub fn sbc(registers: &mut Registers, operand: u8) {
    let carry_in = registers.is_flag_set(CARRY) as u8;
    if registers.is_flag_set(DECIMAL) {
        let effective = !operand;
        let bin = (registers.a as u16) + (effective as u16) + (carry_in as u16);
        let bin_byte = bin as u8;
        let overflow = (!(registers.a ^ effective) & (registers.a ^ bin_byte) & 0x80) != 0;
        registers.update_overflow_flag(overflow);
        registers.update_zero_and_negative(bin_byte);

        let borrow = 1 - carry_in;
        let lo = (registers.a & 0x0F) as i16 - (operand & 0x0F) as i16 - borrow as i16;
        let lo_borrow = if lo < 0 { 1 } else { 0 };
        let lo = if lo < 0 { lo - 6 } else { lo };

        let hi = (registers.a >> 4) as i16 - (operand >> 4) as i16 - lo_borrow;
        let carry_out = hi >= 0;
        let hi = if hi < 0 { hi - 6 } else { hi };

        registers.update_carry_flag(carry_out);
        registers.a = ((hi as u8 & 0x0F) << 4) | (lo as u8 & 0x0F);
    } else {
        let effective = !operand;
        let result = (registers.a as u16) + (effective as u16) + (carry_in as u16);
        let result_byte = result as u8;
        let overflow = (!(registers.a ^ effective) & (registers.a ^ result_byte) & 0x80) != 0;
        registers.update_overflow_flag(overflow);
        registers.update_carry_flag(result > 0xFF);
        registers.set_accumulator(result_byte);
    }
}

/// Compare: sets C, Z, N based on reg - operand (no store).
pub fn compare(registers: &mut Registers, reg: u8, operand: u8) {
    let result = reg.wrapping_sub(operand);
    registers.update_carry_flag(reg >= operand);
    registers.update_zero_and_negative(result);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::{NEGATIVE, OVERFLOW, ZERO};

    #[test]
    fn test_adc_binary_basic() {
        let mut r = Registers::default();
        r.a = 0x01;
        r.update_carry_flag(false);
        adc(&mut r, 0x01);
        assert_eq!(r.a, 0x02);
        assert!(!r.is_flag_set(CARRY));
        assert!(!r.is_flag_set(ZERO));
        assert!(!r.is_flag_set(NEGATIVE));
        assert!(!r.is_flag_set(OVERFLOW));
    }

    #[test]
    fn test_adc_binary_carry_out() {
        let mut r = Registers::default();
        r.a = 0xFF;
        r.update_carry_flag(false);
        adc(&mut r, 0x01);
        assert_eq!(r.a, 0x00);
        assert!(r.is_flag_set(CARRY));
        assert!(r.is_flag_set(ZERO));
    }

    #[test]
    fn test_adc_binary_overflow() {
        let mut r = Registers::default();
        r.a = 0x50;
        r.update_carry_flag(false);
        adc(&mut r, 0x50);
        assert_eq!(r.a, 0xA0);
        assert!(r.is_flag_set(OVERFLOW));
        assert!(r.is_flag_set(NEGATIVE));
    }

    #[test]
    fn test_adc_binary_carry_in() {
        let mut r = Registers::default();
        r.a = 0x01;
        r.update_carry_flag(true);
        adc(&mut r, 0x01);
        assert_eq!(r.a, 0x03);
    }

    #[test]
    fn test_adc_decimal() {
        let mut r = Registers::default();
        r.update_decimal_flag(true);
        r.a = 0x15;
        r.update_carry_flag(false);
        adc(&mut r, 0x27);
        assert_eq!(r.a, 0x42);
        assert!(!r.is_flag_set(CARRY));
    }

    #[test]
    fn test_sbc_binary() {
        let mut r = Registers::default();
        r.a = 0x50;
        r.update_carry_flag(true); // no borrow
        sbc(&mut r, 0x30);
        assert_eq!(r.a, 0x20);
        assert!(r.is_flag_set(CARRY));
    }

    #[test]
    fn test_sbc_binary_borrow() {
        let mut r = Registers::default();
        r.a = 0x00;
        r.update_carry_flag(true); // no borrow initially
        sbc(&mut r, 0x01);
        assert_eq!(r.a, 0xFF);
        assert!(!r.is_flag_set(CARRY)); // borrow occurred
        assert!(r.is_flag_set(NEGATIVE));
    }

    #[test]
    fn test_compare() {
        let mut r = Registers::default();
        compare(&mut r, 0x42, 0x40);
        assert!(r.is_flag_set(CARRY));
        assert!(!r.is_flag_set(ZERO));
        assert!(!r.is_flag_set(NEGATIVE));

        compare(&mut r, 0x40, 0x42);
        assert!(!r.is_flag_set(CARRY));
        assert!(r.is_flag_set(NEGATIVE));

        compare(&mut r, 0x42, 0x42);
        assert!(r.is_flag_set(CARRY));
        assert!(r.is_flag_set(ZERO));
    }
}
