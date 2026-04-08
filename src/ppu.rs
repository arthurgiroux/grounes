mod ppu_control;
mod ppu_mask;
mod tile_fetcher;

use crate::{
    mapper::Mapper,
    ppu::{ppu_control::PPUControl, ppu_mask::PPUMask, tile_fetcher::TileFetcher},
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

pub struct PPU {
    write_latch: WriteLatch,
    ppu_mask: PPUMask,
    ppu_control: PPUControl,
    vram: Vec<u8>,
    vram_address: u16,
    vram_read_buffer: u8,
    scroll_x: u8,
    scroll_y: u8,
    oam: Vec<u8>,
    oam_address: u8,
    status: PPUStatus,
    palette: Vec<u8>,
    scanline: u16,
    current_state_cycles: u32,
    frame_number: u32,
    pub frame: Vec<u8>,
    tile_fetcher: TileFetcher,
}

impl PPU {
    pub const IMG_HEIGHT: usize = 240;
    pub const IMG_WIDTH: usize = 256;
    pub const IMG_BPP: usize = 3;

    fn update_vram_address(&mut self, value: u8) {
        match self.write_latch.location {
            WriteLocation::First => {
                self.vram_address = (self.vram_address & 0x00FF) | (value as u16 & 0x3F) << 8;
            }
            WriteLocation::Second => {
                self.vram_address = (self.vram_address & 0xFF00) | (value as u16);
            }
        }
        self.write_latch.toggle();
    }

    fn update_scroll(&mut self, value: u8) {
        match self.write_latch.location {
            WriteLocation::First => {
                self.scroll_x = (self.scroll_x & 0x01) | (value & 0b11111110);
            }
            WriteLocation::Second => {
                self.scroll_y = (self.scroll_y & 0x01) | (value & 0b11111110);
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
            vram_address: 0,
            vram_read_buffer: 0,
            ppu_mask: PPUMask::default(),
            ppu_control: PPUControl::default(),
            scroll_x: 0,
            scroll_y: 0,
            oam: vec![0u8; 256],
            oam_address: 0,
            status: PPUStatus::from_bits_truncate(0),
            palette: vec![0u8; 32],
            scanline: 261,
            frame_number: 0,
            current_state_cycles: 0,
            frame: vec![0u8; PPU::IMG_HEIGHT * PPU::IMG_WIDTH * PPU::IMG_BPP],
            tile_fetcher: TileFetcher::default(),
        }
    }
}

impl PPU {
    fn read_byte_ppu(&mut self, mapper: &mut dyn Mapper, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => mapper.read_byte(crate::mapper::MapperSource::PPU, addr),
            0x2000..=0x3EFF => {
                let value = self.vram_read_buffer;
                let offset = (addr - 0x2000) & (self.vram.len() as u16 - 1);
                self.vram_read_buffer = self.vram[offset as usize];
                value
            }
            0x3F00..=0x3FFF => {
                let offset = (addr - 0x3F00) & (self.palette.len() as u16 - 1);
                self.palette[offset as usize]
            }
            _ => panic!("Requested address outside of PPU address space."),
        }
    }

    fn write_byte_ppu(&mut self, mapper: &mut dyn Mapper, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                mapper.write_byte(crate::mapper::MapperSource::PPU, addr, value);
            }
            0x2000..=0x3EFF => {
                let offset = (addr - 0x2000) & (self.vram.len() as u16 - 1);
                self.vram[offset as usize] = value;
            }
            0x3F00..=0x3FFF => {
                let offset = (addr - 0x3F00) & (self.palette.len() as u16 - 1);
                self.palette[offset as usize] = value;
            }
            _ => panic!("Requested address outside of PPU address space."),
        }
    }

    pub fn read_byte(&mut self, mapper: &mut dyn Mapper, addr: u16) -> u8 {
        match addr {
            ppu_reg::DATA => {
                let value = self.read_byte_ppu(mapper, self.vram_address);
                self.vram_address = self
                    .vram_address
                    .wrapping_add(self.ppu_control.get_vram_address_increment());
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
                self.scroll_x = (self.scroll_x & 0b11111110) | (value & 0x01);
                self.scroll_y = (self.scroll_y & 0b11111110) | ((value >> 1) & 0x01);
            }
            ppu_reg::MASK => {
                self.ppu_mask.update(value);
            }
            ppu_reg::ADDR => {
                self.update_vram_address(value);
            }
            ppu_reg::DATA => {
                self.write_byte_ppu(mapper, self.vram_address, value);
                self.vram_address = self
                    .vram_address
                    .wrapping_add(self.ppu_control.get_vram_address_increment());
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

                // TODO
            }
            ScanlineRendererState::VisibleScanline => {
                if self.current_state_cycles >= 1 && self.current_state_cycles <= 256 {
                    let x = (self.current_state_cycles - 1) as u16;
                    let nametable_addr = self.ppu_control.get_base_nametable_address();
                    let pattern_base = self.ppu_control.get_background_pattern_table_address();
                    let scanline = self.scanline;
                    self.tile_fetcher.step(
                        scanline,
                        x,
                        &self.vram,
                        mapper,
                        nametable_addr,
                        pattern_base,
                        &self.palette,
                        &mut self.frame,
                    );
                }
                // TODO
            }
            ScanlineRendererState::PostRender => {
                // TODO
                self.frame_number += 1;
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
            self.scanline = (self.scanline + 1) % 261;
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
        assert_eq!(ppu.vram_address, 0x2000);
    }

    #[test]
    fn update_vram_address_two_writes_should_set_full_address() {
        let mut ppu = PPU::default();
        ppu.update_vram_address(0x21);
        ppu.update_vram_address(0x50);
        assert_eq!(ppu.vram_address, 0x2150);
    }

    #[test]
    fn update_vram_address_high_byte_should_be_masked_to_6_bits() {
        let mut ppu = PPU::default();
        ppu.update_vram_address(0xFF);
        assert_eq!(ppu.vram_address, 0x3F00);
    }

    // update_scroll

    #[test]
    fn update_scroll_first_write_should_set_scroll_x() {
        let mut ppu = PPU::default();
        ppu.update_scroll(0xAA);
        assert_eq!(ppu.scroll_x, 0xAA);
    }

    #[test]
    fn update_scroll_second_write_should_set_scroll_y() {
        let mut ppu = PPU::default();
        ppu.update_scroll(0x00);
        ppu.update_scroll(0xAA);
        assert_eq!(ppu.scroll_y, 0xAA);
    }

    #[test]
    fn update_scroll_should_preserve_scroll_x_lsb() {
        let mut ppu = PPU::default();
        ppu.scroll_x = 0x01;
        // 0xAB has bit 0 set, but update_scroll masks it out and preserves the existing LSB
        ppu.update_scroll(0xAB);
        assert_eq!(ppu.scroll_x, 0xAB); // 0xAA | preserved LSB 0x01 = 0xAB
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
        ppu.vram_address = 0x2000;
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
        ppu.vram_address = 0x2000;
        ppu.read_byte(&mut mapper, ppu_reg::DATA);
        assert_eq!(ppu.vram_address, 0x2001);
    }

    #[test]
    fn read_data_should_increment_vram_address_by_32_when_control_bit_set() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::CONTROL, 0x04);
        ppu.vram_address = 0x2000;
        ppu.read_byte(&mut mapper, ppu_reg::DATA);
        assert_eq!(ppu.vram_address, 0x2020);
    }

    // write_byte CONTROL

    #[test]
    fn write_control_should_update_ppu_control() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::CONTROL, 0x80);
        assert!(ppu.ppu_control.is_vblank_nmi_enabled());
    }

    #[test]
    fn write_control_should_set_scroll_x_lsb() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::CONTROL, 0x01);
        assert_eq!(ppu.scroll_x & 0x01, 0x01);
    }

    #[test]
    fn write_control_should_set_scroll_y_lsb() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::CONTROL, 0x02);
        assert_eq!(ppu.scroll_y & 0x01, 0x01);
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
        ppu.vram_address = 0x2000;
        ppu.write_byte(&mut mapper, ppu_reg::DATA, 0x01);
        ppu.write_byte(&mut mapper, ppu_reg::DATA, 0x02);
        assert_eq!(ppu.vram_address, 0x2002);
    }

    #[test]
    fn write_data_should_auto_increment_address_by_32_when_control_bit_set() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::CONTROL, 0x04);
        ppu.vram_address = 0x2000;
        ppu.write_byte(&mut mapper, ppu_reg::DATA, 0x01);
        assert_eq!(ppu.vram_address, 0x2020);
    }

    // write_byte SCROLL

    #[test]
    fn write_scroll_first_write_should_set_scroll_x() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::SCROLL, 0xAA);
        assert_eq!(ppu.scroll_x, 0xAA);
    }

    #[test]
    fn write_scroll_second_write_should_set_scroll_y() {
        let mut ppu = PPU::default();
        let mut mapper = MockMapper::new();
        ppu.write_byte(&mut mapper, ppu_reg::SCROLL, 0x00);
        ppu.write_byte(&mut mapper, ppu_reg::SCROLL, 0xAA);
        assert_eq!(ppu.scroll_y, 0xAA);
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
