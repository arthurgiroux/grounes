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

#[derive(Debug)]
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

/// A memory page is crossed after an increment operation when the high-byte is increased.
fn is_page_crossed(base_addr: u16, incremented_addr: u16) -> bool {
    (base_addr & 0xFF00) != (incremented_addr & 0xFF00)
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

    fn fetch_byte<T: MemoryBus>(&mut self, memory: &T) -> u8 {
        let value = memory.read_byte(self.pc);
        self.pc += 1;
        value
    }

    fn fetch_word<T: MemoryBus>(&mut self, memory: &T) -> u16 {
        let low = memory.read_byte(self.pc);
        let high = memory.read_byte(self.pc + 1);
        self.pc += 2;
        ((high as u16) << 8) | low as u16
    }

    fn get_operand_address<T: MemoryBus>(
        &mut self,
        memory: &T,
        addressing_mode: AddressingMode,
    ) -> (u16, bool) {
        match addressing_mode {
            // Fetch a value from the zero-page (0x00FF)
            AddressingMode::Zp => {
                let arg = self.fetch_byte(memory);
                (arg as u16, false)
            }
            // Fetches the value from an 8-bit address with the offset in X on the zero page.
            AddressingMode::ZpX => {
                let arg = self.fetch_byte(memory);
                (arg.wrapping_add(self.x) as u16, false)
            }
            // Fetches the value from an 8-bit address with the offset in Y on the zero page.
            AddressingMode::ZpY => {
                let arg = self.fetch_byte(memory);
                (arg.wrapping_add(self.y) as u16, false)
            }
            // Fetches the value from a 16-bit address anywhere in memory.
            AddressingMode::Abs => (self.fetch_word(memory), false),
            // Fetches the value from a 16-bit address with the offset in X.
            AddressingMode::AbsX => {
                let arg = self.fetch_word(memory);
                let addr = arg.wrapping_add(self.x as u16);
                (addr, is_page_crossed(arg, addr))
            }
            // Fetches the value from a 16-bit address with the offset in Y.
            AddressingMode::AbsY => {
                let arg = self.fetch_word(memory);
                let addr = arg.wrapping_add(self.y as u16);
                (addr, is_page_crossed(arg, addr))
            }
            AddressingMode::Ind => {
                let arg = self.fetch_word(memory);
                let low = memory.read_byte(arg);
                // 6502 indirect jump bug:
                // when the address is at the end of page, the CPU fails to increment the page when reading the second byte.
                // Instead, it will wraps to the beginning of the page, reading the wrong address.
                // For eample JMP ($03FF) reads $03FF and $0300 instead of $0400
                // We need to replicate this behavior to ensure correctness.
                let high_addr = if (arg & 0x00FF) == 0x00FF {
                    arg & 0xFF00
                } else {
                    arg + 1
                };
                let high = memory.read_byte(high_addr);
                let addr = ((high as u16) << 8) | low as u16;
                (addr, false)
            }
            AddressingMode::IndX => {
                let arg = self.fetch_byte(memory);

                let ptr = arg.wrapping_add(self.x);
                let low = memory.read_byte(ptr as u16);
                let high = memory.read_byte(ptr.wrapping_add(1) as u16);
                let addr = (high as u16) << 8 | low as u16;
                (addr, false)
            }
            AddressingMode::IndY => {
                let arg = self.fetch_byte(memory);
                let low = memory.read_byte(arg as u16);
                let high = memory.read_byte(arg.wrapping_add(1) as u16);
                let base_addr = (high as u16) << 8 | low as u16;
                let addr = base_addr.wrapping_add(self.y as u16);
                (addr, is_page_crossed(base_addr, addr))
            }
            _ => panic!("addressing mode {addressing_mode:?} is not operating on memory."),
        }
    }

    fn instr_adc<T: MemoryBus>(&mut self, memory: &mut T, addressing_mode: AddressingMode) {
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
