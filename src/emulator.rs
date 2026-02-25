use crate::cpu::CPU;

pub struct Emulator {
    cpu: CPU,
}

impl Emulator {
    pub fn new() -> Emulator {
        Emulator { cpu: CPU::new() }
    }
}
