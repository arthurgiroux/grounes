use std::usize;
use crate::mapper::{Mapper, MapperSource};

pub trait MemoryBus {
    fn read_byte(&self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, value: u8);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegion {
    Ram,
    OpenBus,
    Cartridge,
}

/// Pure address-to-region mapping. Returns (region, local_offset).
/// Ram: 0x0000..0x2000 with mirroring (offset = addr % 2048).
pub fn map_address(addr: u16) -> (MemoryRegion, u16) {
    if addr <= 0x1FFF {
        (MemoryRegion::Ram, addr % 2048)
    } else if addr >= 0x4020 {
        (MemoryRegion::Cartridge, addr)
    } else {
        (MemoryRegion::OpenBus, 0)
    }
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
    fn read_byte(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }
}

/// View over device(s) used at step time; implements MemoryBus using map_address.
pub struct BusView<'a> {
    pub ram: &'a mut RAM,
    pub mapper: &'a mut dyn Mapper,
}

impl MemoryBus for BusView<'_> {
    fn read_byte(&self, addr: u16) -> u8 {
        let (region, offset) = map_address(addr);
        match region {
            MemoryRegion::Ram => self.ram.read_byte(offset),
            MemoryRegion::Cartridge => self.mapper.read_byte(MapperSource::CPU, offset),
            MemoryRegion::OpenBus => 0,
        }
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        let (region, offset) = map_address(addr);
        if let MemoryRegion::Ram = region {
            self.ram.write_byte(offset, value);
        }
        if let MemoryRegion::Cartridge = region {
            self.mapper.write_byte(MapperSource::CPU, offset, value);
        }
    }
}