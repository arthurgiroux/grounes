mod ppu_control;
mod ppu_mask;
mod ppu_reg_v;
mod tile_fetcher;

use crate::{
    mapper::Mapper,
    ppu::{
        ppu_control::PPUControl, ppu_mask::PPUMask, ppu_reg_v::PPURegV, tile_fetcher::TileFetcher,
    },
};

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PPUStatus: u8 {
        const SpriteOverflow = 0b00100000;
        const Sprite0Hit = 0b01000000;
        const VBlank = 0b10000000;
    }
}

pub mod ppu_reg {
    pub const CONTROL: u16 = 0x2000;
    pub const MASK: u16 = 0x2001;
    pub const STATUS: u16 = 0x2002;
    pub const OAM_ADDR: u16 = 0x2003;
    pub const OAM_DATA: u16 = 0x2004;
    pub const SCROLL: u16 = 0x2005;
    pub const ADDR: u16 = 0x2006;
    pub const DATA: u16 = 0x2007;
}

pub mod nametable {
    pub const HEIGHT: usize = 30;
    pub const WIDTH: usize = 32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanlineRendererState {
    PreRender,
    VisibleScanline,
    PostRender,
    Vblank,
}

const SCANLINE_CYCLE_DURATION: u32 = 341;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteLocation {
    First,
    Second,
}

struct WriteLatch {
    pub location: WriteLocation,
}

impl WriteLatch {
    pub fn toggle(&mut self) {
        if self.location == WriteLocation::First {
            self.location = WriteLocation::Second
        } else {
            self.location = WriteLocation::First
        }
    }

    pub fn clear(&mut self) {
        self.location = WriteLocation::First
    }
}

impl Default for WriteLatch {
    fn default() -> Self {
        WriteLatch {
            location: WriteLocation::First,
        }
    }
}

// Standard NES NTSC system palette (64 colors, RGB)
const SYSTEM_PALETTE: [(u8, u8, u8); 64] = [
    (84, 84, 84),
    (0, 30, 116),
    (8, 16, 144),
    (48, 0, 136),
    (68, 0, 100),
    (92, 0, 48),
    (84, 4, 0),
    (60, 24, 0),
    (32, 42, 0),
    (8, 58, 0),
    (0, 64, 0),
    (0, 60, 0),
    (0, 50, 60),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (152, 150, 152),
    (8, 76, 196),
    (48, 50, 236),
    (92, 30, 228),
    (136, 20, 176),
    (160, 20, 100),
    (152, 34, 32),
    (120, 60, 0),
    (84, 90, 0),
    (40, 114, 0),
    (8, 124, 0),
    (0, 118, 40),
    (0, 102, 120),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (236, 238, 236),
    (76, 154, 236),
    (120, 124, 236),
    (176, 98, 236),
    (228, 84, 236),
    (236, 88, 180),
    (236, 106, 100),
    (212, 136, 32),
    (160, 170, 0),
    (116, 196, 0),
    (76, 208, 32),
    (56, 204, 108),
    (56, 180, 204),
    (60, 60, 60),
    (0, 0, 0),
    (0, 0, 0),
    (236, 238, 236),
    (168, 204, 236),
    (188, 188, 236),
    (212, 178, 236),
    (236, 174, 236),
    (236, 174, 212),
    (236, 180, 176),
    (228, 196, 144),
    (204, 210, 120),
    (180, 222, 120),
    (168, 226, 144),
    (152, 226, 180),
    (160, 214, 228),
    (160, 162, 160),
    (0, 0, 0),
    (0, 0, 0),
];

pub struct PPU {
    write_latch: WriteLatch,
    ppu_mask: PPUMask,
    ppu_control: PPUControl,
    vram: Vec<u8>,
    vram_read_buffer: u8,
    oam: Vec<u8>,
    oam_address: u8,
    status: PPUStatus,
    scanline: u16,
    current_state_cycles: u32,
    frame_number: u32,
    pub frame: Vec<u8>,
    tile_fetcher: TileFetcher,
    reg_v: PPURegV,
    reg_t: u16,
    fine_x: u8,
    palette: Vec<u8>,
}

impl PPU {
    pub const IMG_HEIGHT: usize = 240;
    pub const IMG_WIDTH: usize = 256;
    pub const IMG_BPP: usize = 3;

    fn update_vram_address(&mut self, value: u8) {
        match self.write_latch.location {
            WriteLocation::First => {
                // t: .CDEFGH ........ <- d: ..CDEFGH
                // t: Z...... ........ <- 0 (bit Z is cleared)
                self.reg_t = (self.reg_t & 0x00FF) | (value as u16 & 0x3F) << 8;
            }
            WriteLocation::Second => {
                // t: ....... ABCDEFGH <- d: ABCDEFGH
                // v: <...all bits...> <- t: <...all bits...>
                self.reg_t = (self.reg_t & 0xFF00) | (value as u16);
                self.reg_v.set_value(self.reg_t);
            }
        }
        self.write_latch.toggle();
    }

    fn is_rendering_enabled(&self) -> bool {
        self.ppu_mask.is_background_rendering_enabled()
            || self.ppu_mask.is_sprite_rendering_enabled()
    }

    fn update_scroll(&mut self, value: u8) {
        match self.write_latch.location {
            WriteLocation::First => {
                self.fine_x = value & 0x07;
                // Copy the 5 upper bits of the value to the last 5 bits of t
                // t: ....... ...ABCDE <- d: ABCDE...
                self.reg_t = (self.reg_t & 0xFFE0) | (value as u16 >> 3);
            }
            WriteLocation::Second => {
                // t: FGH..AB CDE..... <- d: ABCDEFGH
                let value16 = value as u16;
                self.reg_t =
                    ((value16 & 0x07) << 12) | ((value16 & 0xF8) << 2) | (self.reg_t & 0x0C1F);
            }
        }
        self.write_latch.toggle();
    }
}

impl Default for PPU {
    fn default() -> Self {
        PPU {
            write_latch: WriteLatch::default(),
            vram: vec![0u8; 2048],
            palette: vec![0u8; 64],
            vram_read_buffer: 0,
            ppu_mask: PPUMask::default(),
            ppu_control: PPUControl::default(),
            oam: vec![0u8; 256],
            oam_address: 0,
            status: PPUStatus::from_bits_truncate(0),
            scanline: 261,
            frame_number: 0,
            current_state_cycles: 0,
            frame: vec![0u8; PPU::IMG_HEIGHT * PPU::IMG_WIDTH * PPU::IMG_BPP],
            tile_fetcher: TileFetcher::default(),
            reg_v: PPURegV::default(),
            reg_t: 0,
            fine_x: 0,
        }
    }
}

impl PPU {
    fn read_byte_ppu(&mut self, mapper: &mut dyn Mapper, addr: u16) -> u8 {
        match addr & 0x3FFF {
            0x0000..=0x1FFF => mapper.read_byte(crate::mapper::MapperSource::PPU, addr),
            0x2000..=0x3EFF => {
                let value = self.vram_read_buffer;
                let offset = (addr - 0x2000) as usize % self.vram.len();
                self.vram_read_buffer = self.vram[offset];
                value
            }
            0x3F00..=0x3FFF => {
                let offset = (addr - 0x3F00) as usize % self.palette.len();
                self.palette[offset]
            }
            _ => 0x00, //_ => panic!("Requested address outside of PPU address space."),
        }
    }

    fn write_byte_ppu(&mut self, mapper: &mut dyn Mapper, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                mapper.write_byte(crate::mapper::MapperSource::PPU, addr, value);
            }
            0x2000..=0x3EFF => {
                let offset = (addr - 0x2000) as usize % self.vram.len();
                self.vram[offset] = value;
            }
            0x3F00..=0x3FFF => {
                let offset = (addr - 0x3F00) as usize % self.palette.len();
                self.palette[offset] = value;
            }
            _ => {} //_ => panic!("Requested address outside of PPU address space. {:2X}", addr),
        }
    }

    pub fn read_byte(&mut self, mapper: &mut dyn Mapper, addr: u16) -> u8 {
        match addr {
            ppu_reg::DATA => {
                let value = self.read_byte_ppu(mapper, self.reg_v.get_value());
                self.reg_v.set_value(
                    self.reg_v
                        .get_value()
                        .wrapping_add(self.ppu_control.get_vram_address_increment()),
                );
                value
            }
            ppu_reg::OAM_DATA => self.oam[self.oam_address as usize],
            ppu_reg::STATUS => {
                let value = self.status.bits();
                self.status.remove(PPUStatus::VBlank);
                self.write_latch.clear();
                value
            }
            _ => 0,
        }
    }

    pub fn write_byte(&mut self, mapper: &mut dyn Mapper, addr: u16, value: u8) {
        match addr {
            ppu_reg::CONTROL => {
                self.ppu_control.update(value);
                // Copy the last two bits of the value to the 11/12th bits of reg_t
                self.reg_t = (self.reg_t & 0xF3FF) | ((value as u16 & 0x03) << 10);
            }
            ppu_reg::MASK => {
                self.ppu_mask.update(value);
            }
            ppu_reg::ADDR => {
                self.update_vram_address(value);
            }
            ppu_reg::DATA => {
                self.write_byte_ppu(mapper, self.reg_v.get_value(), value);
                self.reg_v.set_value(
                    self.reg_v
                        .get_value()
                        .wrapping_add(self.ppu_control.get_vram_address_increment()),
                );
            }
            ppu_reg::SCROLL => {
                self.update_scroll(value);
            }
            ppu_reg::OAM_ADDR => {
                self.oam_address = value;
            }
            ppu_reg::OAM_DATA => {
                self.oam[self.oam_address as usize] = value;
                self.oam_address = self.oam_address.wrapping_add(1);
            }

            _ => {}
        }
    }

    pub fn step(&mut self, mapper: &mut dyn Mapper) {
        match self.get_state() {
            ScanlineRendererState::PreRender => {
                if self.current_state_cycles == 1 {
                    self.status.remove(PPUStatus::VBlank);
                }

                if self.is_rendering_enabled() {
                    // v: ....A.. ...BCDEF <- t: ....A.. ...BCDEF
                    if self.current_state_cycles == 257 {
                        self.reg_v
                            .set_value((self.reg_t & 0x041F) | (self.reg_v.get_value() & !0x041F));
                    }

                    if self.current_state_cycles >= 280 && self.current_state_cycles <= 304 {
                        // At the end of vblank, shortly after the horizontal bits are copied from t to v at dot 257,
                        // the PPU will repeatedly copy the vertical bits from t to v from dots 280 to 304, completing the full initialization of v from t:
                        // v: GHIA.BC DEF..... <- t: GHIA.BC DEF.....
                        self.reg_v
                            .set_value((self.reg_t & 0x7BE0) | (self.reg_v.get_value() & !0x7BE0));
                    }
                }
            }
            ScanlineRendererState::VisibleScanline => {
                if self.current_state_cycles >= 1 && self.current_state_cycles <= 256 {
                    let pattern_base = self.ppu_control.get_background_pattern_table_address();
                    let pixels =
                        self.tile_fetcher
                            .step(&self.reg_v, pattern_base, &self.vram, mapper);

                    if let Some(pixels) = pixels {
                        let x_start = self.reg_v.get_coarse_x() as usize * 8;
                        let y = self.scanline as usize;
                        for (i, &palette_idx) in pixels.iter().enumerate() {
                            // Subtract fine_x to shift the viewport right by fine_x pixels.
                            // wrapping_sub produces a value >= IMG_WIDTH for pixels that would
                            // land before x=0, which the bounds check discards.
                            let x = (x_start + i).wrapping_sub(self.fine_x as usize);
                            if x >= PPU::IMG_WIDTH {
                                continue;
                            }
                            let system_color_idx =
                                self.palette[palette_idx as usize & 0x1F] as usize;
                            let (r, g, b) = SYSTEM_PALETTE[system_color_idx & 0x3F];
                            let frame_offset = (y * PPU::IMG_WIDTH + x) * PPU::IMG_BPP;
                            self.frame[frame_offset] = r;
                            self.frame[frame_offset + 1] = g;
                            self.frame[frame_offset + 2] = b;
                        }
                        if self.is_rendering_enabled() {
                            self.reg_v.inc_coarse_x();
                        }
                    }

                    if self.current_state_cycles == 256 && self.is_rendering_enabled() {
                        self.reg_v.inc_y();
                    }
                }

                if self.current_state_cycles == 257 && self.is_rendering_enabled() {
                    // The PPU copies all bits related to horizontal position from t to v:
                    // v: ....A.. ...BCDEF <- t: ....A.. ...BCDEF
                    self.reg_v
                        .set_value((self.reg_t & 0x041F) | (self.reg_v.get_value() & !0x041F));
                }
                // TODO
            }
            ScanlineRendererState::PostRender => {
                // TODO
                if self.current_state_cycles == 0 {
                    self.frame_number += 1;
                }
            }
            ScanlineRendererState::Vblank => {
                if self.scanline == 241 && self.current_state_cycles == 1 {
                    self.status.insert(PPUStatus::VBlank);
                }
            }
        }

        self.current_state_cycles += 1;
        if self.current_state_cycles == SCANLINE_CYCLE_DURATION {
            self.current_state_cycles = 0;
            self.scanline = (self.scanline + 1) % 262;
        }
    }

    fn get_state(&self) -> ScanlineRendererState {
        match self.scanline {
            261 => ScanlineRendererState::PreRender,
            0..=239 => ScanlineRendererState::VisibleScanline,
            240 => ScanlineRendererState::PostRender,
            241..=260 => ScanlineRendererState::Vblank,
            _ => panic!("Scanline value not handled"),
        }
    }

    pub fn is_new_frame_ready(&self) -> bool {
        return self.get_state() == ScanlineRendererState::PostRender
            && self.current_state_cycles == 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapper::MapperSource;

    struct MockMapper {
        data: Vec<u8>,
    }

    impl MockMapper {
        fn new() -> Self {
            MockMapper {
                data: vec![0u8; 0x2000],
            }
        }
    }

    impl crate::mapper::Mapper for MockMapper {
        fn read_byte(&self, _source: MapperSource, addr: u16) -> u8 {
            self.data[addr as usize]
        }

        fn write_byte(&mut self, _source: MapperSource, addr: u16, value: u8) {
            self.data[addr as usize] = value;
        }
    }

    // WriteLatch

    #[test]
    fn write_latch_toggle_should_switch_location() {
        let mut latch = WriteLatch::default();
        assert_eq!(latch.location, WriteLocation::First);
        latch.toggle();
        assert_eq!(latch.location, WriteLocation::Second);
        latch.toggle();
        assert_eq!(latch.location, WriteLocation::First);
    }

    #[test]
    fn write_latch_clear_should_reset_to_first() {
        let mut latch = WriteLatch::default();
        latch.toggle();
        assert_eq!(latch.location, WriteLocation::Second);
        latch.clear();
        assert_eq!(latch.location, WriteLocation::First);
    }

    // update_vram_address

    #[test]
    fn update_vram_address_first_write_should_set_high_byte() {
        let mut ppu = PPU::default();
        ppu.update_vram_address(0x20);
        assert_eq!(ppu.reg_t, 0x2000);
    }

    #[test]
    fn update_vram_address_two_writes_should_set_full_address() {
        let mut ppu = PPU::default();
        ppu.update_vram_address(0x21);
        ppu.update_vram_address(0x50);
        assert_eq!(ppu.reg_t, 0x2150);
        assert_eq!(ppu.reg_v.get_value(), 0x2150);
    }

    // read_byte STATUS

    #[test]
    fn read_status_should_return_status_bits() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.status.insert(PPUStatus::VBlank);
        let value = ppu.read_byte(&mut mapper, ppu_reg::STATUS);
        assert_eq!(value & PPUStatus::VBlank.bits(), PPUStatus::VBlank.bits());
    }

    #[test]
    fn read_status_should_clear_vblank_flag() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.status.insert(PPUStatus::VBlank);
        ppu.read_byte(&mut mapper, ppu_reg::STATUS);
        assert!(!ppu.status.contains(PPUStatus::VBlank));
    }

    #[test]
    fn read_status_should_reset_write_latch() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::ADDR, 0x21);
        assert_eq!(ppu.write_latch.location, WriteLocation::Second);
        ppu.read_byte(&mut mapper, ppu_reg::STATUS);
        assert_eq!(ppu.write_latch.location, WriteLocation::First);
    }

    // read_byte OAM_DATA

    #[test]
    fn read_oam_data_should_return_byte_at_oam_address() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.oam[0x10] = 0xBE;
        ppu.oam_address = 0x10;
        assert_eq!(ppu.read_byte(&mut mapper, ppu_reg::OAM_DATA), 0xBE);
    }

    // read_byte DATA

    #[test]
    fn read_data_should_return_buffered_value() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.reg_v.set_value(0x2000);
        ppu.vram[0] = 0xAB;
        // First read returns old buffer (empty)
        assert_eq!(ppu.read_byte(&mut mapper, ppu_reg::DATA), 0x00);
        // Second read returns the value that was loaded into the buffer
        assert_eq!(ppu.read_byte(&mut mapper, ppu_reg::DATA), 0xAB);
    }

    #[test]
    fn read_data_should_increment_vram_address_by_1() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.reg_v.set_value(0x2000);
        ppu.read_byte(&mut mapper, ppu_reg::DATA);
        assert_eq!(ppu.reg_v.get_value(), 0x2001);
    }

    #[test]
    fn read_data_should_increment_vram_address_by_32_when_control_bit_set() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::CONTROL, 0x04);
        ppu.reg_v.set_value(0x2000);
        ppu.read_byte(&mut mapper, ppu_reg::DATA);
        assert_eq!(ppu.reg_v.get_value(), 0x2020);
    }

    // write_byte CONTROL

    #[test]
    fn write_control_should_update_ppu_control() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::CONTROL, 0x80);
        assert!(ppu.ppu_control.is_vblank_nmi_enabled());
    }

    // write_byte MASK

    #[test]
    fn write_mask_should_update_ppu_mask() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::MASK, 0x01);
        assert!(ppu.ppu_mask.is_greyscale());
    }

    // write_byte ADDR + DATA

    #[test]
    fn write_addr_two_writes_should_set_vram_address() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::ADDR, 0x21);
        ppu.write_byte(&mut mapper, ppu_reg::ADDR, 0x00);
        ppu.write_byte(&mut mapper, ppu_reg::DATA, 0xAB);
        assert_eq!(ppu.vram[0x0100], 0xAB);
    }

    #[test]
    fn write_data_should_auto_increment_address_by_1() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.reg_v.set_value(0x2000);
        ppu.write_byte(&mut mapper, ppu_reg::DATA, 0x01);
        ppu.write_byte(&mut mapper, ppu_reg::DATA, 0x02);
        assert_eq!(ppu.reg_v.get_value(), 0x2002);
    }

    #[test]
    fn write_data_should_auto_increment_address_by_32_when_control_bit_set() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::CONTROL, 0x04);
        ppu.reg_v.set_value(0x2000);
        ppu.write_byte(&mut mapper, ppu_reg::DATA, 0x01);
        assert_eq!(ppu.reg_v.get_value(), 0x2020);
    }

    // write_byte OAM_ADDR + OAM_DATA

    #[test]
    fn write_oam_addr_should_set_oam_address() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::OAM_ADDR, 0x10);
        assert_eq!(ppu.oam_address, 0x10);
    }

    #[test]
    fn write_oam_data_should_write_to_oam_and_increment_address() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::OAM_ADDR, 0x05);
        ppu.write_byte(&mut mapper, ppu_reg::OAM_DATA, 0xBE);
        assert_eq!(ppu.oam[0x05], 0xBE);
        assert_eq!(ppu.oam_address, 0x06);
    }

    #[test]
    fn write_oam_data_should_wrap_oam_address() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::OAM_ADDR, 0xFF);
        ppu.write_byte(&mut mapper, ppu_reg::OAM_DATA, 0x00);
        assert_eq!(ppu.oam_address, 0x00);
    }
}
