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
    value: u8,
}

impl PPUMask {
    pub fn is_greyscale(&self) -> bool {
        self.value & 0x01 > 0
    }

    pub fn show_background_in_leftmost_8_pixels(&self) -> bool {
        self.value & 0x02 > 0
    }

    pub fn show_sprites_in_leftmost_8_pixels(&self) -> bool {
        self.value & 0x04 > 0
    }

    pub fn is_background_rendering_enabled(&self) -> bool {
        self.value & 0x08 > 0
    }

    pub fn is_sprite_rendering_enabled(&self) -> bool {
        self.value & 0x10 > 0
    }

    pub fn is_red_emphasized(&self) -> bool {
        self.value & 0x20 > 0
    }

    pub fn is_green_emphasized(&self) -> bool {
        self.value & 0x40 > 0
    }

    pub fn is_blue_emphasized(&self) -> bool {
        self.value & 0x80 > 0
    }

    pub fn update(&mut self, value: u8) {
        self.value = value;
    }
}

impl Default for PPUMask {
    fn default() -> Self {
        PPUMask { value: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
