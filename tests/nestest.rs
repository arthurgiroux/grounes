use grounes::emulator::Emulator;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[test]
fn nestest_comparison() {
    let file_path = Path::new("data/nestest.log");
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);

    let mut emulator = Emulator::new();
    emulator.load_rom("data/nestest.nes").unwrap();
    emulator.power_up();
    emulator.set_pc(0xC000);
    let mut elapsed_cycles = 7;

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
                u16::from_str_radix(bytes.split_ascii_whitespace().next().unwrap(), 16).unwrap();
            let _disasm = &caps[3]; // "JMP $C5F5"
            let a = u8::from_str_radix(&caps[4], 16).unwrap(); // "00"
            let x = u8::from_str_radix(&caps[5], 16).unwrap(); // "00"
            let y = u8::from_str_radix(&caps[6], 16).unwrap(); // "00"
            let p = u8::from_str_radix(&caps[7], 16).unwrap(); // "24"
            let sp = u8::from_str_radix(&caps[8], 16).unwrap(); // "FD"
            let _ppu_dot = &caps[9]; // "0"
            let _ppu_cyc = &caps[10]; // "21"
            let cyc = u32::from_str_radix(&caps[11], 10).unwrap(); // "7"

            assert_eq!(
                pc, emulator.cpu.pc,
                "PC mismatch on line {}, expected:{:2X} Got:{:2X}\n\t{}",
                line_number, pc, emulator.cpu.pc, emulator.cpu,
            );

            assert_eq!(
                a, emulator.cpu.a,
                "Register 'a' mismatch on line {}, expected:{:2X} Got:{:2X}\n\t{}",
                line_number, a, emulator.cpu.a, emulator.cpu,
            );

            assert_eq!(
                x, emulator.cpu.x,
                "Register 'x' mismatch on line {}, expected:{:2X} Got:{:2X}\n\t{}",
                line_number, x, emulator.cpu.x, emulator.cpu,
            );

            assert_eq!(
                y, emulator.cpu.y,
                "Register 'y' mismatch on line {}, expected:{:2X} Got:{:2X}\n\t{}",
                line_number, y, emulator.cpu.y, emulator.cpu,
            );

            assert_eq!(
                p,
                emulator.cpu.p.bits(),
                "Register 'p' mismatch on line {}, expected:{:2X} Got:{:2X}\n\t{}",
                line_number,
                p,
                emulator.cpu.p.bits(),
                emulator.cpu,
            );

            assert_eq!(
                sp, emulator.cpu.sp.value,
                "Stack pointer mismatch on line {}, expected:{:2X} Got:{:2X}\n\t{}",
                line_number, sp, emulator.cpu.sp.value, emulator.cpu,
            );

            assert_eq!(
                cyc, elapsed_cycles,
                "Cycle mismatch on line {}, expected:{:} Got:{:}\n\t{}",
                line_number, cyc, elapsed_cycles, emulator.cpu,
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
            elapsed_cycles += cycles as u32;
            line_number += 1;
        }
        println!("{}", line);
    }
}
