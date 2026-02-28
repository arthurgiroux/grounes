pub struct CPU {
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
    // 7  bit  0
    // ---- ----
    // NV1B DIZC
    // |||| ||||
    // |||| |||+- Carry
    // |||| ||+-- Zero
    // |||| |+--- Interrupt Disable
    // |||| +---- Decimal
    // |||+------ (No CPU effect; see: the B flag)
    // ||+------- (No CPU effect; always pushed as 1)
    // |+-------- Overflow
    // +--------- Negative
    p: u8,
}

pub enum Instruction {
    ADC,
    AND,
}

pub enum AddressingMode {
    Imp,
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
    fn read_byte(&self, addr: u16) -> u8;
    fn read_word(&self, addr: u16) -> u16;
    fn write_byte(&mut self, addr: u16, value: u8);
    fn write_word(&mut self, addr: u16, value: u16);
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0,
            p: 0,
        }
    }

    pub fn step<T: MemoryBus>(&mut self, memory: &mut T) {
        let value = memory.read_byte(self.pc);
        self.pc += 1;
        match self.decode(value) {
            Some(opcode) => match opcode.instr {
                Instruction::ADC => self.instr_adc(memory, opcode.mode),
                _ => panic!(),
            },
            None => panic!(),
        }
    }

    pub fn decode(&self, opcode: u8) -> Option<OpCode> {
        match opcode {
            0x69 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Imm,
                value: opcode,
            }),
            _ => None,
        }
    }

    fn instr_adc<T: MemoryBus>(&mut self, memory: &mut T, addressing_mode: AddressingMode) {
        let value = match addressing_mode {
            AddressingMode::Imm => {
                let arg = memory.read_byte(self.pc);
                self.pc += 1;
                arg
            }
            AddressingMode::Zp => {
                let arg = memory.read_byte(self.pc);
                self.pc += 1;
                memory.read_byte((arg & 0x00FF).into())
            }
            AddressingMode::ZpX => {
                let arg = memory.read_byte(self.pc);
                self.pc += 1;
                memory.read_byte(((arg + self.x) & 0x00FF).into())
            }
            AddressingMode::Abs => {
                let arg = memory.read_word(self.pc);
                self.pc += 2;
                memory.read_byte(arg)
            }
            AddressingMode::AbsX => {
                let arg = memory.read_word(self.pc);
                self.pc += 2;
                memory.read_byte(arg + (self.x as u16))
            }
            AddressingMode::AbsY => {
                let arg = memory.read_word(self.pc);
                self.pc += 2;
                memory.read_byte(arg + (self.y as u16))
            },
            AddressingMode::IndX => {
                let arg = memory.read_byte(self.pc);
                self.pc += 1;
                let addr = ((arg + self.x) & 0xFF) as u16;
                memory.read_byte(memory.read_word(addr))
            },
            AddressingMode::IndY => {
                let arg = memory.read_byte(self.pc);
                self.pc += 1;
                let low = memory.read_byte(arg as u16);
                let high = memory.read_byte(((arg as u16) + 1) & 0xFF);
                let base_addr = (high as u16) << 8 | low as u16;
                memory.read_byte(base_addr)
            }
            _ => panic!(),
        };

        self.a += value + self.get_carry()

        // TODO set flags
    }

    fn get_carry(&self) -> u8 {
        self.p & 0x01
    }

    fn set_carry(&mut self, value: bool) {}
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
