use crate::cpu::CPU;
use crate::memory::RAM;

pub struct Emulator {
    cpu: CPU,
    ram: RAM,
}

impl Emulator {
    pub fn new() -> Emulator {
        Emulator {
            cpu: CPU::default(),
            ram: RAM::new(2048),
        }
    }
}
