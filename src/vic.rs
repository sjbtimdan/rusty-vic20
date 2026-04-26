use crate::{
    addressable::Addressable,
    bus::{CHARACTER_ROM_END, CHARACTER_ROM_START, SCREEN_RAM_SIZE},
    screen::renderer::{ACTIVE_HEIGHT, ACTIVE_WIDTH, CHAR_HEIGHT, CHAR_WIDTH, TEXT_COLUMNS, palette},
};

pub struct VIC {
    registers: [u8; 16],
    cycle_count: u64,
}

impl Default for VIC {
    fn default() -> Self {
        let mut vic = Self {
            registers: [0; 16],
            cycle_count: 0,
        };
        vic.registers[0x03] = 0x1E;
        vic.registers[0x05] = 0x80;
        vic.registers[0x0F] = 0x0E;
        vic
    }
}

impl VIC {
    pub fn step(&mut self) {
        self.cycle_count += 1;
    }

    #[must_use]
    pub fn render_active_screen(&self, memory: &[u8; 65536]) -> Vec<u32> {
        let screen_ram_start = self.screen_ram_start() as usize;
        let screen_ram = &memory[screen_ram_start..screen_ram_start + SCREEN_RAM_SIZE];
        let colour_ram_start = self.colour_ram_start() as usize;
        let colour_ram = &memory[colour_ram_start..=colour_ram_start + SCREEN_RAM_SIZE];
        let char_rom = &memory[CHARACTER_ROM_START..=CHARACTER_ROM_END];
        let background_colour = self.background_colour();
        let mut framebuffer = Vec::with_capacity(ACTIVE_WIDTH * ACTIVE_HEIGHT);

        for active_y in 0..ACTIVE_HEIGHT {
            for active_x in 0..ACTIVE_WIDTH {
                let row = active_y / CHAR_HEIGHT;
                let col = active_x / CHAR_WIDTH;
                let idx = row * TEXT_COLUMNS + col;
                let char_code = screen_ram[idx];
                let fg_color = colour_ram[idx] & 0x0F;
                let bitmap_row = &char_rom[char_code as usize * CHAR_HEIGHT..(char_code as usize + 1) * CHAR_HEIGHT]
                    [active_y % CHAR_HEIGHT];
                let bit = (bitmap_row >> (7 - (active_x % CHAR_WIDTH))) & 1;
                let colour_index = if bit == 1 { fg_color } else { background_colour };

                framebuffer.push(palette(colour_index));
            }
        }
        framebuffer
    }

    pub fn border_rgba(&self) -> u32 {
        let border_color = self.registers[0x0F] & 0x07;
        palette(border_color)
    }

    fn screen_ram_start(&self) -> u16 {
        // S = 4* (PEEK (36866) AND 128) + 64* (PEEK (36869) AND 112)
        let m_36866 = self.registers[0x02] as u16;
        let m_36869 = self.registers[0x05] as u16;
        4 * (m_36866 & 0x80) + 64 * (m_36869 & 0x70)
    }

    fn colour_ram_start(&self) -> u16 {
        // C = 37888 + 4* (PEEK (36866) AND 128)
        let m_36866 = self.registers[0x02] as u16;
        0x9400 + 4 * (m_36866 & 0x80)
    }

    fn background_colour(&self) -> u8 {
        (self.registers[0x0F] & 0xF0) >> 4
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
