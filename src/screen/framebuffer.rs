use crate::screen::renderer::{TEXT_COLUMNS, TEXT_ROWS};

pub type Glyph = [u8; 8];
pub const SCREEN_GLYPH_COUNT: usize = TEXT_COLUMNS * TEXT_ROWS;

/// VIC-20 palette (approximate RGB values)
const VIC20_PALETTE: [(u8, u8, u8); 16] = [
    (0x00, 0x00, 0x00), // 0: Black
    (0xFF, 0xFF, 0xFF), // 1: White
    (0xA0, 0x00, 0x00), // 2: Red
    (0x00, 0xA0, 0x00), // 3: Cyan (actually greenish)
    (0x00, 0x00, 0xA0), // 4: Blue
    (0xA0, 0xA0, 0x00), // 5: Yellow
    (0xA0, 0x00, 0xA0), // 6: Magenta
    (0x00, 0xA0, 0xA0), // 7: Green
    (0x50, 0x50, 0x50), // 8: Dark gray
    (0xA0, 0xA0, 0xA0), // 9: Medium gray
    (0xFF, 0x50, 0x50), // 10: Light red
    (0x50, 0xFF, 0x50), // 11: Light green
    (0x50, 0x50, 0xFF), // 12: Light blue
    (0xFF, 0xFF, 0x50), // 13: Light yellow
    (0xFF, 0x50, 0xFF), // 14: Light magenta
    (0x50, 0xFF, 0xFF), // 15: Light cyan
];

pub struct Framebuffer {
    width: usize,
    pixels: Vec<u32>,
}

pub trait Blittable {
    fn blit_screen(&mut self, glyphs: &[Glyph; SCREEN_GLYPH_COUNT], colors: &[u8; SCREEN_GLYPH_COUNT], bg_color: u8);
}

impl Blittable for Framebuffer {
    fn blit_screen(&mut self, glyphs: &[Glyph; SCREEN_GLYPH_COUNT], colors: &[u8; SCREEN_GLYPH_COUNT], bg_color: u8) {
        for (index, (glyph, color)) in glyphs.iter().zip(colors.iter()).enumerate() {
            let x = index % TEXT_COLUMNS;
            let y = index / TEXT_COLUMNS;
            self.blit_glyph(x, y, glyph, *color, bg_color);
        }
    }
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            pixels: vec![0; width * height],
        }
    }

    fn blit_glyph(&mut self, x: usize, y: usize, glyph: &[u8; 8], color: u8, bg_color: u8) {
        let pixel_x = x * 8;
        let pixel_y = y * 8;
        let fg_palette_color = VIC20_PALETTE[color as usize];
        let bg_palette_color = VIC20_PALETTE[bg_color as usize];

        for (row, byte) in glyph.iter().enumerate() {
            for col in 0..8 {
                let bit = (byte >> (7 - col)) & 1;
                let (r, g, b) = if bit == 1 { fg_palette_color } else { bg_palette_color };
                let px = (pixel_y + row) * self.width + (pixel_x + col);
                self.pixels[px] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(color_index: u8) -> u32 {
        let (r, g, b) = VIC20_PALETTE[color_index as usize];
        0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    #[test]
    fn blit_glyph_uses_glyph_coordinates() {
        let mut framebuffer = Framebuffer::new(32, 32);
        let glyph: Glyph = [
            0b1000_0001,
            0b0100_0010,
            0b0010_0100,
            0b0001_1000,
            0b0000_0000,
            0b1111_1111,
            0b0000_0000,
            0b0000_0000,
        ];

        framebuffer.blit_glyph(1, 1, &glyph, 2, 1);

        assert_eq!(framebuffer.pixels[(8 * 32) + 8], rgba(2));
        assert_eq!(framebuffer.pixels[(8 * 32) + 9], rgba(1));
        assert_eq!(framebuffer.pixels[(8 * 32) + 15], rgba(2));
        assert_eq!(framebuffer.pixels[(9 * 32) + 9], rgba(2));
        assert_eq!(framebuffer.pixels[(9 * 32) + 14], rgba(2));
        assert_eq!(framebuffer.pixels[(10 * 32) + 10], rgba(2));
        assert_eq!(framebuffer.pixels[(10 * 32) + 13], rgba(2));
        assert_eq!(framebuffer.pixels[(11 * 32) + 11], rgba(2));
        assert_eq!(framebuffer.pixels[(11 * 32) + 12], rgba(2));

        for pixel_x in 8..16 {
            assert_eq!(framebuffer.pixels[(13 * 32) + pixel_x], rgba(2));
        }

        assert_eq!(framebuffer.pixels[0], 0);
        assert_eq!(framebuffer.pixels[(7 * 32) + 8], 0);
        assert_eq!(framebuffer.pixels[(8 * 32) + 7], 0);
        assert_eq!(framebuffer.pixels[31 * 32 + 31], 0);
    }

    #[test]
    fn blit_screen_renders_all_glyph_cells_with_shared_background() {
        let mut framebuffer = Framebuffer::new(TEXT_COLUMNS * 8, TEXT_ROWS * 8);
        let mut glyphs = [[0; 8]; SCREEN_GLYPH_COUNT];
        let mut colors = [0; SCREEN_GLYPH_COUNT];

        glyphs[0] = [0b1000_0000, 0, 0, 0, 0, 0, 0, 0];
        colors[0] = 2;

        let bottom_right_index = SCREEN_GLYPH_COUNT - 1;
        glyphs[bottom_right_index] = [0, 0, 0, 0, 0, 0, 0, 0b0000_0001];
        colors[bottom_right_index] = 4;

        framebuffer.blit_screen(&glyphs, &colors, 1);

        assert_eq!(framebuffer.pixels[0], rgba(2));
        assert_eq!(framebuffer.pixels[1], rgba(1));

        let last_glyph_origin_x = (TEXT_COLUMNS - 1) * 8;
        let last_glyph_origin_y = (TEXT_ROWS - 1) * 8;
        let last_pixel_index = (last_glyph_origin_y + 7) * (TEXT_COLUMNS * 8) + (last_glyph_origin_x + 7);
        let before_last_pixel_index = last_pixel_index - 1;

        assert_eq!(framebuffer.pixels[last_pixel_index], rgba(4));
        assert_eq!(framebuffer.pixels[before_last_pixel_index], rgba(1));

        let middle_index = (10 * 8) * (TEXT_COLUMNS * 8) + (10 * 8);
        assert_eq!(framebuffer.pixels[middle_index], rgba(1));
    }
}
