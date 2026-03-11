use std::io::{Error, ErrorKind, Result};

struct INES {}

#[derive(Debug)]
pub struct INESHeader {
    prg_rom_size: u8,
    chr_rom_size: u8,
    flag_6: u8,
    flag_7: u8,
    flag_8: u8,
    flag_9: u8,
    flag_10: u8,
}

pub fn parse_header(header: &[u8]) -> Result<INESHeader> {
    if header.len() < 16 {
        return Err(Error::new(ErrorKind::InvalidData, "iNES header too short"));
    }
    if &header[..4] != b"NES\x1A" {
        return Err(Error::new(ErrorKind::InvalidData, "invalid iNES magic"));
    }

    Ok(INESHeader {
        prg_rom_size: header[4],
        chr_rom_size: header[5],
        flag_6: header[6],
        flag_7: header[7],
        flag_8: header[8],
        flag_9: header[9],
        flag_10: header[10],
    })
}
