mod ppu_control;
mod ppu_mask;

use crate::ppu::{ppu_control::PPUControl, ppu_mask::PPUMask};

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
    pub const OAM_DMA: u16 = 0x4014;
}

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
}

impl PPU {
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
        }
    }
}

impl PPU {
    pub fn read_byte(&mut self, addr: u16) -> u8 {
        match addr & 0x2007 {
            ppu_reg::DATA => {
                let value = self.vram_read_buffer;
                self.vram_read_buffer = self
                    .vram
                    .get(self.vram_address as usize)
                    .copied()
                    .unwrap_or(0);
                self.vram_address = self
                    .vram_address
                    .wrapping_add(self.ppu_control.get_vram_address_increment());
                value
            }
            ppu_reg::OAM_DATA => self.oam[self.oam_address as usize],
            ppu_reg::STATUS => {
                let value = self.status.bits();
                self.status.set(PPUStatus::VBlank, false);
                self.write_latch.clear();
                value
            }
            _ => 0,
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr & 0x2007 {
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
                if let Some(item) = self.vram.get_mut(self.vram_address as usize) {
                    *item = value;
                }
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
}
