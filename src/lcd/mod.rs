pub const HEIGHT: usize = 272;
pub const WIDTH: usize = 480;

pub const LAYER_1_OCTETS_PER_PIXEL: usize = 4;
pub const LAYER_1_LENGTH: usize = HEIGHT * WIDTH * LAYER_1_OCTETS_PER_PIXEL;
pub const LAYER_2_OCTETS_PER_PIXEL: usize = 2;
pub const LAYER_2_LENGTH: usize = HEIGHT * WIDTH * LAYER_2_OCTETS_PER_PIXEL;

pub const SDRAM_START: usize = 0xC000_0000;
pub const LAYER_1_START: usize = SDRAM_START;
pub const LAYER_2_START: usize = SDRAM_START + LAYER_1_LENGTH;
