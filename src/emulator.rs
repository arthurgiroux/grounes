use crate::cpu::CPU;
use crate::ines::parse_file;
use crate::memory::{BusView, Mapper, RAM, create_mapper};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub struct Emulator {
    cpu: CPU,
    ram: RAM,
    mapper: Option<Box<dyn Mapper>>,
}

impl Emulator {
    pub fn new() -> Emulator {
        Emulator {
            cpu: CPU::default(),
            ram: RAM::new(2048),
            mapper: None,
        }
    }

    pub fn load_rom(&mut self, filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
        let ines = parse_file(filepath)?;
        self.mapper = Some(create_mapper(ines)?);
        Ok(())
    }

    pub fn power_up(&mut self) {
        let mapper = self.mapper.as_mut().expect("no ROM loaded");
        let mut view = BusView {
            ram: &mut self.ram,
            mapper: mapper.as_mut(),
        };
        self.cpu.power_up(&mut view);
    }

    pub fn step(&mut self) -> (u8, u8) {
        let mapper = self.mapper.as_mut().expect("no ROM loaded");
        let mut view = BusView {
            ram: &mut self.ram,
            mapper: mapper.as_mut(),
        };
        self.cpu.step(&mut view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn nestest_comparison() {
        let file_path = Path::new("data/nestest.log");
        let file = File::open(file_path).unwrap();
        let reader = BufReader::new(file);

        let mut emulator = Emulator::new();
        emulator.load_rom("data/nestest.nes").unwrap();
        emulator.power_up();

        let mut line_number = 0;
        // We are going to compare line by line the result of the CPU
        for line_result in reader.lines() {
            let line = line_result.unwrap();

            let re = Regex::new(
    r"^([0-9A-F]{4})\s+([0-9A-F]{2}(?:\s[0-9A-F]{2}){0,2})\s+(.+?)\s+A:([0-9A-F]{2})\s+X:([0-9A-F]{2})\s+Y:([0-9A-F]{2})\s+P:([0-9A-F]{2})\s+SP:([0-9A-F]{2})\s+PPU:\s*(\d+),\s*(\d+)\s+CYC:(\d+)$"
).unwrap();

            if let Some(caps) = re.captures(&line) {
                let pc = u16::from_str_radix(&caps[1], 16).unwrap(); // "C000"
                let bytes = &caps[2]; // "4C F5 C5"
                let ref_opcode =
                    u16::from_str_radix(bytes.split_ascii_whitespace().next().unwrap(), 16)
                        .unwrap();
                let disasm = &caps[3]; // "JMP $C5F5"
                let a = u16::from_str_radix(&caps[4], 16).unwrap(); // "00"
                let x = u16::from_str_radix(&caps[5], 16).unwrap(); // "00"
                let y = u16::from_str_radix(&caps[6], 16).unwrap(); // "00"
                let p = u16::from_str_radix(&caps[7], 16).unwrap(); // "24"
                let sp = &caps[8]; // "FD"
                let ppu_dot = &caps[9]; // "0"
                let ppu_cyc = &caps[10]; // "21"
                let cyc = &caps[11]; // "7"

                assert_eq!(
                    pc, emulator.cpu.pc,
                    "PC mismatch on line {}, got={:X} but expected {:X}",
                    line_number, emulator.cpu.pc, pc
                );
                assert_eq!(
                    a,
                    emulator.cpu.a.into(),
                    "register 'a' mismatch on line {} for opcode={:X}, got={:X} but expected {:X}",
                    line_number,
                    ref_opcode,
                    emulator.cpu.a,
                    a
                );
                let (opcode, cycles) = emulator.step();
                assert_eq!(
                    ref_opcode,
                    opcode.into(),
                    "opcode mismatch on line {}, got={:X} but expected {:X}",
                    line_number,
                    opcode,
                    ref_opcode
                );
                line_number += 1;
            }
            println!("{}", line);
        }
    }
}
