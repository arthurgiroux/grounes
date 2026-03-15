use std::usize;

use crate::ines::INES;

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
        (MemoryRegion::Cartridge, addr - 0x4020)
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

#[derive(Debug, PartialEq)]
pub enum MapperSource {
    CPU,
    PPU,
}

pub fn create_mapper(ines: INES) -> Result<Box<dyn Mapper>, String> {
    match ines.header.mapper_number {
        0 => Ok(Box::new(Mapper0 { ines })),
        n => Err(format!("unsupported mapper: {}", n)),
    }
}

pub trait Mapper {
    fn read_byte(&self, source: MapperSource, addr: u16) -> u8;
    fn write_byte(&mut self, source: MapperSource, addr: u16, value: u8);
}

struct Mapper0 {
    ines: INES,
}

impl Mapper for Mapper0 {
    fn read_byte(&self, source: MapperSource, addr: u16) -> u8 {
        let address = addr as usize;
        let value = match source {
            // PPU $0000-$1FFF: 8 KiB CHR-ROM.
            MapperSource::PPU => self.ines.chr_rom.as_deref().unwrap_or(&[]).get(address),
            MapperSource::CPU => {
                // CPU $6000-$7FFF: Unbanked PRG-RAM, mirrored as necessary to fill entire 8 KiB window, write protectable with an external switch. (Family BASIC only)
                if addr >= 0x6000 && addr <= 0x7FFF {
                    self.ines
                        .prg_ram
                        .as_deref()
                        .unwrap_or(&[])
                        .get(address - 0x6000)
                }
                // CPU $8000-$BFFF: First 16 KiB of PRG-ROM.
                else if addr >= 0x8000 && addr <= 0xBFFF {
                    self.ines.prg_rom.get(address - 0x8000)
                }
                // CPU $C000-$FFFF: Last 16 KiB of PRG-ROM (NROM-256) or mirror of $8000-$BFFF (NROM-128).
                else if addr >= 0xC000 && addr <= 0xFFFF {
                    let rom_offset = if self.ines.prg_rom.len() >= 0x4000 {
                        0x4000
                    } else {
                        0
                    };
                    self.ines.prg_rom.get(address - 0xC000 + rom_offset)
                } else {
                    None
                }
            }
        };

        value.copied().unwrap_or(0)
    }

    fn write_byte(&mut self, source: MapperSource, addr: u16, value: u8) {
        let address = addr as usize;
        match source {
            MapperSource::CPU => {
                // CPU $6000-$7FFF: PRG-RAM (writable)
                if addr >= 0x6000 && addr <= 0x7FFF {
                    if let Some(ram) = self.ines.prg_ram.as_mut() {
                        let offset = address - 0x6000;
                        if let Some(byte) = ram.get_mut(offset) {
                            *byte = value;
                        }
                    }
                }
                // $8000-$FFFF: PRG-ROM, writes ignored on NROM
            }
            MapperSource::PPU => {
                // CHR-ROM, writes ignored on NROM
            }
        }
    }
}
