use std::io::BufReader;
use std::path::Path;
use std::result::Result;
use std::{fs::File, io::Read};

pub struct INES {
    pub header: INESHeader,
    /// Trainer, if present (0 or 512 bytes)
    pub trainer: Option<Vec<u8>>,
    /// PRG ROM data (16384 * x bytes)
    pub prg_rom: Vec<u8>,
    /// CHR ROM data, if present (8192 * y bytes)
    pub chr_rom: Option<Vec<u8>>,
    /// PlayChoice INST-ROM, if present (0 or 8192 bytes)
    pub playchoice_inst_rom: Option<Vec<u8>>,
    /// PlayChoice PROM, if present (16 bytes Data, 16 bytes CounterOut) (this is often missing; see PC10 ROM-Images for details)
    pub playchoice_prom: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum InesParseError {
    HeaderTooShort { size: usize },
    InvalidHeader,
    Io(std::io::Error),
}

impl std::fmt::Display for InesParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HeaderTooShort { size } => {
                write!(f, "iNES header too short: got {} bytes, need 16", size)
            }
            Self::InvalidHeader => write!(f, "invalid iNES magic (expected NES\\x1A)"),
            Self::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl From<std::io::Error> for InesParseError {
    fn from(e: std::io::Error) -> Self {
        InesParseError::Io(e)
    }
}

#[derive(Debug)]
pub enum NametableArrangement {
    Vertical,
    Horizontal,
}

#[derive(Debug)]
pub struct INESHeader {
    /// Size of PRG ROM in 16 KB units
    pub prg_rom_size: usize,
    /// Size of CHR ROM in 8 KB units (value 0 means the board uses CHR RAM)
    pub chr_rom_size: usize,
    pub nametable_arrangement: NametableArrangement,
    pub has_battery: bool,
    pub has_trainer: bool,
    pub use_alternative_nametable_layout: bool,
    pub mapper_number: u8,
}

impl TryFrom<&[u8]> for INESHeader {
    type Error = InesParseError;

    fn try_from(header: &[u8]) -> Result<Self, Self::Error> {
        if header.len() < 16 {
            return Err(InesParseError::HeaderTooShort { size: header.len() });
        }
        if &header[..4] != b"NES\x1A" {
            return Err(InesParseError::InvalidHeader);
        }

        Ok(INESHeader {
            prg_rom_size: header[4] as usize,
            chr_rom_size: header[5] as usize,
            nametable_arrangement: if header[6] & 0x01 == 0 {
                NametableArrangement::Vertical
            } else {
                NametableArrangement::Horizontal
            },
            has_battery: header[6] & 0x02 > 0,
            has_trainer: header[6] & 0x04 > 0,
            use_alternative_nametable_layout: header[6] & 0x08 > 0,
            mapper_number: (header[7] & 0xF0) | ((header[6] & 0xF0) >> 4),
        })
    }
}

pub fn parse_file(filepath: &str) -> Result<INES, InesParseError> {
    let path = Path::new(filepath);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut header = vec![0u8; 16];
    reader.read_exact(&mut header)?;
    let parsed_header = INESHeader::try_from(header.as_slice())?;
    let trainer = if parsed_header.has_trainer {
        let mut buf = vec![0u8; 512];
        reader.read_exact(&mut buf)?;
        Some(buf)
    } else {
        None
    };

    let mut prg_rom = vec![0u8; 16384 * parsed_header.prg_rom_size];
    reader.read_exact(&mut prg_rom)?;

    let chr_rom = if parsed_header.chr_rom_size > 0 {
        let mut buf = vec![0u8; 8192 * parsed_header.chr_rom_size];
        reader.read_exact(&mut buf)?;
        Some(buf)
    } else {
        None
    };

    Ok(INES {
        header: parsed_header,
        trainer,
        prg_rom,
        chr_rom,
        playchoice_inst_rom: None,
        playchoice_prom: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header_with_wrong_magic_returns_invalid_header_error() {
        let header = vec![0u8; 16];
        let parsed_header = INESHeader::try_from(header.as_slice());
        assert!(matches!(
            parsed_header.unwrap_err(),
            InesParseError::InvalidHeader
        ));
    }

    #[test]
    fn parse_header_with_wrong_size_returns_header_too_short_error() {
        let header = vec![0u8; 10];
        let parsed_header = INESHeader::try_from(header.as_slice());
        assert!(matches!(
            parsed_header.unwrap_err(),
            InesParseError::HeaderTooShort { size: 10 }
        ));
    }

    #[test]
    fn parse_file_test() {
        let file = parse_file("data/nestest.nes");
    }
}
