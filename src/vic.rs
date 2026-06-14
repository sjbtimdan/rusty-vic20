use crate::{
    addressable::Addressable,
    bus::{CHARSET_SIZE, SCREEN_RAM_SIZE},
    ui::screen::renderer::{ACTIVE_HEIGHT, CHAR_HEIGHT, CHAR_WIDTH, palette},
};

const HORIZONTAL_ORIGIN_OFFSET: usize = 0x00;
const VERTICAL_ORIGIN_OFFSET: usize = 0x01;
const COLUMNS_AND_SCREEN_SELECT_OFFSET: usize = 0x02;
const ROWS_AND_DOUBLE_HEIGHT_OFFSET: usize = 0x03;
const RASTER_LINE_OFFSET: usize = 0x04;
const SCREEN_AND_CHAR_BASE_OFFSET: usize = 0x05;
const LIGHT_PEN_HORIZONTAL_OFFSET: usize = 0x06;
const LIGHT_PEN_VERTICAL_OFFSET: usize = 0x07;
const PADDLE_X_OFFSET: usize = 0x08;
const PADDLE_Y_OFFSET: usize = 0x09;
const FREQUENCY_1_OFFSET: usize = 0x0A;
const FREQUENCY_2_OFFSET: usize = 0x0B;
const FREQUENCY_3_OFFSET: usize = 0x0C;
const NOISE_AND_CONTROL_OFFSET: usize = 0x0D;
const AUXILLIARY_COLOUR_AND_VOLUME_OFFSET: usize = 0x0E;
const SCREEN_CONTROL_OFFSET: usize = 0x0F;

struct RenderContext<'a> {
    screen_ram: &'a [u8],
    colour_ram: &'a [u8],
    char_rom: &'a [u8],
    background_colour: u8,
    columns: usize,
}

pub struct VIC {
    horizontal_origin: u8,
    vertical_origin: u8,
    columns_and_screen_select: u8,
    rows_and_double_height: u8,
    raster_line: u8,
    screen_and_char_base: u8,
    light_pen_horizontal: u8,
    light_pen_vertical: u8,
    paddle_x: u8,
    paddle_y: u8,
    frequency_1: u8,
    frequency_2: u8,
    frequency_3: u8,
    noise_and_control: u8,
    auxiliary_colour_and_volume: u8,
    screen_control: u8,
    screen_dirty: bool,
}

impl Default for VIC {
    fn default() -> Self {
        Self {
            horizontal_origin: 0,
            vertical_origin: 0,
            columns_and_screen_select: 0,
            rows_and_double_height: 0x1E,
            raster_line: 0,
            screen_and_char_base: 0x80,
            light_pen_horizontal: 0,
            light_pen_vertical: 0,
            paddle_x: 0,
            paddle_y: 0,
            frequency_1: 0,
            frequency_2: 0,
            frequency_3: 0,
            noise_and_control: 0,
            auxiliary_colour_and_volume: 0,
            screen_control: 0x0E,
            screen_dirty: true,
        }
    }
}

impl VIC {
    pub fn mark_screen_dirty(&mut self) {
        self.screen_dirty = true;
    }

    pub fn is_address_in_screen_memory(&self, address: u16) -> bool {
        let start = self.screen_ram_start() as usize;
        let addr = address as usize;
        addr >= start && addr < start + SCREEN_RAM_SIZE
    }

    pub fn is_address_in_colour_memory(&self, address: u16) -> bool {
        let start = self.colour_ram_start() as usize;
        let addr = address as usize;
        addr >= start && addr < start + SCREEN_RAM_SIZE
    }

    pub fn columns(&self) -> usize {
        (self.columns_and_screen_select & 0x7F) as usize
    }

    pub fn render_active_screen(&mut self, memory: &[u8; 65536], frame_buffer: &mut [u8]) {
        if !self.screen_dirty {
            return;
        }
        self.screen_dirty = false;
        let columns = self.columns();
        if columns == 0 {
            return;
        }
        let active_width = columns * CHAR_WIDTH;
        let screen_ram_start = self.screen_ram_start() as usize;
        let screen_ram = &memory[screen_ram_start..screen_ram_start + SCREEN_RAM_SIZE];
        let colour_ram_start = self.colour_ram_start() as usize;
        let colour_ram = &memory[colour_ram_start..colour_ram_start + SCREEN_RAM_SIZE];
        let charset_base = self.charset_base() as usize;
        let char_rom = &memory[charset_base..charset_base + CHARSET_SIZE];
        let background_colour = self.background_colour();
        let ctx = RenderContext {
            screen_ram,
            colour_ram,
            char_rom,
            background_colour,
            columns,
        };
        let mut frame_buffer_index = 0;
        for active_y in 0..ACTIVE_HEIGHT {
            for active_x in 0..active_width {
                let colour_index = self.colour_index(&ctx, active_y, active_x);
                let colour = palette(colour_index);
                frame_buffer[frame_buffer_index..frame_buffer_index + 4].copy_from_slice(&colour);
                frame_buffer_index += 4;
            }
        }
    }

    fn colour_index(&self, ctx: &RenderContext, active_y: usize, active_x: usize) -> u8 {
        let row = active_y / CHAR_HEIGHT;
        let col = active_x / CHAR_WIDTH;
        let idx = (row * ctx.columns + col).min(SCREEN_RAM_SIZE - 1);
        let char_code = ctx.screen_ram[idx];
        let fg_color = ctx.colour_ram[idx] & 0x0F;
        let bitmap_row = &ctx.char_rom[char_code as usize * CHAR_HEIGHT..(char_code as usize + 1) * CHAR_HEIGHT]
            [active_y % CHAR_HEIGHT];
        let bit = (bitmap_row >> (7 - (active_x % CHAR_WIDTH))) & 1;
        if bit == 1 { fg_color } else { ctx.background_colour }
    }

    pub fn border_rgba(&self) -> [u8; 4] {
        let border_color = self.screen_control & 0x07;
        palette(border_color)
    }

    pub fn screen_ram_start(&self) -> u16 {
        let m_36866 = self.columns_and_screen_select as u16;
        let m_36869 = self.screen_and_char_base as u16;
        4 * (m_36866 & 0x80) + 64 * (m_36869 & 0x70)
    }

    fn colour_ram_start(&self) -> u16 {
        let m_36866 = self.columns_and_screen_select as u16;
        0x9400 + 4 * (m_36866 & 0x80)
    }

    fn background_colour(&self) -> u8 {
        (self.screen_control & 0xF0) >> 4
    }

    fn charset_base(&self) -> u16 {
        let lower_bits = (self.screen_and_char_base & 0x0F) as u16;
        let base = if lower_bits < 8 { 0x8000 } else { 0x0000 };
        base + 0x0400 * (lower_bits & 0x07)
    }
}

impl Addressable for VIC {
    fn read_byte(&self, address: u16) -> u8 {
        let offset = address as usize;
        match offset {
            HORIZONTAL_ORIGIN_OFFSET => self.horizontal_origin,
            VERTICAL_ORIGIN_OFFSET => self.vertical_origin,
            COLUMNS_AND_SCREEN_SELECT_OFFSET => self.columns_and_screen_select,
            ROWS_AND_DOUBLE_HEIGHT_OFFSET => self.rows_and_double_height,
            RASTER_LINE_OFFSET => self.raster_line,
            SCREEN_AND_CHAR_BASE_OFFSET => self.screen_and_char_base,
            LIGHT_PEN_HORIZONTAL_OFFSET => self.light_pen_horizontal,
            LIGHT_PEN_VERTICAL_OFFSET => self.light_pen_vertical,
            PADDLE_X_OFFSET => self.paddle_x,
            PADDLE_Y_OFFSET => self.paddle_y,
            FREQUENCY_1_OFFSET => self.frequency_1,
            FREQUENCY_2_OFFSET => self.frequency_2,
            FREQUENCY_3_OFFSET => self.frequency_3,
            NOISE_AND_CONTROL_OFFSET => self.noise_and_control,
            AUXILLIARY_COLOUR_AND_VOLUME_OFFSET => self.auxiliary_colour_and_volume,
            SCREEN_CONTROL_OFFSET => self.screen_control,
            _ => 0,
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.screen_dirty = true;
        let offset = address as usize;
        match offset {
            HORIZONTAL_ORIGIN_OFFSET => self.horizontal_origin = value,
            VERTICAL_ORIGIN_OFFSET => self.vertical_origin = value,
            COLUMNS_AND_SCREEN_SELECT_OFFSET => self.columns_and_screen_select = value,
            ROWS_AND_DOUBLE_HEIGHT_OFFSET => self.rows_and_double_height = value,
            RASTER_LINE_OFFSET => self.raster_line = value,
            SCREEN_AND_CHAR_BASE_OFFSET => self.screen_and_char_base = value,
            LIGHT_PEN_HORIZONTAL_OFFSET => self.light_pen_horizontal = value,
            LIGHT_PEN_VERTICAL_OFFSET => self.light_pen_vertical = value,
            PADDLE_X_OFFSET => self.paddle_x = value,
            PADDLE_Y_OFFSET => self.paddle_y = value,
            FREQUENCY_1_OFFSET => self.frequency_1 = value,
            FREQUENCY_2_OFFSET => self.frequency_2 = value,
            FREQUENCY_3_OFFSET => self.frequency_3 = value,
            NOISE_AND_CONTROL_OFFSET => self.noise_and_control = value,
            AUXILLIARY_COLOUR_AND_VOLUME_OFFSET => self.auxiliary_colour_and_volume = value,
            SCREEN_CONTROL_OFFSET => self.screen_control = value,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::CHARACTER_ROM_START;
    use rstest::{fixture, rstest};

    const SCREEN_COLOR: u8 = 2;
    const BACKGROUND_COLOR: u8 = 4;

    #[fixture]
    fn vic() -> VIC {
        let mut vic = VIC::default();
        vic.screen_control = (BACKGROUND_COLOR << 4) | SCREEN_COLOR;
        vic.columns_and_screen_select = 22;
        vic
    }

    fn build_memory(char_code: u8, fg_color: u8) -> [u8; 65536] {
        let mut mem = [0u8; 65536];

        let screen_start = 0;
        let colour_start = 0x9400;

        mem[screen_start] = char_code;
        mem[colour_start] = fg_color;

        let char_offset = CHARACTER_ROM_START + char_code as usize * CHAR_HEIGHT;
        for row in 0..CHAR_HEIGHT {
            mem[char_offset + row] = 0xFF;
        }

        mem
    }

    fn pixel_at(framebuffer: &[u8], x: usize, y: usize) -> [u8; 4] {
        let active_width = 22 * CHAR_WIDTH;
        let idx = (y * active_width + x) * 4;
        framebuffer[idx..idx + 4].try_into().unwrap()
    }

    #[rstest]
    fn reverse_mode_off_char_without_bit7_uses_fg(mut vic: VIC) {
        let mem = build_memory(0x01, SCREEN_COLOR);
        let columns = 22;
        let mut fb = vec![0_u8; ACTIVE_HEIGHT * columns * CHAR_WIDTH * 4];
        vic.render_active_screen(&mem, &mut fb);

        assert_eq!(pixel_at(&fb, 0, 0), palette(SCREEN_COLOR));
    }

    #[rstest]
    #[case(HORIZONTAL_ORIGIN_OFFSET, 0x00)]
    #[case(VERTICAL_ORIGIN_OFFSET, 0x00)]
    #[case(COLUMNS_AND_SCREEN_SELECT_OFFSET, 0x00)]
    #[case(ROWS_AND_DOUBLE_HEIGHT_OFFSET, 0x1E)]
    #[case(RASTER_LINE_OFFSET, 0x00)]
    #[case(SCREEN_AND_CHAR_BASE_OFFSET, 0x80)]
    #[case(LIGHT_PEN_HORIZONTAL_OFFSET, 0x00)]
    #[case(LIGHT_PEN_VERTICAL_OFFSET, 0x00)]
    #[case(PADDLE_X_OFFSET, 0x00)]
    #[case(PADDLE_Y_OFFSET, 0x00)]
    #[case(FREQUENCY_1_OFFSET, 0x00)]
    #[case(FREQUENCY_2_OFFSET, 0x00)]
    #[case(FREQUENCY_3_OFFSET, 0x00)]
    #[case(NOISE_AND_CONTROL_OFFSET, 0x00)]
    #[case(AUXILLIARY_COLOUR_AND_VOLUME_OFFSET, 0x00)]
    #[case(SCREEN_CONTROL_OFFSET, 0x0E)]
    fn vic_register_reset_value(#[case] offset: usize, #[case] expected: u8) {
        let vic = VIC::default();
        let address = offset as u16;
        assert_eq!(vic.read_byte(address), expected);
    }

    #[rstest]
    #[case(HORIZONTAL_ORIGIN_OFFSET)]
    #[case(VERTICAL_ORIGIN_OFFSET)]
    #[case(COLUMNS_AND_SCREEN_SELECT_OFFSET)]
    #[case(ROWS_AND_DOUBLE_HEIGHT_OFFSET)]
    #[case(RASTER_LINE_OFFSET)]
    #[case(SCREEN_AND_CHAR_BASE_OFFSET)]
    #[case(LIGHT_PEN_HORIZONTAL_OFFSET)]
    #[case(LIGHT_PEN_VERTICAL_OFFSET)]
    #[case(PADDLE_X_OFFSET)]
    #[case(PADDLE_Y_OFFSET)]
    #[case(FREQUENCY_1_OFFSET)]
    #[case(FREQUENCY_2_OFFSET)]
    #[case(FREQUENCY_3_OFFSET)]
    #[case(NOISE_AND_CONTROL_OFFSET)]
    #[case(AUXILLIARY_COLOUR_AND_VOLUME_OFFSET)]
    #[case(SCREEN_CONTROL_OFFSET)]
    fn vic_read_returns_last_written_value(mut vic: VIC, #[case] offset: usize) {
        let address = offset as u16;
        let value = 50;
        vic.write_byte(address, value);
        assert_eq!(vic.read_byte(address), value);
    }

    #[rstest]
    #[case(0x00, 0x8000)]
    #[case(0x01, 0x8400)]
    #[case(0x02, 0x8800)]
    #[case(0x03, 0x8C00)]
    #[case(0x04, 0x9000)]
    #[case(0x05, 0x9400)]
    #[case(0x06, 0x9800)]
    #[case(0x07, 0x9C00)]
    #[case(0x08, 0x0000)]
    #[case(0x09, 0x0400)]
    #[case(0x0A, 0x0800)]
    #[case(0x0B, 0x0C00)]
    #[case(0x0C, 0x1000)]
    #[case(0x0D, 0x1400)]
    #[case(0x0E, 0x1800)]
    #[case(0x0F, 0x1C00)]
    fn charset_base(mut vic: VIC, #[case] lower_nibble: u8, #[case] expected: u16) {
        vic.write_byte(SCREEN_AND_CHAR_BASE_OFFSET as u16, lower_nibble);
        assert_eq!(vic.charset_base(), expected);
    }
}
