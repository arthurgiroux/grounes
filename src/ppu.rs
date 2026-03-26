#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteSize {
    Size8x8,
    Size8x16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtMode {
    Master,
    Slave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PPUControl {
    // 7  bit  0
    // ---- ----
    // VPHB SINN
    // |||| ||||
    // |||| ||++- Base nametable address
    // |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    // |||| |+--- VRAM address increment per CPU read/write of PPUDATA
    // |||| |     (0: add 1, going across; 1: add 32, going down)
    // |||| +---- Sprite pattern table address for 8x8 sprites
    // ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
    // |||+------ Background pattern table address (0: $0000; 1: $1000)
    // ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
    // |+-------- PPU master/slave select
    // |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
    // +--------- Vblank NMI enable (0: off, 1: on)
    pub value: u8,
}

impl PPUControl {
    pub fn get_base_nametable_address(&self) -> u16 {
        match self.value & 0b00000011 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => panic!("Unhandled base nametable address value"),
        }
    }

    pub fn get_vram_address_increment(&self) -> u16 {
        if self.value & 0b00000100 == 0 { 1 } else { 32 }
    }

    pub fn get_sprite_pattern_table_address(&self) -> u16 {
        if self.value & 0b00001000 == 0 {
            0x0000
        } else {
            0x1000
        }
    }

    pub fn get_background_pattern_table_address(&self) -> u16 {
        if self.value & 0b00010000 == 0 {
            0x0000
        } else {
            0x1000
        }
    }

    pub fn get_sprite_size(&self) -> SpriteSize {
        if self.value & 0b00100000 == 0 {
            SpriteSize::Size8x8
        } else {
            SpriteSize::Size8x16
        }
    }

    pub fn get_ext_mode(&self) -> ExtMode {
        if self.value & 0b01000000 == 0 {
            ExtMode::Master
        } else {
            ExtMode::Slave
        }
    }

    pub fn is_vblank_nmi_enabled(&self) -> bool {
        self.value & 0b10000000 > 0
    }
}

pub struct PPUMask {
    // 7  bit  0
    // ---- ----
    // BGRs bMmG
    // |||| ||||
    // |||| |||+- Greyscale (0: normal color, 1: greyscale)
    // |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
    // |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
    // |||| +---- 1: Enable background rendering
    // |||+------ 1: Enable sprite rendering
    // ||+------- Emphasize red (green on PAL/Dendy)
    // |+-------- Emphasize green (red on PAL/Dendy)
    // +--------- Emphasize blue
    pub value: u8,
}

impl PPUMask {
    pub fn is_greyscale(self) -> bool {
        self.value & 0x01 > 0
    }

    pub fn show_background_in_leftmost_8_pixels(self) -> bool {
        self.value & 0x02 > 0
    }

    pub fn show_sprites_in_leftmost_8_pixels(self) -> bool {
        self.value & 0x04 > 0
    }

    pub fn is_background_rendering_enabled(self) -> bool {
        self.value & 0x08 > 0
    }

    pub fn is_sprite_rendering_enabled(self) -> bool {
        self.value & 0x10 > 0
    }

    pub fn is_red_emphasized(self) -> bool {
        self.value & 0x20 > 0
    }

    pub fn is_green_emphasized(self) -> bool {
        self.value & 0x40 > 0
    }

    pub fn is_blue_emphasized(self) -> bool {
        self.value & 0x80 > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_name_table_should_return_correct_value() {
        assert_eq!(
            (PPUControl { value: 0xF0 }).get_base_nametable_address(),
            0x2000
        );
        assert_eq!(
            (PPUControl { value: 0xF1 }).get_base_nametable_address(),
            0x2400
        );
        assert_eq!(
            (PPUControl { value: 0xF2 }).get_base_nametable_address(),
            0x2800
        );
        assert_eq!(
            (PPUControl { value: 0xF3 }).get_base_nametable_address(),
            0x2C00
        );
    }

    #[test]
    fn get_vram_address_increment_should_return_correct_value() {
        assert_eq!((PPUControl { value: 0xF0 }).get_vram_address_increment(), 1);
        assert_eq!(
            (PPUControl { value: 0xF4 }).get_vram_address_increment(),
            32
        );
    }

    #[test]
    fn get_sprite_pattern_table_address_should_return_correct_value() {
        assert_eq!(
            (PPUControl { value: 0xF0 }).get_sprite_pattern_table_address(),
            0x0
        );
        assert_eq!(
            (PPUControl { value: 0xF8 }).get_sprite_pattern_table_address(),
            0x1000
        );
    }

    #[test]
    fn get_background_pattern_table_address_should_return_correct_value() {
        assert_eq!(
            (PPUControl { value: 0x0F }).get_background_pattern_table_address(),
            0x0
        );
        assert_eq!(
            (PPUControl { value: 0x10 }).get_background_pattern_table_address(),
            0x1000
        );
    }

    #[test]
    fn get_sprite_size_should_return_correct_value() {
        assert_eq!(
            (PPUControl { value: 0x0F }).get_sprite_size(),
            SpriteSize::Size8x8
        );
        assert_eq!(
            (PPUControl { value: 0x20 }).get_sprite_size(),
            SpriteSize::Size8x16
        );
    }

    #[test]
    fn get_ext_mode_should_return_correct_value() {
        assert_eq!((PPUControl { value: 0x0F }).get_ext_mode(), ExtMode::Master);
        assert_eq!((PPUControl { value: 0x40 }).get_ext_mode(), ExtMode::Slave);
    }

    #[test]
    fn is_vblank_nmi_enabled_should_return_correct_value() {
        assert_eq!((PPUControl { value: 0x0F }).is_vblank_nmi_enabled(), false);
        assert_eq!((PPUControl { value: 0x80 }).is_vblank_nmi_enabled(), true);
    }

    #[test]
    fn is_greyscale_return_correct_value() {
        assert!(!(PPUMask { value: 0xF0 }).is_greyscale());
        assert!((PPUMask { value: 0xF1 }).is_greyscale());
    }

    #[test]
    fn show_background_return_correct_value() {
        assert!(!(PPUMask { value: 0xF0 }).show_background_in_leftmost_8_pixels());
        assert!((PPUMask { value: 0xF2 }).show_background_in_leftmost_8_pixels());
    }

    #[test]
    fn show_sprites_return_correct_value() {
        assert!(!(PPUMask { value: 0xF0 }).show_sprites_in_leftmost_8_pixels());
        assert!((PPUMask { value: 0xF4 }).show_sprites_in_leftmost_8_pixels());
    }

    #[test]
    fn is_background_rendering_enabled_return_correct_value() {
        assert!(!(PPUMask { value: 0xF0 }).is_background_rendering_enabled());
        assert!((PPUMask { value: 0xF8 }).is_background_rendering_enabled());
    }

    #[test]
    fn is_sprites_rendering_enabled_return_correct_value() {
        assert!(!(PPUMask { value: 0x0F }).is_sprite_rendering_enabled());
        assert!((PPUMask { value: 0x10 }).is_sprite_rendering_enabled());
    }

    #[test]
    fn is_red_emphasized_return_correct_value() {
        assert!(!(PPUMask { value: 0x0F }).is_red_emphasized());
        assert!((PPUMask { value: 0x20 }).is_red_emphasized());
    }

    #[test]
    fn is_green_emphasized_return_correct_value() {
        assert!(!(PPUMask { value: 0x0F }).is_green_emphasized());
        assert!((PPUMask { value: 0x40 }).is_green_emphasized());
    }

    #[test]
    fn is_blue_emphasized_return_correct_value() {
        assert!(!(PPUMask { value: 0x0F }).is_blue_emphasized());
        assert!((PPUMask { value: 0x80 }).is_blue_emphasized());
    }
}
