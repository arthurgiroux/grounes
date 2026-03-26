mod ppu_control;
mod ppu_mask;
use crate::{
    memory::MemoryBus,
    ppu::{ppu_control::PPUControl, ppu_mask::PPUMask},
};

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
    LowByte,
    HighByte,
}

struct WriteLatch {
    pub location: WriteLocation,
}

impl WriteLatch {
    pub fn toggle(&mut self) {
        if self.location == WriteLocation::LowByte {
            self.location = WriteLocation::HighByte
        } else {
            self.location = WriteLocation::LowByte
        }
    }

    pub fn clear(&mut self) {
        self.location = WriteLocation::LowByte
    }

    pub fn update_target_value(&mut self, value: u8, target: &mut u16) {
        match self.location {
            WriteLocation::LowByte => {
                *target = (*target & 0xFF00) | (value as u16);
            }
            WriteLocation::HighByte => {
                *target = (*target & 0x00FF) | ((value as u16) << 8);
            }
        }
        self.toggle();
    }
}

impl Default for WriteLatch {
    fn default() -> Self {
        WriteLatch {
            location: WriteLocation::LowByte,
        }
    }
}

struct PPU {
    write_latch: WriteLatch,
    ppu_mask: PPUMask,
    ppu_control: PPUControl,
    vram: Vec<u8>,
    vram_address: u16,
}

impl Default for PPU {
    fn default() -> Self {
        PPU {
            write_latch: WriteLatch::default(),
            vram: vec![0u8; 2048],
            vram_address: 0,
            ppu_mask: PPUMask::default(),
            ppu_control: PPUControl::default(),
        }
    }
}

impl MemoryBus for PPU {
    fn read_byte(&self, addr: u16) -> u8 {
        todo!()
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        match addr & 0x2007 {
            ppu_reg::CONTROL => {
                self.ppu_control.update(value);
            }
            ppu_reg::MASK => {
                self.ppu_mask.update(value);
            }
            ppu_reg::ADDR => {
                self.write_latch
                    .update_target_value(value, &mut self.vram_address);
            }

            _ => {}
        }
    }
}
