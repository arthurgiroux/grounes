use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct StatusRegister: u8 {
        /// Carry flag: set after some operations if it carried over
        const C = 0b00000001;
        /// Zero flag: set if the result of the last operation is zero
        const Z = 0b00000010;
        /// Interrupt disabled: set if interrupts are disabled
        const I = 0b00000100;
        /// Decimal flag: On NES decimal mode is disabled so this flag has no effect
        const D = 0b00001000;
        /// Break flag: Set only when flags are pushed to the stack: 1 for BRK, 0 for IRQ/NMI.
        /// The CPU does not maintain B in the live status register.
        const B = 0b00010000;
        /// Unused flag: always pushed to 1
        const Unused = 0b00100000;
        /// Overflow flag: set after some operations if it overflows
        const V = 0b01000000;
        /// Negative flag: Set after some operations when the highest bit is set
        const N = 0b10000000;
    }
}

#[derive(Debug)]
pub struct CPU {
    /// accumulator
    a: u8,

    // Indexes, used for several addressing modes
    x: u8,
    y: u8,

    // Program counter
    pc: u16,

    // stack pointer
    sp: u8,

    // status register
    p: StatusRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    AND,
    ASL,
    // Branch operation
    BCC,
    BCS,
    BEQ,
    BNE,
    BPL,
    BMI,
    BVC,
    BVS,
    // Arithmetic
    ADC,
    SBC,
    INC,
    DEC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Rel,
    IndX,
    IndY,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operand {
    Accumulator,
    Immediate(u8),
    Memory(u16, bool),
    Relative(i8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpCode {
    /// The instruction that will be executed from this opcode
    instr: Instruction,
    /// The addressing mode, this will determine how to fetch the operand
    mode: AddressingMode,
    /// The value of the opcode
    value: u8,
    /// The usual number of cycles that the CPU takes to execute this opcode, additional cycles can be added depending on the addressing mode
    base_cycle: u8,
}

pub trait MemoryBus {
    fn read_byte(&self, addr: u16) -> u8;
    fn read_word(&self, addr: u16) -> u16;
    fn write_byte(&mut self, addr: u16, value: u8);
    fn write_word(&mut self, addr: u16, value: u16);
}

fn word_from_bytes(low: u8, high: u8) -> u16 {
    (high as u16) << 8 | low as u16
}

/// A memory page is crossed after an increment operation when the high-byte is increased.
fn is_page_crossed(base_addr: u16, incremented_addr: u16) -> bool {
    (base_addr & 0xFF00) != (incremented_addr & 0xFF00)
}

impl Default for CPU {
    fn default() -> Self {
        Self::new()
    }
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0,
            p: StatusRegister::empty(),
        }
    }

    /// Step the CPU: fetch the next instruction and execute it
    /// returns the number of cycles it took
    pub fn step<T: MemoryBus>(&mut self, memory: &mut T) -> u8 {
        // Fetch the next instruction
        let value = self.fetch_byte(memory);

        // Decode it
        let opcode = self.decode(value).expect("Unknown opcode");
        let operand = self.resolve_operand(memory, opcode.mode);

        let extra_cycles = match opcode.instr {
            Instruction::ADC => self.instr_adc(memory, operand),
            Instruction::SBC => self.instr_sbc(memory, operand),
            Instruction::INC => self.instr_inc(memory, operand),
            Instruction::DEC => self.instr_dec(memory, operand),
            Instruction::AND => self.instr_and(memory, operand),
            Instruction::ASL => self.instr_asl(memory, operand),
            Instruction::BCC => {
                self.generic_instr_branch(operand, !self.p.contains(StatusRegister::C))
            }
            Instruction::BCS => {
                self.generic_instr_branch(operand, self.p.contains(StatusRegister::C))
            }
            Instruction::BEQ => {
                self.generic_instr_branch(operand, self.p.contains(StatusRegister::Z))
            }
            Instruction::BNE => {
                self.generic_instr_branch(operand, !self.p.contains(StatusRegister::Z))
            }
            Instruction::BPL => {
                self.generic_instr_branch(operand, !self.p.contains(StatusRegister::N))
            }
            Instruction::BMI => {
                self.generic_instr_branch(operand, self.p.contains(StatusRegister::N))
            }
            Instruction::BVC => {
                self.generic_instr_branch(operand, !self.p.contains(StatusRegister::V))
            }
            Instruction::BVS => {
                self.generic_instr_branch(operand, self.p.contains(StatusRegister::V))
            }
        };

        opcode.base_cycle + extra_cycles.unwrap_or_default()
    }

    pub fn decode(&self, opcode: u8) -> Option<OpCode> {
        match opcode {
            // --- BEGIN SECTION ADC ---
            0x69 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0x65 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0x75 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0x6D => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0x7D => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 4,
            }),
            0x79 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 4,
            }),
            0x61 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::IndX,
                value: opcode,
                base_cycle: 6,
            }),
            0x71 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::IndY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION ADC ---
            // --- BEGIN SECTION SBC ---
            0xE9 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0xE5 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0xF5 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0xED => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0xFD => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 4,
            }),
            0xF9 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 4,
            }),
            0xE1 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::IndX,
                value: opcode,
                base_cycle: 6,
            }),
            0xF1 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::IndY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION SBC ---
            // --- BEGIN SECTION INC ---
            0xE6 => Some(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 5,
            }),
            0xF6 => Some(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 6,
            }),
            0xEE => Some(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 6,
            }),
            0xFE => Some(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION INC ---
            // --- BEGIN SECTION DEC ---
            0xC6 => Some(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 5,
            }),
            0xD6 => Some(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 6,
            }),
            0xCE => Some(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 6,
            }),
            0xDE => Some(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION DEC ---
            // --- BEGIN SECTION AND ---
            0x29 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0x25 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0x35 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0x2D => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0x3D => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 4,
            }),
            0x39 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 4,
            }),
            0x21 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::IndX,
                value: opcode,
                base_cycle: 6,
            }),
            0x31 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::IndY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION AND ---
            // --- BEGIN SECTION ASL ---
            0x0A => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::Acc,
                value: opcode,
                base_cycle: 2,
            }),
            0x06 => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 5,
            }),
            0x16 => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 6,
            }),
            0x0E => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 6,
            }),
            0x1E => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ASL ---
            // --- BEGIN BRANCH INSTRUCTIONS ---
            0x90 => Some(OpCode {
                instr: Instruction::BCC,
                mode: AddressingMode::Rel,
                value: opcode,
                base_cycle: 2,
            }),
            0xB0 => Some(OpCode {
                instr: Instruction::BCS,
                mode: AddressingMode::Rel,
                value: opcode,
                base_cycle: 2,
            }),
            0xF0 => Some(OpCode {
                instr: Instruction::BEQ,
                mode: AddressingMode::Rel,
                value: opcode,
                base_cycle: 2,
            }),
            0xD0 => Some(OpCode {
                instr: Instruction::BNE,
                mode: AddressingMode::Rel,
                value: opcode,
                base_cycle: 2,
            }),
            0x10 => Some(OpCode {
                instr: Instruction::BPL,
                mode: AddressingMode::Rel,
                value: opcode,
                base_cycle: 2,
            }),
            0x30 => Some(OpCode {
                instr: Instruction::BMI,
                mode: AddressingMode::Rel,
                value: opcode,
                base_cycle: 2,
            }),
            0x50 => Some(OpCode {
                instr: Instruction::BVC,
                mode: AddressingMode::Rel,
                value: opcode,
                base_cycle: 2,
            }),
            0x70 => Some(OpCode {
                instr: Instruction::BVS,
                mode: AddressingMode::Rel,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION BRANCH ---
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
        word_from_bytes(low, high)
    }

    fn resolve_operand<T: MemoryBus>(&mut self, memory: &T, mode: AddressingMode) -> Operand {
        match mode {
            AddressingMode::Imm => Operand::Immediate(self.fetch_byte(memory)),
            AddressingMode::Acc => Operand::Accumulator,
            AddressingMode::Rel => Operand::Relative(self.fetch_byte(memory) as i8),
            _ => {
                let (addr, page_crossed) = self.get_operand_address(memory, mode);
                Operand::Memory(addr, page_crossed)
            }
        }
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
                // For example JMP ($03FF) reads $03FF and $0300 instead of $0400
                // We need to replicate this behavior to ensure correctness.
                let high_addr = if (arg & 0x00FF) == 0x00FF {
                    arg & 0xFF00
                } else {
                    arg + 1
                };
                let high = memory.read_byte(high_addr);
                let addr = word_from_bytes(low, high);
                (addr, false)
            }
            AddressingMode::IndX => {
                let arg = self.fetch_byte(memory);
                let ptr = arg.wrapping_add(self.x);
                let low = memory.read_byte(ptr as u16);
                let high = memory.read_byte(ptr.wrapping_add(1) as u16);
                let addr = word_from_bytes(low, high);
                (addr, false)
            }
            AddressingMode::IndY => {
                let arg = self.fetch_byte(memory);
                let low = memory.read_byte(arg as u16);
                let high = memory.read_byte(arg.wrapping_add(1) as u16);
                let base_addr = word_from_bytes(low, high);
                let addr = base_addr.wrapping_add(self.y as u16);
                (addr, is_page_crossed(base_addr, addr))
            }
            _ => panic!("addressing mode {addressing_mode:?} is not operating on memory."),
        }
    }

    /// ADC instruction: Adds the carry flag and an operand to the accumulator.
    fn instr_adc<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let result: u16 = self.a as u16 + value as u16 + self.p.contains(StatusRegister::C) as u16;
        let prev_value = self.a;
        self.a = result as u8;
        self.p.set(StatusRegister::C, result > 0xFF);
        self.p.set(StatusRegister::Z, self.a == 0);
        // If the result's sign is different from both A's and memory's, signed overflow (or underflow) occurred.
        self.p.set(
            StatusRegister::V,
            ((self.a ^ prev_value) & (self.a ^ value) & 0x80) != 0,
        );
        self.p.set(StatusRegister::N, (self.a & 0x80) != 0);

        // If we crossed a memory page, we need do add an extra cycle
        matches!(&operand, Operand::Memory(_, true)).then_some(1)
    }

    /// SBC instruction: Subtract the NOT of the carry flag and an operand from the accumulator.
    fn instr_sbc<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let result: u16 = self.a as u16 - value as u16 - !self.p.contains(StatusRegister::C) as u16;
        let prev_value = self.a;
        self.a = result as u8;
        self.p.set(StatusRegister::C, result >= 0x100);
        self.p.set(StatusRegister::Z, self.a == 0);
        // If the result's sign is different from both A's and memory's, signed overflow (or underflow) occurred.
        self.p.set(
            StatusRegister::V,
            ((self.a ^ prev_value) & (self.a ^ !value) & 0x80) != 0,
        );
        self.p.set(StatusRegister::N, (self.a & 0x80) != 0);

        // If we crossed a memory page, we need do add an extra cycle
        matches!(&operand, Operand::Memory(_, true)).then_some(1)
    }

    /// INC instruction: Adds 1 to a memory location.
    fn instr_inc<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        match operand {
            Operand::Memory(addr, _) => {
                let value = memory.read_byte(addr).wrapping_add(1);
                memory.write_byte(addr, value);
                self.p.set(StatusRegister::Z, value == 0);
                self.p.set(StatusRegister::N, (value & 0x80) != 0);
            }
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        }
        None
    }

    /// DEC instruction: Substracts 1 from a memory location.
    fn instr_dec<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        match operand {
            Operand::Memory(addr, _) => {
                let value = memory.read_byte(addr).wrapping_sub(1);
                memory.write_byte(addr, value);
                self.p.set(StatusRegister::Z, value == 0);
                self.p.set(StatusRegister::N, (value & 0x80) != 0);
            }
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        }
        None
    }

    /// AND instruction: bitwise and operation between the accumulator and the operand
    fn instr_and<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        self.a &= value;
        self.p.set(StatusRegister::Z, self.a == 0);
        self.p.set(StatusRegister::N, self.a & 0x80 != 0);

        // If we crossed a memory page, we need do add an extra cycle
        matches!(&operand, Operand::Memory(_, true)).then_some(1)
    }

    /// ASL instruction: shifts all the bits of an operand one position to the left
    /// highest bit will be put in the carry
    fn instr_asl<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let shifted_value = value << 1;
        self.p.set(StatusRegister::C, value & 0x80 != 0);
        self.p.set(StatusRegister::Z, shifted_value == 0);
        self.p.set(StatusRegister::N, shifted_value & 0x80 != 0);

        match operand {
            Operand::Accumulator => {
                self.a = shifted_value;
            }
            Operand::Memory(addr, _) => memory.write_byte(addr, shifted_value),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        None
    }

    fn generic_instr_branch(&mut self, operand: Operand, should_branch: bool) -> Option<u8> {
        let value = match operand {
            Operand::Relative(val) => val,
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        if should_branch {
            let prev_pc = self.pc;
            self.pc = self.pc.wrapping_add_signed(value.into());
            let page_crossed = is_page_crossed(prev_pc, self.pc);
            Some(if page_crossed { 2 } else { 1 })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockMemory {
        memory: Vec<u8>,
    }

    impl MockMemory {
        fn new() -> Self {
            MockMemory {
                memory: vec![0; 0x10000],
            }
        }
    }

    impl MemoryBus for MockMemory {
        fn read_byte(&self, addr: u16) -> u8 {
            self.memory[addr as usize]
        }

        fn read_word(&self, addr: u16) -> u16 {
            word_from_bytes(self.memory[addr as usize], self.memory[(addr + 1) as usize])
        }

        fn write_byte(&mut self, addr: u16, value: u8) {
            self.memory[addr as usize] = value;
        }

        fn write_word(&mut self, addr: u16, value: u16) {
            self.memory[addr as usize] = value as u8;
            self.memory[(addr + 1) as usize] = (value >> 8) as u8;
        }
    }

    #[test]
    fn decode_adc_should_give_correct_opcode() {
        let cpu = CPU::default();
        assert_eq!(
            cpu.decode(0x69),
            Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Imm,
                value: 0x69,
                base_cycle: 2
            })
        );
    }

    #[test]
    fn mock_memory_can_write_and_read_bytes() {
        let mut memory = MockMemory::new();
        let addr = 0x12;
        let value = 0x34;
        memory.write_byte(addr, value);
        assert_eq!(memory.read_byte(addr), value);
    }

    #[test]
    fn mock_memory_can_write_and_read_words() {
        let mut memory = MockMemory::new();
        let addr = 0x12;
        let value = 0x3456;
        memory.write_word(addr, value);
        assert_eq!(memory.read_word(addr), value);
    }

    #[test]
    fn fetch_byte_reads_current_pc_and_increments_by_one() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let addr = 0x12;
        let value = 0x34;
        cpu.pc = addr;
        memory.write_byte(addr, value);
        assert_eq!(cpu.fetch_byte(&memory), value);
        assert_eq!(cpu.pc, addr + 1);
    }

    #[test]
    fn fetch_word_reads_current_pc_and_increments_by_two() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let addr = 0x12;
        let value = 0x3456;
        cpu.pc = addr;
        memory.write_word(addr, value);
        assert_eq!(cpu.fetch_word(&memory), value);
        assert_eq!(cpu.pc, addr + 2);
    }

    #[test]
    fn get_operand_address_zero_page_returns_address_from_zero_page() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        memory.write_byte(0x00, 0x12);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Zp);
        assert_eq!(addr, 0x0012);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_address_zero_page_x_returns_address_from_zero_page_plus_x() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        cpu.x = offset;
        memory.write_byte(0x00, 0x12);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZpX);
        assert_eq!(addr, 0x0012 + offset as u16);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_address_zero_page_x_should_wrap_address_in_zero_page() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        let address = 0xFE;
        cpu.x = offset;
        memory.write_byte(0x00, address);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZpX);
        assert_eq!(addr, 0x03);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_address_zero_page_y_returns_address_from_zero_page_plus_y() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        cpu.y = offset;
        memory.write_byte(0x00, 0x12);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZpY);
        assert_eq!(addr, 0x0012 + offset as u16);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_address_zero_page_y_should_wrap_address_in_zero_page() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        let address = 0xFE;
        cpu.y = offset;
        memory.write_byte(0x00, address);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZpY);
        assert_eq!(addr, 0x03);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_abs_returns_absolute_16bits_addr() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let value = 0x1234;
        memory.write_word(0x00, value);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Abs);
        assert_eq!(addr, value);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_abs_x_returns_absolute_16bits_addr_with_x_offset() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let value = 0x1234;
        let offset = 5;
        cpu.x = offset;
        memory.write_word(0x00, value);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsX);
        assert_eq!(addr, 0x1239);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_abs_x_should_wrap_addr() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let value = 0xFFFE;
        let offset = 5;
        cpu.x = offset;
        memory.write_word(0x00, value);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsX);
        assert_eq!(addr, 0x0003);
        assert_eq!(page_crossed, true);
    }

    #[test]
    fn get_operand_abs_x_should_allow_crossing_pages() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let value = 0x01FE;
        let offset = 5;
        cpu.x = offset;
        memory.write_word(0x00, value);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsX);
        assert_eq!(addr, 0x0203);
        assert_eq!(page_crossed, true);
    }

    #[test]
    fn get_operand_abs_y_returns_absolute_16bits_addr_with_y_offset() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let value = 0x1234;
        let offset = 5;
        cpu.y = offset;
        memory.write_word(0x00, value);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsY);
        assert_eq!(addr, 0x1239);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_abs_y_should_wrap_addr() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let value = 0xFFFE;
        let offset = 5;
        cpu.y = offset;
        memory.write_word(0x00, value);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsY);
        assert_eq!(addr, 0x0003);
        assert_eq!(page_crossed, true);
    }

    #[test]
    fn get_operand_abs_y_should_allow_crossing_pages() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let value = 0x01FE;
        let offset = 5;
        cpu.y = offset;
        memory.write_word(0x00, value);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsY);
        assert_eq!(addr, 0x0203);
        assert_eq!(page_crossed, true);
    }

    #[test]
    fn get_operand_ind_returns_indirected_addr() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let indirection = 0x1234;
        let expected_addr = 0x3456;
        memory.write_word(0x00, indirection);
        memory.write_word(indirection, expected_addr);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Ind);
        assert_eq!(addr, expected_addr);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_ind_cpu_bug_should_wrap_to_same_page() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        // We make sure that the CPU bug is correctly implemented:
        // When the indirect addr is the last one of a page, it will
        // load the second byte from the beginning of the same page instead of the next page.
        let indirection = 0x12FF;
        // This should be the indirected address without the CPU bug
        let ind_addr = 0x3456;

        // We are going to put a value at the beginning of the page to ensure the CPU bug is implemented
        let zp_addr = 0x1200;
        let zp_value = 0x89;

        // We expect the high-byte to be from the beginning of the page and the low byte to be from the end of the page
        let expected_addr = 0x8956;
        memory.write_byte(zp_addr, zp_value);
        memory.write_word(0x00, indirection);
        memory.write_word(indirection, ind_addr);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Ind);
        assert_eq!(addr, expected_addr);
        assert_eq!(page_crossed, false);
    }
}
