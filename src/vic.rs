use crate::{
    addressable::Addressable,
    bus::{CHARACTER_ROM_END, CHARACTER_ROM_START, COLOUR_RAM_END, COLOUR_RAM_START, SCREEN_RAM_END, SCREEN_RAM_START},
    screen::renderer::{
        ACTIVE_HEIGHT, ACTIVE_WIDTH, BORDER_LEFT, BORDER_TOP, CHAR_HEIGHT, CHAR_WIDTH, PAL_HEIGHT, PAL_WIDTH,
        TEXT_COLUMNS,
    },
};

#[derive(Default)]
pub struct VIC {
    registers: [u8; 16],
    cycle_count: u64,
}

impl VIC {
    pub fn step(&mut self, _memory: &[u8; 65536]) {
        self.cycle_count += 1;
        // if self.cycle_count.is_multiple_of(10_000) {
        //     let screen_ram = &memory[SCREEN_RAM_START as usize..SCREEN_RAM_END as usize];
        //     for (i, (&old_byte, &new_byte)) in self.screen_ram.iter().zip(screen_ram.iter()).enumerate() {
        //         if old_byte != new_byte {
        //             println!(
        //                 "Screen RAM changed at offset {:04X}: {:02X} -> {:02X}",
        //                 i, old_byte, new_byte
        //             );
        //         }
        //     }
        //     self.screen_ram.clone_from_slice(screen_ram);
        // }
    }

    pub fn render_frame(&self, memory: &[u8; 65536]) -> Vec<u32> {
        let screen_ram = &memory[SCREEN_RAM_START as usize..SCREEN_RAM_END as usize];
        let color_ram = &memory[COLOUR_RAM_START..=COLOUR_RAM_END];
        let char_rom = &memory[CHARACTER_ROM_START..=CHARACTER_ROM_END];
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
                    let active_x = x - BORDER_LEFT;
                    let active_y = y - BORDER_TOP;
                    let row = active_y / CHAR_HEIGHT;
                    let col = active_x / CHAR_WIDTH;
                    let idx = row * TEXT_COLUMNS + col;
                    let char_code = screen_ram[idx];
                    let fg_color = color_ram[idx] & 0x0F;
                    let bitmap_row = &char_rom
                        [char_code as usize * CHAR_HEIGHT..(char_code as usize + 1) * CHAR_HEIGHT]
                        [active_y % CHAR_HEIGHT];
                    let bit = (bitmap_row >> (7 - (active_x % CHAR_WIDTH))) & 1;
                    if bit == 1 { fg_color } else { background_color }
                };
                framebuffer.push(self.palette(colour_index));
            }
        }
        framebuffer
    }

    fn is_border(&self, x: usize, y: usize) -> bool {
        x < BORDER_LEFT || y < BORDER_TOP || x >= BORDER_LEFT + ACTIVE_WIDTH || y >= BORDER_TOP + ACTIVE_HEIGHT
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
