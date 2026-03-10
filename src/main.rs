pub mod cpu;
pub mod emulator;
pub mod memory;

use crate::emulator::Emulator;

fn main() {
    let emulator = Emulator::new();
}
