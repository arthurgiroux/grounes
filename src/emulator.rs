use crate::cpu::CPU;
use crate::ines::parse_file;
use crate::mapper::Mapper;
use crate::mapper::create_mapper;
use crate::memory::{BusView, RAM};

pub struct Emulator {
    pub cpu: CPU,
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

    pub fn set_pc(&mut self, value: u16) {
        self.cpu.pc = value;
    }
}
