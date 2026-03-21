#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    /// Branch If Carry Clear: jumps to a relative memory location if the carry flag is clear.
    BCC,
    /// Branch If Carry Set: jumps to a relative memory location if the carry flag is set.
    BCS,
    /// Branch If Equal (Comparison): jumps to a relative memory location if the comparison performed returned equality.
    BEQ,
    /// Branch If Not Equal (Comparison): jumps to a relative memory location if the comparison performed returned non-equality.
    BNE,
    /// Branch If Plus (Comparison): jumps to a relative memory location if the comparison performed returned "greater than".
    BPL,
    /// Branch If Minus (Comparison): jumps to a relative memory location if the comparison performed returned "less than".
    BMI,
    /// Branch If Overflow Clear: jumps to a relative memory location if the overflow flag is clear.
    BVC,
    /// Branch If Overflow Set: jumps to a relative memory location if the overflow flag is set.
    BVS,
    /// Add with Carry: adds carry flag and a memory value to the accumulator.
    ADC,
    /// Substract with Carry: subtracts a memory value and the NOT of the carry flag from the accumulator.
    SBC,
    /// Increment Memory: adds 1 to a memory location.
    INC,
    /// Decrement Memory: substract 1 from a memory location.
    DEC,
    /// Increment X: adds 1 to the X register.
    INX,
    /// Decrement X: substracts 1 from the X register.
    DEX,
    /// Increment Y: adds 1 to the Y register.
    INY,
    /// Decrements Y: substracts 1 from the Y register.
    DEY,
    /// Increments and Substracts with Carry (Undocumented): performs INC followed by SBC.
    ISB,
    /// Decrements and Compare (Undocumented): performs DEC followed by CMP.
    DCP,
    /// Arithmetic Shift Left and Bitwise OR (Undocumented): performs ASL followed by ORA.
    SLO,
    /// Rotate Left and Bitwise AND (Undocumented): performs ROL followed by AND.
    RLA,
    /// Logical Shift Right and Bitwise Exclusive OR (Undocumented): performs LSR followed by EOR.
    SRE,
    /// Rotate Right and Add with Carry (Undocumented): performs ROR followed by ADC.
    RRA,
    /// Load A: loads a memory value into the accumulator.
    LDA,
    /// Load X: loads a memory value into the X register.
    LDX,
    /// Load Y: loads a memory value into the X register.
    LDY,
    /// Load AX (Undocumented): loads a memory value into the accumulator and the X register.
    LAX,
    /// Store A: stores the accumulator value into memory.
    STA,
    /// Store X: stores the X register value into memory.
    STX,
    /// Store Y: stores the Y register value into memory.
    STY,
    /// Store AX (Undocumented): stores the result of (A & X) into memory.
    SAX,
    /// Transfer A to X: copies the accumulator value to the X register.
    TAX,
    /// Transfer A to Y: copies the accumulator value to the Y register.
    TAY,
    /// Transfer X to A: copies the X register value to the accumulator.
    TXA,
    /// Transfer Y to A: copies the Y register value to the accumulator.
    TYA,
    /// Arithmetic Shift Left: shifts all the bits of a memory value or the accumulator one position to the left, moving the value of each bit into the next bit.
    ASL,
    /// Logical Shift Left: shifts all the bits of a memory value or the accumulator one position to the right, moving the value of each bit into the next bit.
    LSR,
    /// Rotate Left: shifts a memory value or the accumulator to the left, moving the value of each bit into the next bit and treating the carry flag as though it is both above bit 7 and below bit 0.
    ROL,
    /// Rotate Right: shifts a memory value or the accumulator to the right, moving the value of each bit into the next bit and treating the carry flag as though it is both above bit 7 and below bit 0.
    ROR,
    /// Bitwise AND: store in the accumulator the result of bitwise AND between the accumulator and a memory value.
    AND,
    /// Bitwise OR: store in the accumulator the result of bitwise OR between the accumulator and a memory value.
    ORA,
    /// Bitwise Exclusive OR: store in the accumulator the result of bitwise XOR between the accumulator and a memory value.
    EOR,
    /// Bit Test: Change flag depending on the result of (a & memory).
    BIT,
    /// Compare A: Compares accumulator to a memory value, setting flags as appropriate but not modifying any registers.
    CMP,
    /// Compare X: Compares register X to a memory value, setting flags as appropriate but not modifying any registers.
    CPX,
    /// Compare Y: Compares register Y to a memory value, setting flags as appropriate but not modifying any registers.
    CPY,
    /// Jump: Sets the program counter to a new value, allowing code to execute from a new location.
    JMP,
    /// Jump to Subroutine: pushes the current program counter to the stack and then sets the program counter to a new value.
    JSR,
    /// Returns from Subroutine: pulls an address from the stack into the program counter and then increments the program counter.
    RTS,
    /// Break (Software IRQ): triggers an interrupt request (IRQ).
    BRK,
    /// Return from Interrupt: returns from an interrupt handler, first pulling the 6 status flags from the stack and then pulling the new program counter.
    RTI,
    /// Push A: stores the value of A to the current stack position and then decrements the stack pointer.
    PHA,
    /// Pull A: increments the stack pointer and then loads the value at that stack position into A.
    PLA,
    /// Push Processor Status: stores a byte to the stack containing the 6 status flags and B flag and then decrements the stack pointer.
    PHP,
    /// Pull Processor Status: increments the stack pointer and then loads the value at that stack position into the 6 status flags.
    PLP,
    /// Transfer X to Stack Pointer: copies the X register value to the stack pointer.
    TXS,
    /// Transfer Stack Pointer to X: copies the stack pointer value to the X register.
    TSX,
    // Clear Carry: clears the carry flag.
    CLC,
    /// Set Carry: sets the carry flag.
    SEC,
    /// Clear Interrupt Disable: clears the interrupt disable flag.
    CLI,
    /// Set Interrupt Disable: sets the interrupt disable flag.
    SEI,
    /// Clear Decimal: clears the decimal flag.
    CLD,
    /// Set Decimal: sets the decimal flag.
    SED,
    /// Clear Overflow: clears the overflow flag.
    CLV,
    // No Operation: no effect, just pass CPU cycles.
    NOP,
}
