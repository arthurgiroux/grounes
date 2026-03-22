# Grounes - NES Emulator

[![Build Status](https://github.com/arthurgiroux/grounes/actions/workflows/ci.yml/badge.svg)](https://github.com/arthurgiroux/grounes/actions/workflows/ci.yml)
[![Coverage Status](https://coveralls.io/repos/github/arthurgiroux/grounes/badge.svg?branch=main)](https://coveralls.io/github/arthurgiroux/grounes?branch=main)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Grounes is an NES emulator written in Rust, focused on accuracy and clean code. The project emphasizes correct 6502 CPU emulation validated against the nestest test suite.

## Currently Implemented

- **6502 CPU**: All standard and undocumented opcodes (ISB, DCP, SLO, RLA, SRE, RRA, LAX, SAX)
- **Addressing modes**: 13 modes (Immediate, Absolute, Zero Page, Indexed, Indirect, Relative, and variants)
- **Memory**: 2KB internal RAM with proper mirroring
- **iNES ROM format**: Full parser supporting headers and trainer data
- **Mapper 0 (NROM)**: Basic cartridge mapping support
- **CPU Validation**: Passes the nestest integration test, ensuring cycle-accurate instruction execution

## Planned

- **PPU** (graphics rendering)
- **APU** (audio processing)
- **Additional mappers** (MMC1, UxROM, CNROM, etc.)
- **Controller input**
- **Display output** 
- **WebAssembly** 

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test              # Run all tests
cargo test nestest      # Run the CPU validation test
```

## References

- [NESdev Wiki](https://www.nesdev.org/wiki/) - Comprehensive NES hardware documentation
- [6502 Instruction Reference](https://www.obelisk.me.uk/6502/reference.html) - 6502 CPU instruction set
- [nestest](https://github.com/kevtris/nes-test-roms) by Kevin Horton - The reference CPU validation test suite

## License

MIT License - See LICENSE file for details.
