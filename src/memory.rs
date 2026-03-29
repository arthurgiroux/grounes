use crate::{mapper::{Mapper, MapperSource}, ppu::PPU};
use std::usize;

pub trait MemoryBus {
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, value: u8);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegion {
    Ram,
    PPU,
    OpenBus,
    Cartridge,
}

/// Pure address-to-region mapping. Returns (region, local_offset).
/// Ram: 0x0000..0x2000 with mirroring (offset = addr % 2048).
pub fn map_address(addr: u16) -> (MemoryRegion, u16) {
    match addr {
        0x0000..=0x1FFF => (MemoryRegion::Ram, addr % 2048),
        // Mirrored after 0x2007
        0x2000..=0x3FFF => (MemoryRegion::PPU, addr & 0x2007),
        0x4020..=0xFFFF => (MemoryRegion::Cartridge, addr),
        _ => (MemoryRegion::OpenBus, 0),
    }
}

/// A memory page is crossed after an increment operation when the high-byte is increased.
pub fn is_memory_page_crossed(base_addr: u16, incremented_addr: u16) -> bool {
    (base_addr & 0xFF00) != (incremented_addr & 0xFF00)
}

pub struct RAM {
    pub memory: Vec<u8>,
}

impl RAM {
    pub fn new(size: usize) -> Self {
        RAM {
            memory: vec![0; size],
        }
    }
}

impl MemoryBus for RAM {
    fn read_byte(&mut self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }
}

/// View over device(s) used at step time; implements MemoryBus using map_address.
pub struct BusView<'a> {
    pub ram: &'a mut RAM,
    pub ppu: &'a mut PPU,
    pub mapper: &'a mut dyn Mapper,
}

impl MemoryBus for BusView<'_> {
    fn read_byte(&mut self, addr: u16) -> u8 {
        let (region, offset) = map_address(addr);
        match region {
            MemoryRegion::Ram => self.ram.read_byte(offset),
            MemoryRegion::PPU => self.ppu.read_byte(offset),
            MemoryRegion::Cartridge => self.mapper.read_byte(MapperSource::CPU, offset),
            MemoryRegion::OpenBus => 0,
        }
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        let (region, offset) = map_address(addr);
        match region {
            MemoryRegion::Ram => self.ram.write_byte(offset, value),
            MemoryRegion::Cartridge => self.mapper.write_byte(MapperSource::CPU, offset, value),
            MemoryRegion::PPU => self.ppu.write_byte(addr, value),
            _ => {}
        }
    }
}
