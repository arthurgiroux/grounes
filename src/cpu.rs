#[derive(Default)]
struct CPURegister {
    // accumulator
    a: u8,

    // Indexes, used for several addressing modes
    x: u8,
    y: u8,

    // Program counter
    pc: u16,

    // stack pointer
    sp: u8,

    // status register;
    p: u8,
}

pub struct CPU {
    register: CPURegister,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            register: CPURegister::default(),
        }
    }
}
