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

pub enum Instruction {
    ADC,
    AND,
}

pub enum AddressingMode {
    Impl,
    Acc,
    Imm,
    Zp,
    ZpX,
    ZpY,
    Abs,
    AbsX,
    AbsY,
    Ind,
    IndX,
    IndY,
}

pub struct OpCode {
    instr: Instruction,
    mode: AddressingMode,
    value: u8,
}

trait MemoryBus {
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, value: u8);
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            register: CPURegister::default(),
        }
    }

    pub fn step<B: MemoryBus>(&mut self, memory: &mut B) {
        let opcode = memory.read_byte(self.register.pc);
    }

    pub fn decode(&self, opcode: u8) -> Option<OpCode> {
        match (opcode) {
            0x69 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Imm,
                value: opcode,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_adc_should_give_correct_opcode() {
        let cpu = CPU::new();
        let decode = cpu.decode(0x69);
        assert!(matches!(
            decode,
            Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Imm,
                value: 0x69
            })
        ));
    }
}
