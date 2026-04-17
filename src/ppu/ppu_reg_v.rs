pub struct PPURegV {
    // Source: https://www.nesdev.org/wiki/PPU_scrolling#PPU_internal_registers
    // yyy NN YYYYY XXXXX
    // ||| || ||||| +++++-- coarse X scroll
    // ||| || +++++-------- coarse Y scroll
    // ||| ++-------------- nametable select
    // +++----------------- fine Y scroll
    value: u16,
}

impl Default for PPURegV {
    fn default() -> Self {
        PPURegV { value: 0 }
    }
}

impl PPURegV {
    pub fn get_coarse_x(&self) -> u8 {
        (self.value & 0x1F) as u8
    }

    pub fn get_coarse_y(&self) -> u8 {
        ((self.value >> 5) & 0x1F) as u8
    }

    pub fn get_fine_y(&self) -> u8 {
        ((self.value >> 12) & 0x07) as u8
    }

    pub fn inc_coarse_x(&mut self) {
        if self.get_coarse_x() == 31 {
            self.value &= !0x001F; // Set coarse X to 0
            self.value ^= 0x0400; // switch horizontal nametable
        } else {
            self.value = self.value.wrapping_add(1);
        }
    }

    pub fn inc_y(&mut self) {
        // if fine Y < 7
        if (self.value & 0x7000) != 0x7000 {
            self.value += 0x1000; // increment fine Y
        } else {
            self.value &= !0x7000; // fine Y = 0
            let mut y = self.get_coarse_y(); // let y = coarse Y

            // Row 29 is the last row of tiles in a nametable.
            // To wrap to the next nametable when incrementing coarse Y from 29, the vertical nametable is switched by toggling bit 11, and coarse Y wraps to row 0.
            if y == 29 {
                y = 0; // coarse Y = 0
                self.value ^= 0x0800; // switch vertical nametable
            }
            // Coarse Y can be set out of bounds (> 29), which will cause the PPU to read the attribute data stored there as tile data.
            // If coarse Y is incremented from 31, it will wrap to 0, but the nametable will not switch.
            else if y == 31 {
                y = 0; // coarse Y = 0, nametable not switched
            } else {
                y += 1; // increment coarse Y
            }
            self.value = (self.value & !0x03E0) | ((y as u16) << 5); // put coarse Y back into v
        }
    }

    pub fn get_value(&self) -> u16 {
        self.value
    }

    pub fn set_value(&mut self, value: u16) {
        self.value = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(value: u16) -> PPURegV {
        let mut r = PPURegV::default();
        r.set_value(value);
        r
    }

    // --- Getters ---

    #[test]
    fn get_coarse_x_returns_zero_when_bits_0_to_4_are_clear() {
        assert_eq!(reg(0x0000).get_coarse_x(), 0);
    }

    #[test]
    fn get_coarse_x_returns_31_when_bits_0_to_4_are_set() {
        assert_eq!(reg(0x001F).get_coarse_x(), 31);
    }

    #[test]
    fn get_coarse_y_returns_zero_when_bits_5_to_9_are_clear() {
        assert_eq!(reg(0x0000).get_coarse_y(), 0);
    }

    #[test]
    fn get_coarse_y_returns_31_when_bits_5_to_9_are_set() {
        assert_eq!(reg(0x03E0).get_coarse_y(), 31);
    }

    #[test]
    fn get_fine_y_returns_zero_when_bits_12_to_14_are_clear() {
        assert_eq!(reg(0x0000).get_fine_y(), 0);
    }

    #[test]
    fn get_fine_y_returns_7_when_bits_12_to_14_are_set() {
        assert_eq!(reg(0x7000).get_fine_y(), 7);
    }

    // --- inc_coarse_x ---

    #[test]
    fn inc_coarse_x_increments_normally() {
        let mut r = reg(5); // coarse X = 5
        r.inc_coarse_x();
        assert_eq!(r.get_coarse_x(), 6);
        assert_eq!(r.get_value() & 0x0400, 0); // nametable bit unchanged
    }

    #[test]
    fn inc_coarse_x_wraps_at_31_and_switches_horizontal_nametable() {
        // coarse X = 31, horizontal nametable bit clear
        let mut r = reg(0x001F);
        r.inc_coarse_x();
        assert_eq!(r.get_coarse_x(), 0);
        assert_eq!(r.get_value() & 0x0400, 0x0400); // bit 10 toggled on
    }

    #[test]
    fn inc_coarse_x_wraps_at_31_and_toggles_nametable_bit_back_when_already_set() {
        // coarse X = 31, horizontal nametable bit already set
        let mut r = reg(0x041F);
        r.inc_coarse_x();
        assert_eq!(r.get_coarse_x(), 0);
        assert_eq!(r.get_value() & 0x0400, 0); // bit 10 toggled back off
    }

    // --- inc_y ---

    #[test]
    fn inc_y_increments_fine_y_when_less_than_7() {
        let mut r = reg(0x1000); // fine Y = 1, coarse Y = 0
        r.inc_y();
        assert_eq!(r.get_fine_y(), 2);
        assert_eq!(r.get_coarse_y(), 0);
    }

    #[test]
    fn inc_y_resets_fine_y_and_increments_coarse_y_when_fine_y_is_7() {
        // fine Y = 7, coarse Y = 5
        let mut r = reg(0x7000 | (5 << 5));
        r.inc_y();
        assert_eq!(r.get_fine_y(), 0);
        assert_eq!(r.get_coarse_y(), 6);
        assert_eq!(r.get_value() & 0x0800, 0); // vertical nametable unchanged
    }

    #[test]
    fn inc_y_wraps_coarse_y_at_29_and_switches_vertical_nametable() {
        // fine Y = 7, coarse Y = 29, vertical nametable bit clear
        let mut r = reg(0x7000 | (29 << 5));
        r.inc_y();
        assert_eq!(r.get_fine_y(), 0);
        assert_eq!(r.get_coarse_y(), 0);
        assert_eq!(r.get_value() & 0x0800, 0x0800); // bit 11 toggled on
    }

    #[test]
    fn inc_y_wraps_coarse_y_at_29_and_toggles_nametable_bit_back_when_already_set() {
        // fine Y = 7, coarse Y = 29, vertical nametable bit already set
        let mut r = reg(0x7000 | 0x0800 | (29 << 5));
        r.inc_y();
        assert_eq!(r.get_fine_y(), 0);
        assert_eq!(r.get_coarse_y(), 0);
        assert_eq!(r.get_value() & 0x0800, 0); // bit 11 toggled back off
    }

    #[test]
    fn inc_y_wraps_coarse_y_at_31_without_switching_nametable() {
        // fine Y = 7, coarse Y = 31 (out-of-bounds), vertical nametable bit clear
        let mut r = reg(0x7000 | (31 << 5));
        r.inc_y();
        assert_eq!(r.get_fine_y(), 0);
        assert_eq!(r.get_coarse_y(), 0);
        assert_eq!(r.get_value() & 0x0800, 0); // nametable NOT switched
    }
}
