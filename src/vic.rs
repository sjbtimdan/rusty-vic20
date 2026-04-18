use crate::{
    addressable::Addressable,
    bus::*,
    screen::{PAL_HEIGHT, PAL_WIDTH},
};

pub struct VIC {
    registers: [u8; 16],
    memory: Box<[u8; 65536]>,
}

impl VIC {
    pub fn new(memory: Box<[u8; 65536]>) -> Self {
        Self {
            registers: [0; 16],
            memory,
        }
    }

    pub fn border_colour(&self) -> u8 {
        self.registers[0xF] & 0x0F
    }

    pub fn set_border_color(&mut self, value: u8) {
        self.registers[0x0F] = value & 0x0F;
    }

    pub fn render_frame(&self) -> Vec<u32> {
        let screen_ram = &self.memory[SCREEN_RAM_START as usize..SCREEN_RAM_END as usize];
        let color_ram = &self.memory[COLOUR_RAM_START..=COLOUR_RAM_END];
        let char_rom = &self.memory[CHARACTER_ROM_START..=CHARACTER_ROM_END];
        let width = PAL_WIDTH;
        let height = PAL_HEIGHT;
        let mut framebuffer = Vec::with_capacity(width * height);
        let border_color = self.registers[0x0F] & 0x0F;
        let background_color = self.registers[0x0E] & 0x0F;
        for y in 0..height {
            for x in 0..width {
                let colour_index = if self.is_border(x, y) {
                    border_color
                } else {
                    let row = y / 8;
                    let col = x / 8;
                    let idx = row * 22 + col;
                    let char_code = screen_ram[idx];
                    let fg_color = color_ram[idx] & 0x0F;
                    let bitmap_row = &char_rom[char_code as usize * 8..(char_code as usize + 1) * 8][y % 8];
                    let bit = (bitmap_row >> (7 - (x % 8))) & 1;
                    if bit == 1 { fg_color } else { background_color }
                };
                framebuffer.push(self.palette(colour_index));
            }
        }
        framebuffer
    }

    fn is_border(&self, x: usize, y: usize) -> bool {
        !(16..160).contains(&x) || !(16..168).contains(&y)
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

impl Addressable for VIC {
    fn read_byte(&self, address: u16) -> u8 {
        self.registers[address as usize - 0x9000]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.registers[address as usize - 0x9000] = value;
    }
}
