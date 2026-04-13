use crate::mapper::{Mapper, MapperSource};
use crate::ppu::nametable;

#[derive(Debug, Default)]
enum TileFetcherState {
    #[default]
    FetchNametable,
    FetchAttribute,
    FetchPatternLow,
    FetchPatternHigh,
}

const STATE_CYCLE_LENGTH: u8 = 2;

#[derive(Debug, Default)]
pub struct TileFetcher {
    nametable_byte: u8,
    attribute_table_byte: u8,
    pattern_table_tile_low: u8,
    pattern_table_tile_high: u8,
    current_state: TileFetcherState,
    current_state_cycles: u8,
}

impl TileFetcher {
    pub fn step(
        &mut self,
        reg_v: u16,
        pattern_table_addr: u16,
        vram: &[u8],
        mapper: &mut dyn Mapper,
        palette: &[u8],
        frame: &mut Vec<u8>,
    ) -> Option<Vec<u8>> {
        if self.current_state_cycles == 0 {
            let fine_y = (reg_v >> 12) & 0x07;
            let coarse_x = reg_v & 0x1F;
            let coarse_y = (reg_v >> 5) & 0x1F;

            match self.current_state {
                TileFetcherState::FetchNametable => {
                    // Source: https://www.nesdev.org/wiki/PPU_scrolling#Tile_and_attribute_fetching
                    let tile_addr = (0x2000 | (reg_v & 0x0FFF)) as usize;
                    self.nametable_byte = vram[tile_addr];
                }
                TileFetcherState::FetchAttribute => {
                    // Source: https://www.nesdev.org/wiki/PPU_scrolling#Tile_and_attribute_fetching
                    // The low 12 bits of the attribute address are composed in the following way:
                    //  NN 1111 YYY XXX
                    //  || |||| ||| +++-- high 3 bits of coarse X (x/4)
                    //  || |||| +++------ high 3 bits of coarse Y (y/4)
                    //  || ++++---------- attribute offset (960 bytes)
                    //  ++--------------- nametable select
                    let attr_addr = (0x23C0 | (reg_v & 0x0C00) | ((reg_v >> 4) & 0x38) | ((reg_v >> 2) & 0x07)) as usize;
                    self.attribute_table_byte = vram[attr_addr];
                }
                TileFetcherState::FetchPatternLow => {
                    let tile_addr_low = pattern_table_addr | ((self.nametable_byte as u16) << 4) | fine_y;
                    self.pattern_table_tile_low = mapper.read_byte(MapperSource::PPU, tile_addr_low);
                }
                TileFetcherState::FetchPatternHigh => {
                    let tile_addr_high = pattern_table_addr | ((self.nametable_byte as u16) << 4) | 0x08 | fine_y;
                    self.pattern_table_tile_high = mapper.read_byte(MapperSource::PPU, tile_addr_high);
                    let shift = ((coarse_y & 0x02) << 1) | (coarse_x & 0x02);
                    let palette_num = (self.attribute_table_byte >> shift) & 0x03;

                    // Output 8 pixels for this tile
                    let pixels: Vec<u8> = (0..8u8).map(|bit_pos| {
                        let bit = 7 - bit_pos;
                        let low_bit = (self.pattern_table_tile_low >> bit) & 1;
                        let high_bit = (self.pattern_table_tile_high >> bit) & 1;
                        let color_idx = (high_bit << 1) | low_bit;

                        if color_idx == 0 {
                            palette[0]
                        } else {
                            palette[(palette_num * 4 + color_idx) as usize]
                        }
                    }).collect();
                    return Some(pixels);
                }
            }
        }

        self.current_state_cycles += 1;
        if self.current_state_cycles >= STATE_CYCLE_LENGTH {
            self.transition_state();
        }

        None
    }

    fn transition_state(&mut self) {
        self.current_state_cycles = 0;
        self.current_state = match self.current_state {
            TileFetcherState::FetchNametable => TileFetcherState::FetchAttribute,
            TileFetcherState::FetchAttribute => TileFetcherState::FetchPatternLow,
            TileFetcherState::FetchPatternLow => TileFetcherState::FetchPatternHigh,
            TileFetcherState::FetchPatternHigh => TileFetcherState::FetchNametable,
        };
    }
}
