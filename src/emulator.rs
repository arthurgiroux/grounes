use crate::cpu::CPU;
use crate::memory::{BusView, RAM};

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

    pub fn power_up(&mut self) {
        let mut view = BusView { ram: &mut self.ram };
        self.cpu.power_up(&mut view);
    }

    pub fn step(&mut self) -> u8 {
        let mut view = BusView { ram: &mut self.ram };
        self.cpu.step(&mut view)
    }
}
