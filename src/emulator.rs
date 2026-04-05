use crate::cpu::{CPU, StepResult};
use crate::ines::parse_file;
use crate::mapper::Mapper;
use crate::mapper::create_mapper;
use crate::memory::{BusView, RAM};
use crate::ppu::PPU;

pub struct Emulator {
    pub cpu: CPU,
    ram: RAM,
    ppu: PPU,
    mapper: Option<Box<dyn Mapper>>,
    pub current_frame: Option<Vec<u8>>,
}

impl Emulator {
    pub fn new() -> Emulator {
        Emulator {
            cpu: CPU::default(),
            ram: RAM::new(2048),
            ppu: PPU::default(),
            mapper: None,
            current_frame: None,
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
            ppu: &mut self.ppu,
        };
        self.cpu.power_up(&mut view);
    }

    pub fn step(&mut self) -> StepResult {
        let mapper = self.mapper.as_mut().expect("no ROM loaded");
        let mut view = BusView {
            ram: &mut self.ram,
            mapper: mapper.as_mut(),
            ppu: &mut self.ppu,
        };
        let result = self.cpu.step(&mut view);
        let ppu_cycles = result.cycles * 3;
        for _ in 0..ppu_cycles {
            self.ppu.step();
            if self.ppu.is_new_frame_ready() {
                self.current_frame = Some(self.ppu.frame.clone());
            }
        }

        result
    }

    pub fn step_frame(&mut self) {
        // Clear the current frame then step until the PPU produces a new one.
        self.current_frame = None;
        for _ in 0..50_000 {
            self.step();
            if self.current_frame.is_some() {
                break;
            }
        }
    }

    pub fn set_pc(&mut self, value: u16) {
        self.cpu.pc = value;
    }

    pub fn get_bus_view(&mut self) -> BusView<'_> {
        let mapper = self.mapper.as_mut().expect("no ROM loaded");
        BusView {
            ram: &mut self.ram,
            mapper: mapper.as_mut(),
            ppu: &mut self.ppu,
        }
    }

    pub fn chr_rom(&self) -> Option<&[u8]> {
        self.mapper.as_deref()?.chr_rom()
    }
}
