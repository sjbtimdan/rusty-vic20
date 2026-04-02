#[derive(Default)]
pub struct VIC {
    registers: [u8; 16],
}

impl VIC {
    pub fn border_colour(&self) -> u8 {
        self.registers[0xF] & 0x0F
    }

    pub fn set_border_color(&mut self, value: u8) {
        self.registers[0x0F] = value & 0x0F;
    }

    pub fn render_frame(
        &self,
        screen_ram: &[u8],         // 22x23 = 506 bytes
        color_ram: &[u8],          // 22x23 = 506 bytes
        char_rom: &[[u8; 8]; 256], // 256 chars, 8 rows each
    ) -> Vec<u32> {
        let width = 176;
        let height = 184;
        let mut framebuffer = Vec::with_capacity(width * height);
        let border_color = self.registers[0x0F] & 0x0F;
        let background_color = self.registers[0x0E] & 0x0F;
        for y in 0..height {
            for x in 0..width {
                if self.is_border(x, y) {
                    framebuffer.push(self.palette(border_color));
                } else {
                    let row = y / 8;
                    let col = x / 8;
                    let idx = row * 22 + col;
                    let char_code = screen_ram[idx];
                    let fg_color = color_ram[idx] & 0x0F;
                    let bitmap_row = char_rom[char_code as usize][y % 8];
                    let bit = (bitmap_row >> (7 - (x % 8))) & 1;
                    let color = if bit == 1 { fg_color } else { background_color };
                    framebuffer.push(self.palette(color));
                }
            }
        }
        framebuffer
    }

    fn is_border(&self, x: usize, y: usize) -> bool {
        x < 16 || x >= 160 || y < 16 || y >= 168
    }

    fn palette(&self, index: u8) -> u32 {
        match index {
            0 => 0x000000FF,  // black
            1 => 0xFFFFFFFF,  // white
            2 => 0x880000FF,  // red
            3 => 0x00FFFFFF,  // cyan
            4 => 0xAA00AAFF,  // purple
            5 => 0x00AA00FF,  // green
            6 => 0x0000AAFF,  // blue
            7 => 0xAAAA00FF,  // yellow
            8 => 0xFF8800FF,  // orange
            9 => 0x884400FF,  // brown
            10 => 0xFFAAAAFF, // light red
            11 => 0x444444FF, // dark gray
            12 => 0x888888FF, // medium gray
            13 => 0xAAFFAAFF, // light green
            14 => 0xAAAAFFFF, // light blue
            15 => 0xCCCCCCFF, // light gray
            _ => 0x000000FF,
        }
    }
}
