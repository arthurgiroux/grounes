pub mod cpu;
pub mod emulator;
pub mod ines;
pub mod memory;

use std::io;

use crate::emulator::Emulator;

fn main() {
    let mut emulator = Emulator::new();
    if let Err(err) = emulator.load_rom("data/nestest.nes") {
        println!("Couldn't open ROM, error={}", err);
        return;
    }
    emulator.power_up();
    let mut input = String::new();

    loop {
        emulator.step();
        let _ = io::stdin().read_line(&mut input);
    }
}
