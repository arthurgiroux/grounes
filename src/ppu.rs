mod ppu_control;
mod ppu_mask;

use crate::{
    mapper::Mapper,
    ppu::{ppu_control::PPUControl, ppu_mask::PPUMask},
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
    palette: Vec<u8>,
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
            palette: vec![0u8; 32],
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
                self.status.set(PPUStatus::VBlank, false);
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
}
