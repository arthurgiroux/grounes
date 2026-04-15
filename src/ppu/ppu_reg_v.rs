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
