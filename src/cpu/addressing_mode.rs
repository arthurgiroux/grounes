#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    /// Some instructions have no address operand, the destination of results are implied.
    Implicit,
    /// Many instructions can operate on the accumulator a.
    Accumulator,
    /// Uses the 8-bit operand itself as the value for the operation, rather than fetching a value from a memory address.
    Immediate,
    /// Branch instructions have a relative addressing mode that specifies an 8-bit signed offset relative to the current PC.
    Relative,
    /// Fetches the value from an 8-bit address on the zero page.
    ZeroPage,
    /// Fetches the value from an 8-bit address (offsetted by X) on the zero page.
    ZeroPageX,
    /// Fetches the value from an 8-bit address (offsetted by Y) on the zero page.
    ZeroPageY,
    /// Fetches the value from a 16-bit address anywhere in memory.
    Absolute,
    /// Fetches the value from a 16-bit address (offsetted by X) anywhere in memory.
    AbsoluteX,
    /// Fetches the value from a 16-bit address (offsetted by Y) anywhere in memory.
    AbsoluteY,
    /// The JMP instruction has a special indirect addressing mode that can jump to the address stored in a 16-bit pointer anywhere in memory.
    Indirect,
    /// Indexed indirect: Adds X to a base address before fetching a pointer, then read the address
    IndirectX,
    /// Indirect Indexed: Fetches the pointer then adds Y and read the address there
    IndirectY,
}
