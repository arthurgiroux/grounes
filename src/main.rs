pub mod cpu;
pub mod emulator;

use crate::emulator::Emulator;

fn main() {
    let emulator = Emulator::new();
}
