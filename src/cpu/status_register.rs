use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StatusRegister: u8 {
        /// Carry flag:
        ///    This flag is used in additions, subtractions,
        ///    comparisons and bit rotations. In additions and
        ///    subtractions, it acts as a 9th bit and lets you to chain
        ///    operations to calculate with bigger than 8-bit numbers.
        ///    When subtracting, the Carry flag is the negative of
        ///    Borrow: if an overflow occurs, the flag will be clear,
        ///    otherwise set. Comparisons are a special case of
        ///    subtraction: they assume Carry flag set and Decimal flag
        ///    clear, and do not store the result of the subtraction
        ///    anywhere.
        const Carry = 0b00000001;
        /// Zero flag:
        ///    The Zero flag will be affected in the same cases than
        ///    the Negative flag. Generally, it will be set if an
        ///    arithmetic register is being loaded with the value zero,
        ///    and cleared otherwise. The flag will behave differently
        ///    in Decimal operations.
        const Zero = 0b00000010;
        /// Interrupt disabled:
        ///    This flag can be used to prevent the processor from
        ///    jumping to the IRQ handler vector ($FFFE) whenever the
        ///    hardware line -IRQ is active. The flag will be
        ///    automatically set after taking an interrupt, so that the
        ///    processor would not keep jumping to the interrupt
        ///    routine if the -IRQ signal remains low for several clock
        ///    cycles.
        const InterruptDisabled = 0b00000100;
        /// Decimal flag:
        ///     On the NES the decimal mode is disabled so this flag has no effect.
        const Decimal = 0b00001000;
        /// Break flag:
        ///    This flag is used to distinguish software (BRK)
        ///    interrupts from hardware interrupts (IRQ or NMI). The B
        ///    flag is always set except when the P register is being
        ///    pushed on stack when jumping to an interrupt routine to
        ///    process only a hardware interrupt.
        const Break = 0b00010000;
        /// Unused flag:
        ///     To the current knowledge, this flag is always 1.
        const Unused = 0b00100000;
        /// Overflow flag:
        ///    After a binary addition or subtraction, the V flag will
        ///    be set on a sign overflow, cleared otherwise. What is a
        ///    sign overflow? For instance, if you are trying to add
        ///    123 and 45 together, the result (168) does not fit in a
        ///    8-bit signed integer (upper limit 127 and lower limit
        ///    -128). Similarly, adding -123 to -45 causes the
        ///    overflow, just like subtracting -45 from 123 or 123 from
        ///    -45 would do.
        const Overflow = 0b01000000;
        /// Negative flag:
        ///    This flag will be set after any arithmetic operations
        ///    (when any of the registers A, X or Y is being loaded
        ///    with a value). Generally, the N flag will be copied from
        ///    the topmost bit of the register being loaded.
        const Negative = 0b10000000;
    }
}

impl StatusRegister {
    /// Set the zero flag if the value is 0
    pub fn update_zero_flag(&mut self, value: u8) {
        self.set(StatusRegister::Zero, value == 0);
    }

    /// Set the negative flag if the value is negative
    pub fn update_negative_flag(&mut self, value: u8) {
        self.set(StatusRegister::Negative, (value & 0x80) > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_zero_flag_should_be_set_when_value_is_zero() {
        let mut status_register = StatusRegister::empty();
        status_register.update_zero_flag(0x00);
        assert!(status_register.contains(StatusRegister::Zero));
    }

    #[test]
    fn update_zero_flag_should_not_be_set_when_value_is_notzero() {
        let mut status_register = StatusRegister::empty();
        status_register.update_zero_flag(0x04);
        assert!(!status_register.contains(StatusRegister::Zero));
    }

    #[test]
    fn update_negative_flag_should_set_when_value_is_negative() {
        let mut status_register = StatusRegister::empty();
        status_register.update_negative_flag((-5 as i8) as u8);
        assert!(status_register.contains(StatusRegister::Negative));
    }

    #[test]
    fn update_negative_flag_should_not_beset_when_value_is_positive() {
        let mut status_register = StatusRegister::empty();
        status_register.update_negative_flag((5 as i8) as u8);
        assert!(!status_register.contains(StatusRegister::Negative));
    }
}
