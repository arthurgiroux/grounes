use std::usize;

pub trait MemoryBus {
    fn read_byte(&self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, value: u8);
}

pub struct RAM {
    memory: Vec<u8>,
}

impl RAM {
    pub fn new(size: usize) -> Self {
        RAM {
            memory: vec![0; size],
        }
    }
}

impl MemoryBus for RAM {
    fn read_byte(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    
    fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }
}
