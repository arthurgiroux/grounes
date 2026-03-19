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
    ISB, // undocumented opcode, performs INC + SBC
    DCP, // undocumented opcode, performs DEC + CMP
    SLO, // undocumented opcode, performs ASL + ORA
    RLA, // undocumented opcode, performs ROL + AND
    SRE, // undocumented opcode, performs LSR + EOR
    RRA, // undocumented opcode, performs ROR + ADC
    // Load
    LDA,
    LDX,
    LDY,
    LAX, // undocumented opcode, loads into both A and X
    // Store
    STA,
    STX,
    STY,
    SAX, // undocumented opcode, store the result of (A & X)
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
