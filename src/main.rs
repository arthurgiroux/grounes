pub mod cpu;
pub mod emulator;
pub mod ines;
pub mod memory;

use crate::emulator::Emulator;

fn main() {
    let mut emulator = Emulator::new();
    emulator.power_up();
}
