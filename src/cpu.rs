use crate::memory::MemoryBus;
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

impl StatusRegister {
    fn update_zero_flag(&mut self, value: u8) {
        self.set(StatusRegister::Z, value == 0);
    }

    fn update_negative_flag(&mut self, value: u8) {
        self.set(StatusRegister::N, (value & 0x80) > 0);
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
    sp: StackPointer,

    // status register
    p: StatusRegister,

    pending_interrupt_flag_change: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
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
    INX,
    DEX,
    INY,
    DEY,
    // Load
    LDA,
    LDX,
    LDY,
    // Store
    STA,
    STX,
    STY,
    // Transfer
    TAX,
    TAY,
    TXA,
    TYA,
    // Shift
    ASL,
    LSR,
    ROL,
    ROR,
    // Bitwise
    AND,
    ORA,
    EOR,
    BIT,
    // Compare
    CMP,
    CPX,
    CPY,
    // Jump
    JMP,
    JSR,
    RTS,
    BRK,
    RTI,
    // Stack
    PHA,
    PLA,
    PHP,
    PLP,
    TXS,
    TSX,
    // Flags
    CLC,
    SEC,
    CLI,
    SEI,
    CLD,
    SED,
    CLV,
    // MISC
    NOP,
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
pub enum Register {
    X,
    Y,
    A,
    SP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    Inc,
    Dec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitwiseOp {
    And,
    Or,
    Xor,
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

/// A memory page is crossed after an increment operation when the high-byte is increased.
fn is_page_crossed(base_addr: u16, incremented_addr: u16) -> bool {
    (base_addr & 0xFF00) != (incremented_addr & 0xFF00)
}

#[derive(Debug)]
struct StackPointer {
    value: u8,
}

/// An "Empty Descending" stack pointer.
/// The stack pointer points to the last valid data item pushed onto the stack.
impl StackPointer {
    fn push_byte<T: MemoryBus>(&mut self, memory: &mut T, value: u8) {
        memory.write_byte(0x0100 | self.value as u16, value);
        self.value -= 1;
    }

    fn pop_byte<T: MemoryBus>(&mut self, memory: &T) -> u8 {
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
            p: StatusRegister::I,
            pending_interrupt_flag_change: None,
        }
    }

    pub fn power_up<T: MemoryBus>(&mut self, memory: &T) {
        // Reference value: https://www.nesdev.org/wiki/CPU_power_up_state
        self.pc = u16::from_le_bytes([memory.read_byte(0xFFFC), memory.read_byte(0xFFFD)]);
    }

    /// Step the CPU: fetch the next instruction and execute it
    /// returns the number of cycles it took
    pub fn step<T: MemoryBus>(&mut self, memory: &mut T) -> u8 {
        // When changing the "disable interrupt" flag through some instruction,
        // The change is delayed to the next instruction.
        if let Some(value) = self.pending_interrupt_flag_change {
            self.p.set(StatusRegister::I, value);
        }
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
            Instruction::INX => self.generic_register_arithmetic(Register::X, ArithmeticOp::Inc),
            Instruction::DEX => self.generic_register_arithmetic(Register::X, ArithmeticOp::Dec),
            Instruction::INY => self.generic_register_arithmetic(Register::Y, ArithmeticOp::Inc),
            Instruction::DEY => self.generic_register_arithmetic(Register::Y, ArithmeticOp::Dec),
            Instruction::AND => self.instr_bitwise(memory, operand, BitwiseOp::And),
            Instruction::ORA => self.instr_bitwise(memory, operand, BitwiseOp::Or),
            Instruction::EOR => self.instr_bitwise(memory, operand, BitwiseOp::Xor),
            Instruction::BIT => self.instr_bit(memory, operand),
            Instruction::ASL => self.instr_asl(memory, operand),
            Instruction::LSR => self.instr_lsr(memory, operand),
            Instruction::ROL => self.instr_rol(memory, operand),
            Instruction::ROR => self.instr_ror(memory, operand),
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
            Instruction::LDA => self.instr_load(memory, Register::A, operand),
            Instruction::LDX => self.instr_load(memory, Register::X, operand),
            Instruction::LDY => self.instr_load(memory, Register::Y, operand),
            Instruction::STA => self.instr_store(memory, Register::A, operand),
            Instruction::STX => self.instr_store(memory, Register::X, operand),
            Instruction::STY => self.instr_store(memory, Register::Y, operand),
            Instruction::TAX => self.instr_transfer(Register::A, Register::X),
            Instruction::TAY => self.instr_transfer(Register::A, Register::Y),
            Instruction::TXA => self.instr_transfer(Register::X, Register::A),
            Instruction::TYA => self.instr_transfer(Register::Y, Register::A),
            Instruction::CMP => self.instr_compare(memory, operand, Register::A),
            Instruction::CPX => self.instr_compare(memory, operand, Register::X),
            Instruction::CPY => self.instr_compare(memory, operand, Register::Y),
            Instruction::JMP => self.instr_jump(operand),
            Instruction::JSR => self.instr_jump_to_subroutine(memory, operand),
            Instruction::RTS => self.instr_return_from_subroutine(memory),
            Instruction::BRK => self.instr_break(memory),
            Instruction::RTI => self.instr_return_from_interrupt(memory),
            Instruction::PHA => self.instr_push_register_to_sp(memory, Register::A),
            Instruction::PLA => self.instr_pull_register_from_sp(memory, Register::A),
            Instruction::PHP => self.instr_push_flags_to_sp(memory),
            Instruction::PLP => self.instr_pull_flags_from_sp(memory),
            Instruction::TSX => self.instr_transfer(Register::SP, Register::X),
            Instruction::TXS => self.instr_transfer(Register::X, Register::SP),
            Instruction::CLC => self.instr_clear_flag(StatusRegister::C),
            Instruction::SEC => self.instr_set_flag(StatusRegister::C),
            Instruction::CLI => {
                self.pending_interrupt_flag_change = Some(false);
                None
            }
            Instruction::SEI => {
                self.pending_interrupt_flag_change = Some(true);
                None
            }
            Instruction::CLD => self.instr_clear_flag(StatusRegister::D),
            Instruction::SED => self.instr_set_flag(StatusRegister::D),
            Instruction::CLV => self.instr_clear_flag(StatusRegister::V),
            Instruction::NOP => None,
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
            0xE8 => Some(OpCode {
                instr: Instruction::INX,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0xC8 => Some(OpCode {
                instr: Instruction::INY,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
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
            0xCA => Some(OpCode {
                instr: Instruction::DEX,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0x88 => Some(OpCode {
                instr: Instruction::DEY,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION DEC ---
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
            // --- BEGIN SECTION LOAD ---
            0xA9 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0xA5 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0xB5 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0xAD => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0xBD => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 4,
            }),
            0xB9 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 4,
            }),
            0xA1 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::IndX,
                value: opcode,
                base_cycle: 6,
            }),
            0xB1 => Some(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::IndY,
                value: opcode,
                base_cycle: 5,
            }),
            0xA2 => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0xA6 => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0xB6 => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::ZpY,
                value: opcode,
                base_cycle: 4,
            }),
            0xAE => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0xBE => Some(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 4,
            }),
            0xA0 => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0xA4 => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0xB4 => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0xAC => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0xBC => Some(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION LOAD ---
            // --- BEGIN SECTION STORE ---
            0x85 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0x95 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0x8D => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0x9D => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 5,
            }),
            0x99 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 5,
            }),
            0x81 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::IndX,
                value: opcode,
                base_cycle: 6,
            }),
            0x91 => Some(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::IndY,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION STORE ---
            // --- BEGIN SECTION TRANSFER ---
            0xAA => Some(OpCode {
                instr: Instruction::TAX,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0xA8 => Some(OpCode {
                instr: Instruction::TAY,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0x8A => Some(OpCode {
                instr: Instruction::TXA,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0x98 => Some(OpCode {
                instr: Instruction::TYA,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION TRANSFER ---
            // --- BEGIN SECTION LSR ---
            0x4A => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::Acc,
                value: opcode,
                base_cycle: 2,
            }),
            0x46 => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 5,
            }),
            0x56 => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 6,
            }),
            0x4E => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 6,
            }),
            0x5E => Some(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION LSR ---
            // --- BEGIN SECTION ROL ---
            0x2A => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::Acc,
                value: opcode,
                base_cycle: 2,
            }),
            0x26 => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 5,
            }),
            0x36 => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 6,
            }),
            0x2E => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 6,
            }),
            0x3E => Some(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ROL ---
            // --- BEGIN SECTION ROR ---
            0x6A => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::Acc,
                value: opcode,
                base_cycle: 2,
            }),
            0x66 => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 5,
            }),
            0x76 => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 6,
            }),
            0x6E => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 6,
            }),
            0x7E => Some(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ROR ---
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
            // --- BEGIN SECTION ORA ---
            0x09 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0x05 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0x15 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0x0D => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0x1D => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 4,
            }),
            0x19 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 4,
            }),
            0x01 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::IndX,
                value: opcode,
                base_cycle: 6,
            }),
            0x11 => Some(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::IndY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION ORA ---
            // --- BEGIN SECTION EOR ---
            0x49 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0x45 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0x55 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0x4D => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0x5D => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 4,
            }),
            0x59 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 4,
            }),
            0x41 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::IndX,
                value: opcode,
                base_cycle: 6,
            }),
            0x51 => Some(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::IndY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION EOR ---
            // --- BEGIN SECTION BIT ---
            0x24 => Some(OpCode {
                instr: Instruction::BIT,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0x2C => Some(OpCode {
                instr: Instruction::BIT,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION BIT ---
            // --- BEGIN SECTION CMP ---
            0xC9 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0xC5 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0xD5 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::ZpX,
                value: opcode,
                base_cycle: 4,
            }),
            0xCD => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            0xDD => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::AbsX,
                value: opcode,
                base_cycle: 4,
            }),
            0xD9 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::AbsY,
                value: opcode,
                base_cycle: 4,
            }),
            0xC1 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::IndX,
                value: opcode,
                base_cycle: 6,
            }),
            0xD1 => Some(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::IndY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION CMP ---
            // --- BEGIN SECTION CPX ---
            0xE0 => Some(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0xE4 => Some(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0xEC => Some(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION CPX ---
            // --- BEGIN SECTION CPY ---
            0xC0 => Some(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::Imm,
                value: opcode,
                base_cycle: 2,
            }),
            0xC4 => Some(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::Zp,
                value: opcode,
                base_cycle: 3,
            }),
            0xCC => Some(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION CPY ---
            // --- BEGIN SECTION JMP ---
            0x4C => Some(OpCode {
                instr: Instruction::JMP,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 3,
            }),
            0x6C => Some(OpCode {
                instr: Instruction::JMP,
                mode: AddressingMode::Ind,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION JMP ---
            // --- BEGIN SECTION JSR ---
            0x20 => Some(OpCode {
                instr: Instruction::JSR,
                mode: AddressingMode::Abs,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION JSR ---
            // --- BEGIN SECTION JSR ---
            0x60 => Some(OpCode {
                instr: Instruction::RTS,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION JSR ---
            // --- BEGIN SECTION BRK ---
            0x00 => Some(OpCode {
                instr: Instruction::BRK,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION BRK ---
            // --- BEGIN SECTION RTI ---
            0x40 => Some(OpCode {
                instr: Instruction::RTI,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION RTI ---
            // --- BEGIN SECTION PH/PL ---
            0x48 => Some(OpCode {
                instr: Instruction::PHA,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 3,
            }),
            0x08 => Some(OpCode {
                instr: Instruction::PHP,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 3,
            }),
            0x68 => Some(OpCode {
                instr: Instruction::PLA,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 4,
            }),
            0x28 => Some(OpCode {
                instr: Instruction::PLP,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION PH/PL ---
            // --- END SECTION TXS/TSX ---
            0xBA => Some(OpCode {
                instr: Instruction::TSX,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0x9A => Some(OpCode {
                instr: Instruction::TXS,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION TXS/TSX ---
            // --- BEGIN SECTION SET/CLEAR FLAGS ---
            0x18 => Some(OpCode {
                instr: Instruction::CLC,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0xD8 => Some(OpCode {
                instr: Instruction::CLD,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0x58 => Some(OpCode {
                instr: Instruction::CLI,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0xB8 => Some(OpCode {
                instr: Instruction::CLV,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0x38 => Some(OpCode {
                instr: Instruction::SEC,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0xF8 => Some(OpCode {
                instr: Instruction::SED,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            0x78 => Some(OpCode {
                instr: Instruction::SEI,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION SET/CLEAR FLAGS ---
            // --- BEGIN SECTION MISC ---
            0xEA => Some(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::Imp,
                value: opcode,
                base_cycle: 2,
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
                let addr = u16::from_le_bytes([low, high]);
                (addr, false)
            }
            AddressingMode::IndX => {
                let arg = self.fetch_byte(memory);
                let ptr = arg.wrapping_add(self.x);
                let low = memory.read_byte(ptr as u16);
                let high = memory.read_byte(ptr.wrapping_add(1) as u16);
                let addr = u16::from_le_bytes([low, high]);
                (addr, false)
            }
            AddressingMode::IndY => {
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

        let result: u16 = self.a as u16 + value as u16 + self.p.contains(StatusRegister::C) as u16;
        let prev_value = self.a;
        self.a = result as u8;
        self.p.set(StatusRegister::C, result > 0xFF);
        self.p.update_zero_flag(self.a);
        // If the result's sign is different from both A's and memory's, signed overflow (or underflow) occurred.
        self.p.set(
            StatusRegister::V,
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

        let result = self.a as u16 + (!value) as u16 + self.p.contains(StatusRegister::C) as u16;
        let prev_value = self.a;
        self.a = result as u8;
        self.p.set(StatusRegister::C, result >= 0x100);
        self.p.update_zero_flag(self.a);
        // If the result's sign is different from both A's and memory's, signed overflow (or underflow) occurred.
        self.p.set(
            StatusRegister::V,
            (self.a ^ prev_value) & (self.a ^ value) & 0x80 != 0,
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
        operation: BitwiseOp,
    ) -> Option<u8> {
        let value = match operand {
            Operand::Accumulator => self.a,
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        self.a = match operation {
            BitwiseOp::And => self.a & value,
            BitwiseOp::Or => self.a | value,
            BitwiseOp::Xor => self.a ^ value,
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
        self.p.set(StatusRegister::V, (value & 0b01000000) > 0);
        self.p.set(StatusRegister::N, (value & 0b10000000) > 0);

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
        self.p.set(StatusRegister::C, reg >= value);
        self.p.set(StatusRegister::Z, reg == value);
        self.p
            .set(StatusRegister::N, (reg.wrapping_sub(value)) & 0x80 != 0);

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
        self.p.set(StatusRegister::C, value & 0x80 != 0);
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
        self.p.set(StatusRegister::C, value & 0x01 > 0);
        self.p.update_zero_flag(shifted_value);
        self.p.set(StatusRegister::N, false);

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
        let shifted_value = (value << 1) | (self.p.contains(StatusRegister::C) as u8);
        self.p.set(StatusRegister::C, value & 0x80 != 0);
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

        let shifted_value = (value >> 1) | ((self.p.contains(StatusRegister::C) as u8) << 7);
        self.p.set(StatusRegister::C, value & 0x01 != 0);
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

    fn instr_load<T: MemoryBus>(
        &mut self,
        memory: &T,
        register: Register,
        operand: Operand,
    ) -> Option<u8> {
        let value = match operand {
            Operand::Immediate(val) => val,
            Operand::Memory(addr, _) => memory.read_byte(addr),
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };

        let reg = self.get_register_mut(register);
        *reg = value;

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
        operation: ArithmeticOp,
    ) -> Option<u8> {
        let reg = self.get_register_mut(register);
        let value = match operation {
            ArithmeticOp::Inc => (*reg).wrapping_add(1),
            ArithmeticOp::Dec => (*reg).wrapping_sub(1),
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
        let new_pc = self.pc + 2;
        self.sp.push_byte(memory, (new_pc >> 8) as u8);
        self.sp.push_byte(memory, (new_pc & 0xFF) as u8);

        self.pc = match operand {
            Operand::Memory(addr, _) => addr,
            _ => panic!("Unsupported operand {operand:?} for this instruction"),
        };
        None
    }

    fn instr_return_from_subroutine<T: MemoryBus>(&mut self, memory: &mut T) -> Option<u8> {
        let low = self.sp.pop_byte(memory);
        let high = self.sp.pop_byte(memory);

        self.pc = u16::from_le_bytes([low, high]) + 1;
        None
    }

    fn instr_break<T: MemoryBus>(&mut self, memory: &mut T) -> Option<u8> {
        let pc_value = self.pc + 2;
        // When we get an IRQ we push the current PC and processor flags to the stack.
        self.sp.push_byte(memory, (pc_value >> 8) as u8);
        self.sp.push_byte(memory, (pc_value & 0xFF) as u8);

        // The break flag must be set on the flags that are pushed to the stack, not the flags in the CPU
        let mut current_flag = self.p.clone();
        current_flag.set(StatusRegister::B, true);
        self.sp.push_byte(memory, current_flag.bits());

        self.pc = u16::from_le_bytes([memory.read_byte(0xFFFE), memory.read_byte(0xFFFF)]);
        self.p.set(StatusRegister::I, true);

        None
    }

    fn instr_return_from_interrupt<T: MemoryBus>(&mut self, memory: &mut T) -> Option<u8> {
        let flags = self.sp.pop_byte(memory);
        let pc_low = self.sp.pop_byte(memory);
        let pc_high = self.sp.pop_byte(memory);
        self.pc = u16::from_le_bytes([pc_low, pc_high]);

        self.p = StatusRegister::from_bits_truncate(flags);
        self.p.remove(StatusRegister::Unused | StatusRegister::B);

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
        let flags = self.p.union(StatusRegister::B | StatusRegister::Unused);
        self.sp.push_byte(memory, flags.bits());

        None
    }

    fn instr_pull_flags_from_sp<T: MemoryBus>(&mut self, memory: &T) -> Option<u8> {
        let value = self.sp.pop_byte(memory);

        self.p = StatusRegister::from_bits_truncate(value);
        self.p.remove(StatusRegister::Unused | StatusRegister::B);

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
        memory.write_byte(0x00, 0x34);
        memory.write_byte(0x01, 0x12);
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Abs);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsX);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsX);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsX);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsY);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsY);
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
        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::AbsY);
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

        let (addr, page_crossed) = cpu.get_operand_address(&memory, AddressingMode::Ind);
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
