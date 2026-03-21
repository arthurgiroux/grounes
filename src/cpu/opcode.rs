use crate::cpu::addressing_mode::AddressingMode;
use crate::cpu::instruction::Instruction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpCode {
    /// The instruction that will be executed from this opcode
    pub instr: Instruction,
    /// The addressing mode, this will determine how to fetch the operand
    pub mode: AddressingMode,
    /// The value of the opcode
    pub value: u8,
    /// The usual number of cycles that the CPU takes to execute this opcode, additional cycles can be added depending on the addressing mode
    pub base_cycle: u8,
}

// 6510 Instructions by Addressing Modes

// off- ++++++++++ Positive ++++++++++  ---------- Negative ----------
// set  00      20      40      60      80      a0      c0      e0      mode

// +00  BRK     JSR     RTI     RTS     NOP*    LDY     CPY     CPX     Impl/immed
// +01  ORA     AND     EOR     ADC     STA     LDA     CMP     SBC     (indir,x)
// +02   t       t       t       t      NOP*t   LDX     NOP*t   NOP*t     ? /immed
// +03  SLO*    RLA*    SRE*    RRA*    SAX*    LAX*    DCP*    ISB*    (indir,x)
// +04  NOP*    BIT     NOP*    NOP*    STY     LDY     CPY     CPX     Zeropage
// +05  ORA     AND     EOR     ADC     STA     LDA     CMP     SBC     Zeropage
// +06  ASL     ROL     LSR     ROR     STX     LDX     DEC     INC     Zeropage
// +07  SLO*    RLA*    SRE*    RRA*    SAX*    LAX*    DCP*    ISB*    Zeropage

// +08  PHP     PLP     PHA     PLA     DEY     TAY     INY     INX     Implied
// +09  ORA     AND     EOR     ADC     NOP*    LDA     CMP     SBC     Immediate
// +0a  ASL     ROL     LSR     ROR     TXA     TAX     DEX     NOP     Accu/impl
// +0b  ANC**   ANC**   ASR**   ARR**   ANE**   LXA**   SBX**   SBC*    Immediate
// +0c  NOP*    BIT     JMP     JMP ()  STY     LDY     CPY     CPX     Absolute
// +0d  ORA     AND     EOR     ADC     STA     LDA     CMP     SBC     Absolute
// +0e  ASL     ROL     LSR     ROR     STX     LDX     DEC     INC     Absolute
// +0f  SLO*    RLA*    SRE*    RRA*    SAX*    LAX*    DCP*    ISB*    Absolute

// +10  BPL     BMI     BVC     BVS     BCC     BCS     BNE     BEQ     Relative
// +11  ORA     AND     EOR     ADC     STA     LDA     CMP     SBC     (indir),y
// +12   t       t       t       t       t       t       t       t         ?
// +13  SLO*    RLA*    SRE*    RRA*    SHA**   LAX*    DCP*    ISB*    (indir),y
// +14  NOP*    NOP*    NOP*    NOP*    STY     LDY     NOP*    NOP*    Zeropage,x
// +15  ORA     AND     EOR     ADC     STA     LDA     CMP     SBC     Zeropage,x
// +16  ASL     ROL     LSR     ROR     STX  y) LDX  y) DEC     INC     Zeropage,x
// +17  SLO*    RLA*    SRE*    RRA*    SAX* y) LAX* y) DCP*    ISB*    Zeropage,x

// +18  CLC     SEC     CLI     SEI     TYA     CLV     CLD     SED     Implied
// +19  ORA     AND     EOR     ADC     STA     LDA     CMP     SBC     Absolute,y
// +1a  NOP*    NOP*    NOP*    NOP*    TXS     TSX     NOP*    NOP*    Implied
// +1b  SLO*    RLA*    SRE*    RRA*    SHS**   LAS**   DCP*    ISB*    Absolute,y
// +1c  NOP*    NOP*    NOP*    NOP*    SHY**   LDY     NOP*    NOP*    Absolute,x
// +1d  ORA     AND     EOR     ADC     STA     LDA     CMP     SBC     Absolute,x
// +1e  ASL     ROL     LSR     ROR     SHX**y) LDX  y) DEC     INC     Absolute,x
// +1f  SLO*    RLA*    SRE*    RRA*    SHA**y) LAX* y) DCP*    ISB*    Absolute,x

//         ROR intruction is available on MC650x microprocessors after
//         June, 1976.

//         Legend:

//         t       Jams the machine
//         *t      Jams very rarely
//         *       Undocumented command
//         **      Unusual operation
//         y)      indexed using Y instead of X
//         ()      indirect instead of absolute

// Note that the NOP instructions do have other addressing modes than the
// implied addressing. The NOP instruction is just like any other load
// instruction, except it does not store the result anywhere nor affects the
// flags.
//
// Source: https://www.nesdev.org/6502_cpu.txt
impl TryFrom<u8> for OpCode {
    type Error = &'static str;

    fn try_from(opcode: u8) -> Result<Self, Self::Error> {
        match opcode {
            // --- BEGIN SECTION ADC ---
            0x69 => Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x65 => Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x75 => Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x6D => Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x7D => Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0x79 => Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0x61 => Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x71 => Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION ADC ---
            // --- BEGIN SECTION SBC ---
            0xE9 | 0xEB => Ok(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xE5 => Ok(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xF5 => Ok(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0xED => Ok(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xFD => Ok(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0xF9 => Ok(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0xE1 => Ok(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0xF1 => Ok(OpCode {
                instr: Instruction::SBC,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION SBC ---
            // --- BEGIN SECTION INC ---
            0xE6 => Ok(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0xF6 => Ok(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0xEE => Ok(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0xFE => Ok(OpCode {
                instr: Instruction::INC,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0xE8 => Ok(OpCode {
                instr: Instruction::INX,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xC8 => Ok(OpCode {
                instr: Instruction::INY,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION INC ---
            // --- BEGIN SECTION ISB ---
            0xE7 => Ok(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0xF7 => Ok(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0xEF => Ok(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0xFF => Ok(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0xFB => Ok(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0xE3 => Ok(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0xF3 => Ok(OpCode {
                instr: Instruction::ISB,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION ISB ---
            // --- BEGIN SECTION DEC ---
            0xC6 => Ok(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0xD6 => Ok(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0xCE => Ok(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0xDE => Ok(OpCode {
                instr: Instruction::DEC,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0xCA => Ok(OpCode {
                instr: Instruction::DEX,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x88 => Ok(OpCode {
                instr: Instruction::DEY,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION DEC ---
            // --- BEGIN SECTION DCP ---
            0xC7 => Ok(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0xD7 => Ok(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0xCF => Ok(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0xDF => Ok(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0xDB => Ok(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0xC3 => Ok(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0xD3 => Ok(OpCode {
                instr: Instruction::DCP,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION DCP ---
            // --- BEGIN SECTION ASL ---
            0x0A => Ok(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::Accumulator,
                value: opcode,
                base_cycle: 2,
            }),
            0x06 => Ok(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x16 => Ok(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x0E => Ok(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x1E => Ok(OpCode {
                instr: Instruction::ASL,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ASL ---
            // --- BEGIN SECTION SLO ---
            0x07 => Ok(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x17 => Ok(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x0F => Ok(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x1F => Ok(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0x1B => Ok(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0x03 => Ok(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0x13 => Ok(OpCode {
                instr: Instruction::SLO,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION SLO ---
            // --- BEGIN SECTION SRE ---
            0x47 => Ok(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x57 => Ok(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x4F => Ok(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x5B => Ok(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0x5F => Ok(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0x43 => Ok(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0x53 => Ok(OpCode {
                instr: Instruction::SRE,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION SRE ---
            // --- BEGIN SECTION RRA ---
            0x67 => Ok(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x77 => Ok(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x6F => Ok(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x7B => Ok(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0x7F => Ok(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            0x63 => Ok(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0x73 => Ok(OpCode {
                instr: Instruction::RRA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            // --- END SECTION RRA ---
            // --- BEGIN BRANCH INSTRUCTIONS ---
            0x90 => Ok(OpCode {
                instr: Instruction::BCC,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0xB0 => Ok(OpCode {
                instr: Instruction::BCS,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0xF0 => Ok(OpCode {
                instr: Instruction::BEQ,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0xD0 => Ok(OpCode {
                instr: Instruction::BNE,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0x10 => Ok(OpCode {
                instr: Instruction::BPL,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0x30 => Ok(OpCode {
                instr: Instruction::BMI,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0x50 => Ok(OpCode {
                instr: Instruction::BVC,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            0x70 => Ok(OpCode {
                instr: Instruction::BVS,
                mode: AddressingMode::Relative,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION BRANCH ---
            // --- BEGIN SECTION LOAD ---
            0xA9 => Ok(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xA5 => Ok(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xB5 => Ok(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0xAD => Ok(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xBD => Ok(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0xB9 => Ok(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0xA1 => Ok(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0xB1 => Ok(OpCode {
                instr: Instruction::LDA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            0xA2 => Ok(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xA6 => Ok(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xB6 => Ok(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::ZeroPageY,
                value: opcode,
                base_cycle: 4,
            }),
            0xAE => Ok(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xBE => Ok(OpCode {
                instr: Instruction::LDX,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0xA0 => Ok(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xA4 => Ok(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xB4 => Ok(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0xAC => Ok(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xBC => Ok(OpCode {
                instr: Instruction::LDY,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0xA3 => Ok(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0xA7 => Ok(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xAF => Ok(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xB3 => Ok(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            0xB7 => Ok(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::ZeroPageY,
                value: opcode,
                base_cycle: 4,
            }),
            0xBF => Ok(OpCode {
                instr: Instruction::LAX,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION LOAD ---
            // --- BEGIN SECTION STORE ---
            0x85 => Ok(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x95 => Ok(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x8D => Ok(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x9D => Ok(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 5,
            }),
            0x99 => Ok(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 5,
            }),
            0x81 => Ok(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x91 => Ok(OpCode {
                instr: Instruction::STA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 6,
            }),
            0x86 => Ok(OpCode {
                instr: Instruction::STX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x96 => Ok(OpCode {
                instr: Instruction::STX,
                mode: AddressingMode::ZeroPageY,
                value: opcode,
                base_cycle: 4,
            }),
            0x8E => Ok(OpCode {
                instr: Instruction::STX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x84 => Ok(OpCode {
                instr: Instruction::STY,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x94 => Ok(OpCode {
                instr: Instruction::STY,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x8C => Ok(OpCode {
                instr: Instruction::STY,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x83 => Ok(OpCode {
                instr: Instruction::SAX,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x87 => Ok(OpCode {
                instr: Instruction::SAX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x8F => Ok(OpCode {
                instr: Instruction::SAX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x97 => Ok(OpCode {
                instr: Instruction::SAX,
                mode: AddressingMode::ZeroPageY,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION STORE ---
            // --- BEGIN SECTION TRANSFER ---
            0xAA => Ok(OpCode {
                instr: Instruction::TAX,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xA8 => Ok(OpCode {
                instr: Instruction::TAY,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x8A => Ok(OpCode {
                instr: Instruction::TXA,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x98 => Ok(OpCode {
                instr: Instruction::TYA,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION TRANSFER ---
            // --- BEGIN SECTION LSR ---
            0x4A => Ok(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::Accumulator,
                value: opcode,
                base_cycle: 2,
            }),
            0x46 => Ok(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x56 => Ok(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x4E => Ok(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x5E => Ok(OpCode {
                instr: Instruction::LSR,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION LSR ---
            // --- BEGIN SECTION ROL ---
            0x2A => Ok(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::Accumulator,
                value: opcode,
                base_cycle: 2,
            }),
            0x26 => Ok(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x36 => Ok(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x2E => Ok(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x3E => Ok(OpCode {
                instr: Instruction::ROL,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ROL ---
            // --- BEGIN SECTION RLA ---
            0x23 => Ok(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 8,
            }),
            0x33 => Ok(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 8,
            }),
            0x2F => Ok(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x27 => Ok(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x37 => Ok(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x3B => Ok(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 7,
            }),
            0x3F => Ok(OpCode {
                instr: Instruction::RLA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION RLA ---
            // --- BEGIN SECTION ROR ---
            0x6A => Ok(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::Accumulator,
                value: opcode,
                base_cycle: 2,
            }),
            0x66 => Ok(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 5,
            }),
            0x76 => Ok(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 6,
            }),
            0x6E => Ok(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            0x7E => Ok(OpCode {
                instr: Instruction::ROR,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION ROR ---
            // --- BEGIN SECTION AND ---
            0x29 => Ok(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x25 => Ok(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x35 => Ok(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x2D => Ok(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x3D => Ok(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0x39 => Ok(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0x21 => Ok(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x31 => Ok(OpCode {
                instr: Instruction::AND,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION AND ---
            // --- BEGIN SECTION ORA ---
            0x09 => Ok(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x05 => Ok(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x15 => Ok(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x0D => Ok(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x1D => Ok(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0x19 => Ok(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0x01 => Ok(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x11 => Ok(OpCode {
                instr: Instruction::ORA,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION ORA ---
            // --- BEGIN SECTION EOR ---
            0x49 => Ok(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x45 => Ok(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x55 => Ok(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x4D => Ok(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x5D => Ok(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0x59 => Ok(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0x41 => Ok(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0x51 => Ok(OpCode {
                instr: Instruction::EOR,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION EOR ---
            // --- BEGIN SECTION BIT ---
            0x24 => Ok(OpCode {
                instr: Instruction::BIT,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x2C => Ok(OpCode {
                instr: Instruction::BIT,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION BIT ---
            // --- BEGIN SECTION CMP ---
            0xC9 => Ok(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xC5 => Ok(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xD5 => Ok(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0xCD => Ok(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0xDD => Ok(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            0xD9 => Ok(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::AbsoluteY,
                value: opcode,
                base_cycle: 4,
            }),
            0xC1 => Ok(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::IndirectX,
                value: opcode,
                base_cycle: 6,
            }),
            0xD1 => Ok(OpCode {
                instr: Instruction::CMP,
                mode: AddressingMode::IndirectY,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION CMP ---
            // --- BEGIN SECTION CPX ---
            0xE0 => Ok(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xE4 => Ok(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xEC => Ok(OpCode {
                instr: Instruction::CPX,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION CPX ---
            // --- BEGIN SECTION CPY ---
            0xC0 => Ok(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0xC4 => Ok(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0xCC => Ok(OpCode {
                instr: Instruction::CPY,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION CPY ---
            // --- BEGIN SECTION JMP ---
            0x4C => Ok(OpCode {
                instr: Instruction::JMP,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 3,
            }),
            0x6C => Ok(OpCode {
                instr: Instruction::JMP,
                mode: AddressingMode::Indirect,
                value: opcode,
                base_cycle: 5,
            }),
            // --- END SECTION JMP ---
            // --- BEGIN SECTION JSR ---
            0x20 => Ok(OpCode {
                instr: Instruction::JSR,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION JSR ---
            // --- BEGIN SECTION JSR ---
            0x60 => Ok(OpCode {
                instr: Instruction::RTS,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION JSR ---
            // --- BEGIN SECTION BRK ---
            0x00 => Ok(OpCode {
                instr: Instruction::BRK,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 7,
            }),
            // --- END SECTION BRK ---
            // --- BEGIN SECTION RTI ---
            0x40 => Ok(OpCode {
                instr: Instruction::RTI,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 6,
            }),
            // --- END SECTION RTI ---
            // --- BEGIN SECTION PH/PL ---
            0x48 => Ok(OpCode {
                instr: Instruction::PHA,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 3,
            }),
            0x08 => Ok(OpCode {
                instr: Instruction::PHP,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 3,
            }),
            0x68 => Ok(OpCode {
                instr: Instruction::PLA,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 4,
            }),
            0x28 => Ok(OpCode {
                instr: Instruction::PLP,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION PH/PL ---
            // --- END SECTION TXS/TSX ---
            0xBA => Ok(OpCode {
                instr: Instruction::TSX,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x9A => Ok(OpCode {
                instr: Instruction::TXS,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION TXS/TSX ---
            // --- BEGIN SECTION SET/CLEAR FLAGS ---
            0x18 => Ok(OpCode {
                instr: Instruction::CLC,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xD8 => Ok(OpCode {
                instr: Instruction::CLD,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x58 => Ok(OpCode {
                instr: Instruction::CLI,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xB8 => Ok(OpCode {
                instr: Instruction::CLV,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x38 => Ok(OpCode {
                instr: Instruction::SEC,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0xF8 => Ok(OpCode {
                instr: Instruction::SED,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x78 => Ok(OpCode {
                instr: Instruction::SEI,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            // --- END SECTION SET/CLEAR FLAGS ---
            // --- BEGIN SECTION MISC ---
            0x04 | 0x44 | 0x64 => Ok(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::ZeroPage,
                value: opcode,
                base_cycle: 3,
            }),
            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => Ok(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::ZeroPageX,
                value: opcode,
                base_cycle: 4,
            }),
            0x0C => Ok(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::Absolute,
                value: opcode,
                base_cycle: 4,
            }),
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xEA | 0xFA => Ok(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::Implicit,
                value: opcode,
                base_cycle: 2,
            }),
            0x80 => Ok(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::Immediate,
                value: opcode,
                base_cycle: 2,
            }),
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => Ok(OpCode {
                instr: Instruction::NOP,
                mode: AddressingMode::AbsoluteX,
                value: opcode,
                base_cycle: 4,
            }),
            // --- END SECTION MISC ---
            _ => Err("Invalid opcode"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_valid_opcode_should_succeed() {
        assert_eq!(
            OpCode::try_from(0x69),
            Ok(OpCode {
                instr: Instruction::ADC,
                mode: AddressingMode::Immediate,
                value: 0x69,
                base_cycle: 2
            })
        );
    }

    #[test]
    fn decode_invalid_opcode_should_give_error() {
        assert_eq!(OpCode::try_from(0x02), Err("Invalid opcode"));
    }
}
