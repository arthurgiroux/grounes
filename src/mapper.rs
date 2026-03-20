use crate::ines::INES;

/// Creates a mapper based on a ROM file (format INES)
pub fn create_mapper(ines: INES) -> Result<Box<dyn Mapper>, String> {
    match ines.header.mapper_number {
        0 => Ok(Box::new(Mapper0 { ines })),
        n => Err(format!("unsupported mapper: {}", n)),
    }
}

#[derive(Debug, PartialEq)]
pub enum MapperSource {
    CPU,
    PPU,
}

/// A mapper maps memory from the CPU/PPU to RAM/ROM in the cartridge.
pub trait Mapper {
    fn read_byte(&self, source: MapperSource, addr: u16) -> u8;
    fn write_byte(&mut self, source: MapperSource, addr: u16, value: u8);
}

/// Mapper 0 (NROM), see: https://www.nesdev.org/wiki/INES_Mapper_000
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
                    let ram_size = self.ines.prg_ram.as_deref().unwrap_or_default().len();
                    let addr_offset = address - 0x6000;
                    self.ines
                        .prg_ram
                        .as_deref()
                        .unwrap_or(&[])
                        .get(addr_offset % ram_size)
                }
                // CPU $8000-$BFFF: First 16 KiB of PRG-ROM.
                else if addr >= 0x8000 && addr <= 0xBFFF {
                    self.ines.prg_rom.get(address - 0x8000)
                }
                // CPU $C000-$FFFF: Last 16 KiB of PRG-ROM (NROM-256) or mirror of $8000-$BFFF (NROM-128).
                else if addr >= 0xC000 {
                    // If the ROM is bigger than 16KiB we read from there otherwise we mirror the first 16KiB
                    let rom_offset = if self.ines.prg_rom.len() > 0x4000 {
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
