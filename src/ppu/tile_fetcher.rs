use crate::mapper::{Mapper, MapperSource};
use crate::ppu::nametable;

#[derive(Debug, Default)]
enum TileFetcherState {
    #[default]
    FetchNametable,
    FetchAttribute,
    FetchTileLow,
    FetchTileHigh,
}

const STATE_CYCLE_LENGTH: u8 = 2;

// Standard NES NTSC system palette (64 colors, RGB)
const SYSTEM_PALETTE: [(u8, u8, u8); 64] = [
    (84, 84, 84),
    (0, 30, 116),
    (8, 16, 144),
    (48, 0, 136),
    (68, 0, 100),
    (92, 0, 48),
    (84, 4, 0),
    (60, 24, 0),
    (32, 42, 0),
    (8, 58, 0),
    (0, 64, 0),
    (0, 60, 0),
    (0, 50, 60),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (152, 150, 152),
    (8, 76, 196),
    (48, 50, 236),
    (92, 30, 228),
    (136, 20, 176),
    (160, 20, 100),
    (152, 34, 32),
    (120, 60, 0),
    (84, 90, 0),
    (40, 114, 0),
    (8, 124, 0),
    (0, 118, 40),
    (0, 102, 120),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (236, 238, 236),
    (76, 154, 236),
    (120, 124, 236),
    (176, 98, 236),
    (228, 84, 236),
    (236, 88, 180),
    (236, 106, 100),
    (212, 136, 32),
    (160, 170, 0),
    (116, 196, 0),
    (76, 208, 32),
    (56, 204, 108),
    (56, 180, 204),
    (60, 60, 60),
    (0, 0, 0),
    (0, 0, 0),
    (236, 238, 236),
    (168, 204, 236),
    (188, 188, 236),
    (212, 178, 236),
    (236, 174, 236),
    (236, 174, 212),
    (236, 180, 176),
    (228, 196, 144),
    (204, 210, 120),
    (180, 222, 120),
    (168, 226, 144),
    (152, 226, 180),
    (160, 214, 228),
    (160, 162, 160),
    (0, 0, 0),
    (0, 0, 0),
];

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
        scanline: u16,
        x: u16,
        vram: &[u8],
        mapper: &mut dyn Mapper,
        nametable_addr: u16,
        pattern_base: u16,
        palette: &[u8],
        frame: &mut Vec<u8>,
    ) {
        if self.current_state_cycles == 0 {
            let tile_col = x / 8;
            let tile_row = scanline / 8;
            let fine_y = scanline % 8;

            match self.current_state {
                TileFetcherState::FetchNametable => {
                    let offset = tile_row * nametable::WIDTH as u16 + tile_col;
                    let vram_idx = (nametable_addr - 0x2000 + offset) as usize & (vram.len() - 1);
                    self.nametable_byte = vram[vram_idx];
                }
                TileFetcherState::FetchAttribute => {
                    let attr_base = (nametable_addr - 0x2000 + 0x3C0) as usize;
                    let attr_offset = (tile_row / 4) * 8 + (tile_col / 4);
                    let vram_idx = (attr_base + attr_offset as usize) & (vram.len() - 1);
                    self.attribute_table_byte = vram[vram_idx];
                }
                TileFetcherState::FetchTileLow => {
                    let addr = pattern_base + self.nametable_byte as u16 * 16 + fine_y;
                    self.pattern_table_tile_low = mapper.read_byte(MapperSource::PPU, addr);
                }
                TileFetcherState::FetchTileHigh => {
                    let addr = pattern_base + self.nametable_byte as u16 * 16 + fine_y + 8;
                    self.pattern_table_tile_high = mapper.read_byte(MapperSource::PPU, addr);

                    // Output 8 pixels for this tile
                    let tile_x_start = (x / 8) * 8;
                    let attr_quadrant_x = (tile_col / 2) % 2;
                    let attr_quadrant_y = (tile_row / 2) % 2;
                    let attr_shift = (attr_quadrant_y * 2 + attr_quadrant_x) * 2;
                    let palette_num = (self.attribute_table_byte >> attr_shift) & 0x03;

                    for bit_pos in 0..8u16 {
                        let bit = 7 - bit_pos as u8;
                        let low_bit = (self.pattern_table_tile_low >> bit) & 1;
                        let high_bit = (self.pattern_table_tile_high >> bit) & 1;
                        let color_idx = (high_bit << 1) | low_bit;

                        let palette_entry = if color_idx == 0 {
                            palette[0] as usize
                        } else {
                            palette[(palette_num * 4 + color_idx) as usize] as usize
                        };
                        let (r, g, b) = SYSTEM_PALETTE[palette_entry & 0x3F];

                        let pixel_x = tile_x_start + bit_pos;
                        let frame_idx = (scanline as usize * 256 + pixel_x as usize) * 3;
                        frame[frame_idx] = r;
                        frame[frame_idx + 1] = g;
                        frame[frame_idx + 2] = b;
                    }
                }
            }
        }

        self.current_state_cycles += 1;
        if self.current_state_cycles >= STATE_CYCLE_LENGTH {
            self.transition_state();
        }
    }

    fn transition_state(&mut self) {
        self.current_state_cycles = 0;
        self.current_state = match self.current_state {
            TileFetcherState::FetchNametable => TileFetcherState::FetchAttribute,
            TileFetcherState::FetchAttribute => TileFetcherState::FetchTileLow,
            TileFetcherState::FetchTileLow => TileFetcherState::FetchTileHigh,
            TileFetcherState::FetchTileHigh => TileFetcherState::FetchNametable,
        };
    }
}
