mod addressing_mode;
mod instruction;
mod opcode;
mod status_register;

pub use addressing_mode::AddressingMode;
pub use instruction::Instruction;
pub use opcode::OpCode;
pub use status_register::StatusRegister;

use crate::memory::{MemoryBus, is_memory_page_crossed};
use std::fmt;

#[derive(Debug)]
pub struct CPU {
    /// Accumulator:
    ///     The accumulator is the main register for arithmetic and logic
    ///     operations. Unlike the index registers X and Y, it has a direct
    ///     connection to the Arithmetic and Logic Unit (ALU). This is why
    ///     many operations are only available for the accumulator, not the
    ///     index registers.
    pub a: u8,

    /// Index Register X:
    ///    This is the main register for addressing data with indices. It has
    ///    a special addressing mode, indexed indirect, which lets you to
    ///    have a vector table on the zero page.
    pub x: u8,

    /// Index Register Y:
    ///    The Y register has the least operations available. On the other
    ///    hand, only it has the indirect indexed addressing mode that
    ///    enables access to any memory place without having to use
    ///    self-modifying code.
    pub y: u8,

    /// Program counter:
    ///    This register points the address from which the next instruction
    ///    byte (opcode or parameter) will be fetched. Unlike other
    ///    registers, this one is 16 bits in length. The low and high 8-bit
    ///    halves of the register are called PCL and PCH, respectively. The
    ///    Program Counter may be read by pushing its value on the stack.
    ///    This can be done either by jumping to a subroutine or by causing
    ///    an interrupt.
    pub pc: u16,

    /// Stack Pointer:
    ///    The CPU have 256 bytes of stack memory, ranging
    ///    from $0100 to $01FF. The SP register is a 8-bit offset to the stack
    ///    page. In other words, whenever anything is being pushed on the
    ///    stack, it will be stored to the address $0100+S.
    pub sp: StackPointer,

    /// Processor Status:
    ///    This 8-bit register stores the state of the processor. The bits in
    ///    this register are called flags. Most of the flags have something
    ///    to do with arithmetic operations.
    pub p: StatusRegister,

    /// Some changes to the InterruptDisabled flag are delayed to the next cycle,
    /// We keep track of any pending change that needs to be performed.
    pending_interrupt_flag_change: Option<bool>,
}

impl fmt::Display for CPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PC:{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
            self.pc,
            self.a,
            self.x,
            self.y,
            self.p.bits(),
            self.sp.value
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents the operand that will be used for some instructions
enum Operand {
    /// Use the value of the accumulator
    Accumulator,
    /// The value that comes directly after the opcode
    Immediate(u8),
    /// Use a value from memory at a given address
    Memory(u16, bool),
    /// A signed offset used for branching
    Relative(i8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    X,
    Y,
    A,
    SP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOperation {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitwiseOperation {
    And,
    Or,
    Xor,
}

#[derive(Debug)]
pub struct StackPointer {
    pub value: u8,
}

/// An "Empty Descending" stack pointer.
/// The stack pointer points to the last valid data item pushed onto the stack.
impl StackPointer {
    pub fn push_byte<T: MemoryBus>(&mut self, memory: &mut T, value: u8) {
        memory.write_byte(0x0100 | self.value as u16, value);
        self.value -= 1;
    }

    pub fn pop_byte<T: MemoryBus>(&mut self, memory: &T) -> u8 {
        self.value += 1;
        let value = memory.read_byte(0x0100 | self.value as u16);
        value
    }
}

pub struct StepResult {
    pub opcode: Option<OpCode>,
    pub cycles: u8,
}

impl Default for StackPointer {
    fn default() -> Self {
        StackPointer { value: 0xFF }
    }
}

impl Default for CPU {
    fn default() -> Self {
        Self::new()
    }
}

impl CPU {
    pub fn new() -> CPU {
        // Reference value: https://www.nesdev.org/wiki/CPU_power_up_state
        CPU {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: StackPointer { value: 0xFD },
            p: StatusRegister::InterruptDisabled | StatusRegister::Unused,
            pending_interrupt_flag_change: None,
        }
    }

    pub fn power_up<T: MemoryBus>(&mut self, memory: &T) {
        // Reference value: https://www.nesdev.org/wiki/CPU_power_up_state
        self.pc = u16::from_le_bytes([memory.read_byte(0xFFFC), memory.read_byte(0xFFFD)]);
    }

    /// Step the CPU: fetch the next instruction and execute it
    /// returns the number of cycles it took
    pub fn step<T: MemoryBus>(&mut self, memory: &mut T) -> StepResult {
        // When changing the "disable interrupt" flag through some instruction,
        // The change is delayed to the next instruction.
        if let Some(value) = self.pending_interrupt_flag_change {
            self.p.set(StatusRegister::InterruptDisabled, value);
        }

        // Fetch the next instruction
        let value = self.fetch_byte(memory);

        // Decode it
        let decode = OpCode::try_from(value);
        if decode.is_err() {
            eprintln!("Invalid opcode {:02X}, skipping it.", value);
            return StepResult {
                opcode: None,
                cycles: 0,
            };
        }
        let opcode = decode.unwrap();

        let operand = self.resolve_operand(memory, opcode.mode);

        let extra_cycles = match opcode.instr {
            Instruction::ADC => self.instr_adc(memory, operand.unwrap()),
            Instruction::SBC => self.instr_sbc(memory, operand.unwrap()),
            Instruction::INC => self.instr_inc(memory, operand.unwrap()),
            Instruction::ISB => {
                self.instr_inc(memory, operand.unwrap());
                self.instr_sbc(memory, operand.unwrap());
                None
            }
            Instruction::DEC => self.instr_dec(memory, operand.unwrap()),
            Instruction::DCP => {
                self.instr_dec(memory, operand.unwrap());
                self.instr_compare(memory, operand.unwrap(), Register::A);
                None
            }
            Instruction::INX => {
                self.generic_register_arithmetic(Register::X, ArithmeticOperation::Increment)
            }
            Instruction::DEX => {
                self.generic_register_arithmetic(Register::X, ArithmeticOperation::Decrement)
            }
            Instruction::INY => {
                self.generic_register_arithmetic(Register::Y, ArithmeticOperation::Increment)
            }
            Instruction::DEY => {
                self.generic_register_arithmetic(Register::Y, ArithmeticOperation::Decrement)
            }
            Instruction::AND => self.instr_bitwise(memory, operand.unwrap(), BitwiseOperation::And),
            Instruction::ORA => self.instr_bitwise(memory, operand.unwrap(), BitwiseOperation::Or),
            Instruction::EOR => self.instr_bitwise(memory, operand.unwrap(), BitwiseOperation::Xor),
            Instruction::SLO => {
                self.instr_asl(memory, operand.unwrap());
                self.instr_bitwise(memory, operand.unwrap(), BitwiseOperation::Or);
                None
            }
            Instruction::SRE => {
                self.instr_lsr(memory, operand.unwrap());
                self.instr_bitwise(memory, operand.unwrap(), BitwiseOperation::Xor);
                None
            }
            Instruction::RRA => {
                self.instr_ror(memory, operand.unwrap());
                self.instr_adc(memory, operand.unwrap());
                None
            }
            Instruction::BIT => self.instr_bit(memory, operand.unwrap()),
            Instruction::ASL => self.instr_asl(memory, operand.unwrap()),
            Instruction::LSR => self.instr_lsr(memory, operand.unwrap()),
            Instruction::ROL => self.instr_rol(memory, operand.unwrap()),
            Instruction::ROR => self.instr_ror(memory, operand.unwrap()),
            Instruction::RLA => {
                self.instr_rol(memory, operand.unwrap());
                self.instr_bitwise(memory, operand.unwrap(), BitwiseOperation::And);
                None
            }
            Instruction::BCC => {
                self.generic_instr_branch(operand.unwrap(), !self.p.contains(StatusRegister::Carry))
            }
            Instruction::BCS => {
                self.generic_instr_branch(operand.unwrap(), self.p.contains(StatusRegister::Carry))
            }
            Instruction::BEQ => {
                self.generic_instr_branch(operand.unwrap(), self.p.contains(StatusRegister::Zero))
            }
            Instruction::BNE => {
                self.generic_instr_branch(operand.unwrap(), !self.p.contains(StatusRegister::Zero))
            }
            Instruction::BPL => self
                .generic_instr_branch(operand.unwrap(), !self.p.contains(StatusRegister::Negative)),
            Instruction::BMI => self
                .generic_instr_branch(operand.unwrap(), self.p.contains(StatusRegister::Negative)),
            Instruction::BVC => self
                .generic_instr_branch(operand.unwrap(), !self.p.contains(StatusRegister::Overflow)),
            Instruction::BVS => self
                .generic_instr_branch(operand.unwrap(), self.p.contains(StatusRegister::Overflow)),
            Instruction::LDA => self.instr_load(memory, vec![Register::A], operand.unwrap()),
            Instruction::LDX => self.instr_load(memory, vec![Register::X], operand.unwrap()),
            Instruction::LDY => self.instr_load(memory, vec![Register::Y], operand.unwrap()),
            Instruction::LAX => {
                self.instr_load(memory, vec![Register::A, Register::X], operand.unwrap())
            }
            Instruction::STA => self.instr_store(memory, Register::A, operand.unwrap()),
            Instruction::STX => self.instr_store(memory, Register::X, operand.unwrap()),
            Instruction::STY => self.instr_store(memory, Register::Y, operand.unwrap()),
            Instruction::SAX => {
                self.instr_store_and(memory, Register::A, Register::X, operand.unwrap())
            }
            Instruction::TAX => self.instr_transfer(Register::A, Register::X),
            Instruction::TAY => self.instr_transfer(Register::A, Register::Y),
            Instruction::TXA => self.instr_transfer(Register::X, Register::A),
            Instruction::TYA => self.instr_transfer(Register::Y, Register::A),
            Instruction::CMP => self.instr_compare(memory, operand.unwrap(), Register::A),
            Instruction::CPX => self.instr_compare(memory, operand.unwrap(), Register::X),
            Instruction::CPY => self.instr_compare(memory, operand.unwrap(), Register::Y),
            Instruction::JMP => self.instr_jump(operand.unwrap()),
            Instruction::JSR => self.instr_jump_to_subroutine(memory, operand.unwrap()),
            Instruction::RTS => self.instr_return_from_subroutine(memory),
            Instruction::BRK => self.instr_break(memory),
            Instruction::RTI => self.instr_return_from_interrupt(memory),
            Instruction::PHA => self.instr_push_register_to_sp(memory, Register::A),
            Instruction::PLA => self.instr_pull_register_from_sp(memory, Register::A),
            Instruction::PHP => self.instr_push_flags_to_sp(memory),
            Instruction::PLP => self.instr_pull_flags_from_sp(memory),
            Instruction::TSX => self.instr_transfer(Register::SP, Register::X),
            Instruction::TXS => self.instr_transfer(Register::X, Register::SP),
            Instruction::CLC => self.instr_clear_flag(StatusRegister::Carry),
            Instruction::SEC => self.instr_set_flag(StatusRegister::Carry),
            Instruction::CLI => {
                self.pending_interrupt_flag_change = Some(false);
                None
            }
            Instruction::SEI => {
                self.pending_interrupt_flag_change = Some(true);
                None
            }
            Instruction::CLD => self.instr_clear_flag(StatusRegister::Decimal),
            Instruction::SED => self.instr_set_flag(StatusRegister::Decimal),
            Instruction::CLV => self.instr_clear_flag(StatusRegister::Overflow),
            Instruction::NOP => matches!(&operand, Some(Operand::Memory(_, true))).then_some(1),
        };

        let cycles = opcode.base_cycle + extra_cycles.unwrap_or_default();

        StepResult {
            opcode: Some(opcode),
            cycles,
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
        u16::from_le_bytes([low, high])
    }

    fn resolve_operand<T: MemoryBus>(
        &mut self,
        memory: &T,
        mode: AddressingMode,
    ) -> Option<Operand> {
        match mode {
            AddressingMode::Immediate => Some(Operand::Immediate(self.fetch_byte(memory))),
            AddressingMode::Accumulator => Some(Operand::Accumulator),
            AddressingMode::Relative => Some(Operand::Relative(self.fetch_byte(memory) as i8)),
            AddressingMode::Implicit => None,
            _ => {
                let (addr, page_crossed) = self.get_operand_address(memory, mode);
                Some(Operand::Memory(addr, page_crossed))
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
            AddressingMode::ZeroPage => {
                let arg = self.fetch_byte(memory);
                (arg as u16, false)
            }
            // Fetches the value from an 8-bit address with the offset in X on the zero page.
            AddressingMode::ZeroPageX => {
                let arg = self.fetch_byte(memory);
                (arg.wrapping_add(self.x) as u16, false)
            }
            // Fetches the value from an 8-bit address with the offset in Y on the zero page.
            AddressingMode::ZeroPageY => {
                let arg = self.fetch_byte(memory);
                (arg.wrapping_add(self.y) as u16, false)
            }
            // Fetches the value from a 16-bit address anywhere in memory.
            AddressingMode::Absolute => (self.fetch_word(memory), false),
            // Fetches the value from a 16-bit address with the offset in X.
            AddressingMode::AbsoluteX => {
                let arg = self.fetch_word(memory);
                let addr = arg.wrapping_add(self.x as u16);
                (addr, is_memory_page_crossed(arg, addr))
            }
            // Fetches the value from a 16-bit address with the offset in Y.
            AddressingMode::AbsoluteY => {
                let arg = self.fetch_word(memory);
                let addr = arg.wrapping_add(self.y as u16);
                (addr, is_memory_page_crossed(arg, addr))
            }
            AddressingMode::Indirect => {
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
                let addr = u16::from_le_bytes([low, high]);
                (addr, false)
            }
            AddressingMode::IndirectX => {
                let arg = self.fetch_byte(memory);
                let ptr = arg.wrapping_add(self.x);
                let low = memory.read_byte(ptr as u16);
                let high = memory.read_byte(ptr.wrapping_add(1) as u16);
                let addr = u16::from_le_bytes([low, high]);
                (addr, false)
            }
            AddressingMode::IndirectY => {
                let arg = self.fetch_byte(memory);
                let low = memory.read_byte(arg as u16);
                let high = memory.read_byte(arg.wrapping_add(1) as u16);
                let base_addr = u16::from_le_bytes([low, high]);
                let addr = base_addr.wrapping_add(self.y as u16);
                (addr, is_memory_page_crossed(base_addr, addr))
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

        let result: u16 =
            self.a as u16 + value as u16 + self.p.contains(StatusRegister::Carry) as u16;
        let prev_value = self.a;
        self.a = result as u8;
        self.p.set(StatusRegister::Carry, result > 0xFF);
        self.p.update_zero_flag(self.a);
        // If the result's sign is different from both A's and memory's, signed overflow (or underflow) occurred.
        self.p.set(
            StatusRegister::Overflow,
            ((self.a ^ prev_value) & (self.a ^ value) & 0x80) != 0,
        );
        self.p.update_negative_flag(self.a);

        // If we crossed a memory page, we need to add an extra cycle
        matches!(&operand, Operand::Memory(_, true)).then_some(1)
    }

    /// SBC instruction: Substracts the NOT of the carry flag and an operand from the accumulator.
    fn instr_sbc<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let result =
            self.a as u16 + (!value) as u16 + self.p.contains(StatusRegister::Carry) as u16;
        let prev_value = self.a;
        self.a = result as u8;
        self.p.set(StatusRegister::Carry, result >= 0x100);
        self.p.update_zero_flag(self.a);
        // If the result's sign is different from both A's and memory's, signed overflow (or underflow) occurred.
        self.p.set(
            StatusRegister::Overflow,
            (self.a ^ prev_value) & (self.a ^ !value) & 0x80 != 0,
        );
        self.p.update_negative_flag(self.a);

        // If we crossed a memory page, we need do add an extra cycle
        matches!(&operand, Operand::Memory(_, true)).then_some(1)
    }

    /// INC instruction: Adds 1 to a memory location.
    fn instr_inc<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        match operand {
            Operand::Memory(addr, _) => {
                let value = memory.read_byte(addr).wrapping_add(1);
                memory.write_byte(addr, value);
                self.p.update_zero_flag(value);
                self.p.update_negative_flag(value);
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
                self.p.update_zero_flag(value);
                self.p.update_negative_flag(value);
            }
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        }
        None
    }

    fn instr_bitwise<T: MemoryBus>(
        &mut self,
        memory: &mut T,
        operand: Operand,
        operation: BitwiseOperation,
    ) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        self.a = match operation {
            BitwiseOperation::And => self.a & value,
            BitwiseOperation::Or => self.a | value,
            BitwiseOperation::Xor => self.a ^ value,
        };

        self.p.update_zero_flag(self.a);
        self.p.update_negative_flag(self.a);

        // If we crossed a memory page, we need do add an extra cycle
        matches!(operand, Operand::Memory(_, true)).then_some(1)
    }

    fn instr_bit<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let bit_test = self.a & value;

        self.p.update_zero_flag(bit_test);
        self.p
            .set(StatusRegister::Overflow, (value & 0b01000000) > 0);
        self.p
            .set(StatusRegister::Negative, (value & 0b10000000) > 0);

        None
    }

    fn instr_compare<T: MemoryBus>(
        &mut self,
        memory: &T,
        operand: Operand,
        register: Register,
    ) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let reg = self.get_register(register);
        self.p.set(StatusRegister::Carry, reg >= value);
        self.p.set(StatusRegister::Zero, reg == value);
        self.p.set(
            StatusRegister::Negative,
            (reg.wrapping_sub(value)) & 0x80 != 0,
        );

        matches!(operand, Operand::Memory(_, true)).then_some(1)
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
        self.p.set(StatusRegister::Carry, value & 0x80 != 0);
        self.p.update_zero_flag(shifted_value);
        self.p.update_negative_flag(shifted_value);

        match operand {
            Operand::Accumulator => {
                self.a = shifted_value;
            }
            Operand::Memory(addr, _) => memory.write_byte(addr, shifted_value),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        None
    }

    /// LSR instruction: shifts all the bits of a memory value or the accumulator one position to the right
    /// lowest bit will be put in the carry
    fn instr_lsr<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let shifted_value = value >> 1;
        self.p.set(StatusRegister::Carry, value & 0x01 > 0);
        self.p.update_zero_flag(shifted_value);
        self.p.set(StatusRegister::Negative, false);

        match operand {
            Operand::Accumulator => {
                self.a = shifted_value;
            }
            Operand::Memory(addr, _) => memory.write_byte(addr, shifted_value),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        None
    }

    /// ROL instruction: Shifts a memory value of the accumulator to the left through the carry.
    fn instr_rol<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };
        let shifted_value = (value << 1) | (self.p.contains(StatusRegister::Carry) as u8);
        self.p.set(StatusRegister::Carry, value & 0x80 != 0);
        self.p.update_zero_flag(shifted_value);
        self.p.update_negative_flag(shifted_value);

        match operand {
            Operand::Accumulator => {
                self.a = shifted_value;
            }
            Operand::Memory(addr, _) => memory.write_byte(addr, shifted_value),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        None
    }

    /// ROR instruction: Shifts a memory value of the accumulator to the right through the carry.
    fn instr_ror<T: MemoryBus>(&mut self, memory: &mut T, operand: Operand) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let shifted_value = (value >> 1) | ((self.p.contains(StatusRegister::Carry) as u8) << 7);
        self.p.set(StatusRegister::Carry, value & 0x01 != 0);
        self.p.update_zero_flag(shifted_value);
        self.p.update_negative_flag(shifted_value);

        match operand {
            Operand::Accumulator => {
                self.a = shifted_value;
            }
            Operand::Memory(addr, _) => memory.write_byte(addr, shifted_value),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        None
    }

    fn get_register_mut(&mut self, register: Register) -> &mut u8 {
        match register {
            Register::X => &mut self.x,
            Register::Y => &mut self.y,
            Register::A => &mut self.a,
            Register::SP => &mut self.sp.value,
        }
    }

    fn get_register(&self, register: Register) -> u8 {
        match register {
            Register::X => self.x,
            Register::Y => self.y,
            Register::A => self.a,
            Register::SP => self.sp.value,
        }
    }

    fn instr_store<T: MemoryBus>(
        &mut self,
        memory: &mut T,
        register: Register,
        operand: Operand,
    ) -> Option<u8> {
        match operand {
            Operand::Memory(addr, _) => {
                let value = self.get_register(register);
                memory.write_byte(addr, value);
            }
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        }
        None
    }

    fn instr_store_and<T: MemoryBus>(
        &mut self,
        memory: &mut T,
        register1: Register,
        register2: Register,
        operand: Operand,
    ) -> Option<u8> {
        match operand {
            Operand::Memory(addr, _) => {
                let value1 = self.get_register(register1);
                let value2 = self.get_register(register2);
                memory.write_byte(addr, value1 & value2);
            }
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        }
        None
    }

    fn instr_load<T: MemoryBus>(
        &mut self,
        memory: &T,
        registers: Vec<Register>,
        operand: Operand,
    ) -> Option<u8> {
        let value = match operand {
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        for register in registers {
            let reg = self.get_register_mut(register);
            *reg = value;
        }

        self.p.update_negative_flag(value);
        self.p.update_zero_flag(value);

        matches!(operand, Operand::Memory(_, true)).then_some(1)
    }

    fn instr_transfer(&mut self, source: Register, target: Register) -> Option<u8> {
        let source = self.get_register(source);
        let target_reg = self.get_register_mut(target);
        *target_reg = source;
        // We only update the flags for some register
        if matches!(target, Register::A | Register::X | Register::Y) {
            self.p.update_negative_flag(source);
            self.p.update_zero_flag(source);
        }
        None
    }

    fn generic_register_arithmetic(
        &mut self,
        register: Register,
        operation: ArithmeticOperation,
    ) -> Option<u8> {
        let reg = self.get_register_mut(register);
        let value = match operation {
            ArithmeticOperation::Increment => (*reg).wrapping_add(1),
            ArithmeticOperation::Decrement => (*reg).wrapping_sub(1),
        };

        *reg = value;

        self.p.update_negative_flag(value);
        self.p.update_zero_flag(value);
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
            let page_crossed = is_memory_page_crossed(prev_pc, self.pc);
            Some(if page_crossed { 2 } else { 1 })
        } else {
            None
        }
    }

    fn instr_jump(&mut self, operand: Operand) -> Option<u8> {
        self.pc = match operand {
            Operand::Memory(addr, _) => addr,
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        None
    }

    fn instr_jump_to_subroutine<T: MemoryBus>(
        &mut self,
        memory: &mut T,
        operand: Operand,
    ) -> Option<u8> {
        // JSR pushes the pc that points right before the next instruction
        let [low, high] = self.pc.wrapping_sub(1).to_le_bytes();
        self.sp.push_byte(memory, high);
        self.sp.push_byte(memory, low);

        self.pc = match operand {
            Operand::Memory(addr, _) => addr,
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };
        None
    }

    fn instr_return_from_subroutine<T: MemoryBus>(&mut self, memory: &mut T) -> Option<u8> {
        let low = self.sp.pop_byte(memory);
        let high = self.sp.pop_byte(memory);

        // Since JSR pushed the address before the next instruction, we need to increment by one
        self.pc = u16::from_le_bytes([low, high]).wrapping_add(1);
        None
    }

    fn instr_break<T: MemoryBus>(&mut self, memory: &mut T) -> Option<u8> {
        let pc_value = self.pc.wrapping_add(1);
        // When we get an IRQ we push the current PC and processor flags to the stack.
        let [low, high] = pc_value.to_le_bytes();
        self.sp.push_byte(memory, high);
        self.sp.push_byte(memory, low);

        // The break flag must be set on the flags that are pushed to the stack, not the flags in the CPU
        let mut current_flag = self.p.clone();
        current_flag.set(StatusRegister::Break, true);
        self.sp.push_byte(memory, current_flag.bits());

        self.pc = u16::from_le_bytes([memory.read_byte(0xFFFE), memory.read_byte(0xFFFF)]);
        self.p.set(StatusRegister::InterruptDisabled, true);

        None
    }

    fn instr_return_from_interrupt<T: MemoryBus>(&mut self, memory: &mut T) -> Option<u8> {
        let flags = self.sp.pop_byte(memory);
        let pc_low = self.sp.pop_byte(memory);
        let pc_high = self.sp.pop_byte(memory);
        self.pc = u16::from_le_bytes([pc_low, pc_high]);

        self.p = StatusRegister::from_bits_truncate(flags).union(StatusRegister::Unused);
        self.p.remove(StatusRegister::Break);

        None
    }

    fn instr_push_register_to_sp<T: MemoryBus>(
        &mut self,
        memory: &mut T,
        register: Register,
    ) -> Option<u8> {
        self.sp.push_byte(memory, self.get_register(register));

        None
    }

    fn instr_pull_register_from_sp<T: MemoryBus>(
        &mut self,
        memory: &T,
        register: Register,
    ) -> Option<u8> {
        let value = self.sp.pop_byte(memory);
        let reg = self.get_register_mut(register);
        *reg = value;

        self.p.update_negative_flag(value);
        self.p.update_zero_flag(value);

        None
    }

    fn instr_push_flags_to_sp<T: MemoryBus>(&mut self, memory: &mut T) -> Option<u8> {
        let flags = self.p.union(StatusRegister::Break);
        self.sp.push_byte(memory, flags.bits());

        None
    }

    fn instr_pull_flags_from_sp<T: MemoryBus>(&mut self, memory: &T) -> Option<u8> {
        let value = self.sp.pop_byte(memory);

        self.p = StatusRegister::from_bits_truncate(value).union(StatusRegister::Unused);
        self.p.remove(StatusRegister::Break);

        None
    }

    fn instr_clear_flag(&mut self, flag: StatusRegister) -> Option<u8> {
        self.p.remove(flag);
        None
    }

    fn instr_set_flag(&mut self, flag: StatusRegister) -> Option<u8> {
        self.p.insert(flag);
        None
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

        fn write_byte(&mut self, addr: u16, value: u8) {
            self.memory[addr as usize] = value;
        }
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
        memory.write_byte(addr, 0x56);
        memory.write_byte(addr + 1, 0x34);
        assert_eq!(cpu.fetch_word(&memory), value);
        assert_eq!(cpu.pc, addr + 2);
    }

    #[test]
    fn get_operand_address_zero_page_returns_address_from_zero_page() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        memory.write_byte(0x00, 0x12);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZeroPage);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZeroPageX);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZeroPageX);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZeroPageY);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::ZeroPageY);
        assert_eq!(addr, 0x03);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_abs_returns_absolute_16bits_addr() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let value = 0x1234;
        memory.write_byte(0x00, 0x34);
        memory.write_byte(0x01, 0x12);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Absolute);
        assert_eq!(addr, value);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_abs_x_returns_absolute_16bits_addr_with_x_offset() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        cpu.x = offset;
        memory.write_byte(0x00, 0x34);
        memory.write_byte(0x01, 0x12);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsoluteX);
        assert_eq!(addr, 0x1239);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_abs_x_should_wrap_addr() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        cpu.x = offset;
        memory.write_byte(0x00, 0xFE);
        memory.write_byte(0x01, 0xFF);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsoluteX);
        assert_eq!(addr, 0x0003);
        assert_eq!(page_crossed, true);
    }

    #[test]
    fn get_operand_abs_x_should_allow_crossing_pages() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        cpu.x = offset;
        memory.write_byte(0x00, 0xFE);
        memory.write_byte(0x01, 0x01);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsoluteX);
        assert_eq!(addr, 0x0203);
        assert_eq!(page_crossed, true);
    }

    #[test]
    fn get_operand_abs_y_returns_absolute_16bits_addr_with_y_offset() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        cpu.y = offset;
        memory.write_byte(0x00, 0x34);
        memory.write_byte(0x01, 0x12);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsoluteY);
        assert_eq!(addr, 0x1239);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn get_operand_abs_y_should_wrap_addr() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        cpu.y = offset;
        memory.write_byte(0x00, 0xFE);
        memory.write_byte(0x01, 0xFF);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsoluteY);
        assert_eq!(addr, 0x0003);
        assert_eq!(page_crossed, true);
    }

    #[test]
    fn get_operand_abs_y_should_allow_crossing_pages() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let offset = 5;
        cpu.y = offset;
        memory.write_byte(0x00, 0xFE);
        memory.write_byte(0x01, 0x01);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsoluteY);
        assert_eq!(addr, 0x0203);
        assert_eq!(page_crossed, true);
    }

    #[test]
    fn get_operand_ind_returns_indirected_addr() {
        let mut memory = MockMemory::new();
        let mut cpu = CPU::new();
        let expected_addr = 0x3456;
        memory.write_byte(0x00, 0x34);
        memory.write_byte(0x01, 0x12);
        memory.write_byte(0x1234, 0x56);
        memory.write_byte(0x1235, 0x34);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Indirect);
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
        memory.write_byte(0x00, 0xFF);
        memory.write_byte(0x01, 0x12);

        // This should be the indirected address without the CPU bug
        memory.write_byte(0x12FF, 0x56);
        memory.write_byte(0x1300, 0x34);

        // We are going to put a value at the beginning of the page to ensure the CPU bug is implemented
        let zp_addr = 0x1200;
        let zp_value = 0x89;

        // We expect the high-byte to be from the beginning of the page and the low byte to be from the end of the page
        let expected_addr = 0x8956;
        memory.write_byte(zp_addr, zp_value);

        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Indirect);
        assert_eq!(addr, expected_addr);
        assert_eq!(page_crossed, false);
    }

    #[test]
    fn stack_pointer_should_push_and_pop_byte() {
        let mut memory = MockMemory::new();
        let mut sp = StackPointer::default();
        let first_value = 0xAB;
        let second_value = 0xCD;
        sp.push_byte(&mut memory, first_value);
        sp.push_byte(&mut memory, second_value);
        assert_eq!(sp.pop_byte(&memory), second_value);
        assert_eq!(sp.pop_byte(&memory), first_value);
    }
}
