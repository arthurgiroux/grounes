use crate::memory::MemoryBus;
use bitflags::bitflags;
use std::fmt;
mod addressing_mode;
mod instruction;
mod opcode;

pub use addressing_mode::AddressingMode;
pub use instruction::Instruction;
pub use opcode::OpCode;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StatusRegister: u8 {
        /// Carry flag:
        ///    This flag is used in additions, subtractions,
        ///    comparisons and bit rotations. In additions and
        ///    subtractions, it acts as a 9th bit and lets you to chain
        ///    operations to calculate with bigger than 8-bit numbers.
        ///    When subtracting, the Carry flag is the negative of
        ///    Borrow: if an overflow occurs, the flag will be clear,
        ///    otherwise set. Comparisons are a special case of
        ///    subtraction: they assume Carry flag set and Decimal flag
        ///    clear, and do not store the result of the subtraction
        ///    anywhere.
        const Carry = 0b00000001;
        /// Zero flag:
        ///    The Zero flag will be affected in the same cases than
        ///    the Negative flag. Generally, it will be set if an
        ///    arithmetic register is being loaded with the value zero,
        ///    and cleared otherwise. The flag will behave differently
        ///    in Decimal operations.
        const Zero = 0b00000010;
        /// Interrupt disabled:
        ///    This flag can be used to prevent the processor from
        ///    jumping to the IRQ handler vector ($FFFE) whenever the
        ///    hardware line -IRQ is active. The flag will be
        ///    automatically set after taking an interrupt, so that the
        ///    processor would not keep jumping to the interrupt
        ///    routine if the -IRQ signal remains low for several clock
        ///    cycles.
        const InterruptDisabled = 0b00000100;
        /// Decimal flag:
        ///     On the NES the decimal mode is disabled so this flag has no effect.
        const Decimal = 0b00001000;
        /// Break flag:
        ///    This flag is used to distinguish software (BRK)
        ///    interrupts from hardware interrupts (IRQ or NMI). The B
        ///    flag is always set except when the P register is being
        ///    pushed on stack when jumping to an interrupt routine to
        ///    process only a hardware interrupt.
        const Break = 0b00010000;
        /// Unused flag:
        ///     To the current knowledge, this flag is always 1.
        const Unused = 0b00100000;
        /// Overflow flag:
        ///    After a binary addition or subtraction, the V flag will
        ///    be set on a sign overflow, cleared otherwise. What is a
        ///    sign overflow? For instance, if you are trying to add
        ///    123 and 45 together, the result (168) does not fit in a
        ///    8-bit signed integer (upper limit 127 and lower limit
        ///    -128). Similarly, adding -123 to -45 causes the
        ///    overflow, just like subtracting -45 from 123 or 123 from
        ///    -45 would do.
        const Overflow = 0b01000000;
        /// Negative flag:
        ///    This flag will be set after any arithmetic operations
        ///    (when any of the registers A, X or Y is being loaded
        ///    with a value). Generally, the N flag will be copied from
        ///    the topmost bit of the register being loaded.
        const Negative = 0b10000000;
    }
}

impl StatusRegister {
    fn update_zero_flag(&mut self, value: u8) {
        self.set(StatusRegister::Zero, value == 0);
    }

    fn update_negative_flag(&mut self, value: u8) {
        self.set(StatusRegister::Negative, (value & 0x80) > 0);
    }
}

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

/// A memory page is crossed after an increment operation when the high-byte is increased.
fn is_page_crossed(base_addr: u16, incremented_addr: u16) -> bool {
    (base_addr & 0xFF00) != (incremented_addr & 0xFF00)
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
    pub fn step<T: MemoryBus>(&mut self, memory: &mut T) -> (u8, u8) {
        // When changing the "disable interrupt" flag through some instruction,
        // The change is delayed to the next instruction.
        if let Some(value) = self.pending_interrupt_flag_change {
            self.p.set(StatusRegister::InterruptDisabled, value);
        }

        // Fetch the next instruction
        let value = self.fetch_byte(memory);

        // Decode it
        let opcode = self.decode(value).unwrap_or_else(|| {
            panic!("Unknown opcode {:02X}", value);
        });
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

        (value, cycles)
    }

    pub fn decode(&self, opcode: u8) -> Option<OpCode> {
        match opcode {
            // --- BEGIN SECTION ADC ---
            0x69 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x65 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x75 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x6D => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x7D => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0x79 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0x61 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x71 => Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION ADC ---
            // --- BEGIN SECTION SBC ---
            0xE9 | 0xEB => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xE5 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xF5 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0xED => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xFD => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0xF9 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0xE1 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0xF1 => Some(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION SBC ---
            // --- BEGIN SECTION INC ---
            0xE6 => Some(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0xF6 => Some(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0xEE => Some(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0xFE => Some(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0xE8 => Some(OpCode {
                instr: Instruction::INX,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xC8 => Some(OpCode {
                instr: Instruction::INY,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION INC ---
            // --- BEGIN SECTION ISB ---
            0xE7 => Some(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0xF7 => Some(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0xEF => Some(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0xFF => Some(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0xFB => Some(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0xE3 => Some(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0xF3 => Some(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION ISB ---
            // --- BEGIN SECTION DEC ---
            0xC6 => Some(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0xD6 => Some(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0xCE => Some(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0xDE => Some(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0xCA => Some(OpCode {
                instr: Instruction::DEX,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x88 => Some(OpCode {
                instr: Instruction::DEY,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION DEC ---
            // --- BEGIN SECTION DCP ---
            0xC7 => Some(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0xD7 => Some(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0xCF => Some(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0xDF => Some(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0xDB => Some(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0xC3 => Some(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0xD3 => Some(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION DCP ---
            // --- BEGIN SECTION ASL ---
            0x0A => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::Accumulator,
                value: opcode,
                base_cycle: 2,
            }),
            0x06 => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x16 => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x0E => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x1E => Some(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ASL ---
            // --- BEGIN SECTION SLO ---
            0x07 => Some(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x17 => Some(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x0F => Some(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x1F => Some(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0x1B => Some(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0x03 => Some(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0x13 => Some(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION SLO ---
            // --- BEGIN SECTION SRE ---
            0x47 => Some(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x57 => Some(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x4F => Some(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x5B => Some(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0x5F => Some(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0x43 => Some(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0x53 => Some(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION SRE ---
            // --- BEGIN SECTION RRA ---
            0x67 => Some(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x77 => Some(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x6F => Some(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x7B => Some(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0x7F => Some(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0x63 => Some(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0x73 => Some(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION RRA ---
            // --- BEGIN BRANCH INSTRUCTIONS ---
            0x90 => Some(OpCode {
                instr: Instruction::BCC,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0xB0 => Some(OpCode {
                instr: Instruction::BCS,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0xF0 => Some(OpCode {
                instr: Instruction::BEQ,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0xD0 => Some(OpCode {
                instr: Instruction::BNE,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0x10 => Some(OpCode {
                instr: Instruction::BPL,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0x30 => Some(OpCode {
                instr: Instruction::BMI,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0x50 => Some(OpCode {
                instr: Instruction::BVC,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0x70 => Some(OpCode {
                instr: Instruction::BVS,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION BRANCH ---
            // --- BEGIN SECTION LOAD ---
            0xA9 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xA5 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xB5 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0xAD => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xBD => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0xB9 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0xA1 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0xB1 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            0xA2 => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xA6 => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xB6 => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::ZeroPageY,
                value: opcode,
                base_cycle: 4,
            }),
            0xAE => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xBE => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0xA0 => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xA4 => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xB4 => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0xAC => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xBC => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0xA3 => Some(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0xA7 => Some(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xAF => Some(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xB3 => Some(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            0xB7 => Some(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::ZeroPageY,
                value: opcode,
                base_cycle: 4,
            }),
            0xBF => Some(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION LOAD ---
            // --- BEGIN SECTION STORE ---
            0x85 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x95 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x8D => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x9D => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 5,
            }),
            0x99 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 5,
            }),
            0x81 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x91 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 6,
            }),
            0x86 => Some(OpCode {
                instr: Instruction::STX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x96 => Some(OpCode {
                instr: Instruction::STX,
                mode: AddressingMode::ZeroPageY,
                value: opcode,
                base_cycle: 4,
            }),
            0x8E => Some(OpCode {
                instr: Instruction::STX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x84 => Some(OpCode {
                instr: Instruction::STY,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x94 => Some(OpCode {
                instr: Instruction::STY,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x8C => Some(OpCode {
                instr: Instruction::STY,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x83 => Some(OpCode {
                instr: Instruction::SAX,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x87 => Some(OpCode {
                instr: Instruction::SAX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x8F => Some(OpCode {
                instr: Instruction::SAX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x97 => Some(OpCode {
                instr: Instruction::SAX,
                mode: AddressingMode::ZeroPageY,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION STORE ---
            // --- BEGIN SECTION TRANSFER ---
            0xAA => Some(OpCode {
                instr: Instruction::TAX,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xA8 => Some(OpCode {
                instr: Instruction::TAY,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x8A => Some(OpCode {
                instr: Instruction::TXA,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x98 => Some(OpCode {
                instr: Instruction::TYA,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION TRANSFER ---
            // --- BEGIN SECTION LSR ---
            0x4A => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::Accumulator,
                value: opcode,
                base_cycle: 2,
            }),
            0x46 => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x56 => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x4E => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x5E => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION LSR ---
            // --- BEGIN SECTION ROL ---
            0x2A => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::Accumulator,
                value: opcode,
                base_cycle: 2,
            }),
            0x26 => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x36 => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x2E => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x3E => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ROL ---
            // --- BEGIN SECTION RLA ---
            0x23 => Some(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0x33 => Some(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            0x2F => Some(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x27 => Some(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x37 => Some(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x3B => Some(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0x3F => Some(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION RLA ---
            // --- BEGIN SECTION ROR ---
            0x6A => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::Accumulator,
                value: opcode,
                base_cycle: 2,
            }),
            0x66 => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x76 => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x6E => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x7E => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ROR ---
            // --- BEGIN SECTION AND ---
            0x29 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x25 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x35 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x2D => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x3D => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0x39 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0x21 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x31 => Some(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION AND ---
            // --- BEGIN SECTION ORA ---
            0x09 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x05 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x15 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x0D => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x1D => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0x19 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0x01 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x11 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION ORA ---
            // --- BEGIN SECTION EOR ---
            0x49 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x45 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x55 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x4D => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x5D => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0x59 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0x41 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x51 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION EOR ---
            // --- BEGIN SECTION BIT ---
            0x24 => Some(OpCode {
                instr: Instruction::BIT,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x2C => Some(OpCode {
                instr: Instruction::BIT,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION BIT ---
            // --- BEGIN SECTION CMP ---
            0xC9 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xC5 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xD5 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0xCD => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xDD => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0xD9 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0xC1 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0xD1 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION CMP ---
            // --- BEGIN SECTION CPX ---
            0xE0 => Some(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xE4 => Some(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xEC => Some(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION CPX ---
            // --- BEGIN SECTION CPY ---
            0xC0 => Some(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xC4 => Some(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xCC => Some(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION CPY ---
            // --- BEGIN SECTION JMP ---
            0x4C => Some(OpCode {
                instr: Instruction::JMP,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 3,
            }),
            0x6C => Some(OpCode {
                instr: Instruction::JMP,
                mode: AddressingMode::Indirect,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION JMP ---
            // --- BEGIN SECTION JSR ---
            0x20 => Some(OpCode {
                instr: Instruction::JSR,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION JSR ---
            // --- BEGIN SECTION JSR ---
            0x60 => Some(OpCode {
                instr: Instruction::RTS,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION JSR ---
            // --- BEGIN SECTION BRK ---
            0x00 => Some(OpCode {
                instr: Instruction::BRK,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION BRK ---
            // --- BEGIN SECTION RTI ---
            0x40 => Some(OpCode {
                instr: Instruction::RTI,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION RTI ---
            // --- BEGIN SECTION PH/PL ---
            0x48 => Some(OpCode {
                instr: Instruction::PHA,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 3,
            }),
            0x08 => Some(OpCode {
                instr: Instruction::PHP,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 3,
            }),
            0x68 => Some(OpCode {
                instr: Instruction::PLA,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 4,
            }),
            0x28 => Some(OpCode {
                instr: Instruction::PLP,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION PH/PL ---
            // --- END SECTION TXS/TSX ---
            0xBA => Some(OpCode {
                instr: Instruction::TSX,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x9A => Some(OpCode {
                instr: Instruction::TXS,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION TXS/TSX ---
            // --- BEGIN SECTION SET/CLEAR FLAGS ---
            0x18 => Some(OpCode {
                instr: Instruction::CLC,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xD8 => Some(OpCode {
                instr: Instruction::CLD,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x58 => Some(OpCode {
                instr: Instruction::CLI,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xB8 => Some(OpCode {
                instr: Instruction::CLV,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x38 => Some(OpCode {
                instr: Instruction::SEC,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xF8 => Some(OpCode {
                instr: Instruction::SED,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x78 => Some(OpCode {
                instr: Instruction::SEI,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION SET/CLEAR FLAGS ---
            // --- BEGIN SECTION MISC ---
            0x04 | 0x44 | 0x64 => Some(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => Some(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x0C => Some(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xEA | 0xFA => Some(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x80 => Some(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => Some(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION MISC ---
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
                (addr, is_page_crossed(arg, addr))
            }
            // Fetches the value from a 16-bit address with the offset in Y.
            AddressingMode::AbsoluteY => {
                let arg = self.fetch_word(memory);
                let addr = arg.wrapping_add(self.y as u16);
                (addr, is_page_crossed(arg, addr))
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
            let page_crossed = is_page_crossed(prev_pc, self.pc);
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
    fn decode_adc_should_give_correct_opcode() {
        let cpu = CPU::default();
        assert_eq!(
            cpu.decode(0x69),
            Some(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Immediate,
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
