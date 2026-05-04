use crate::{
    addressable::Addressable,
    bus::{CHARACTER_ROM_END, CHARACTER_ROM_START, SCREEN_RAM_SIZE, VIC_REGISTERS_START},
    screen::renderer::{ACTIVE_HEIGHT, ACTIVE_WIDTH, CHAR_HEIGHT, CHAR_WIDTH, TEXT_COLUMNS, palette},
};

const SCREEN_CONTROL_OFFSET: usize = 0x0F;

pub struct VIC {
    registers: [u8; 15],
    screen_control: u8,
    cycle_count: u64,
}

impl Default for VIC {
    fn default() -> Self {
        let mut vic = Self {
            registers: [0; 15],
            screen_control: 0,
            cycle_count: 0,
        };
        vic.registers[0x03] = 0x1E;
        vic.registers[0x05] = 0x80;
        vic.screen_control = 0x0E;
        vic
    }
}

impl VIC {
    pub fn step(&mut self) {
        self.cycle_count += 1;
    }

    pub fn render_active_screen(
        &self,
        memory: &[u8; 65536],
        frame_buffer: &mut [u8; ACTIVE_HEIGHT * ACTIVE_WIDTH * 4],
    ) {
        let screen_ram_start = self.screen_ram_start() as usize;
        let screen_ram = &memory[screen_ram_start..screen_ram_start + SCREEN_RAM_SIZE];
        let colour_ram_start = self.colour_ram_start() as usize;
        let colour_ram = &memory[colour_ram_start..=colour_ram_start + SCREEN_RAM_SIZE];
        let char_rom = &memory[CHARACTER_ROM_START..=CHARACTER_ROM_END];
        let background_colour = self.background_colour();
        let mut frame_buffer_index = 0;
        for active_y in 0..ACTIVE_HEIGHT {
            for active_x in 0..ACTIVE_WIDTH {
                let colour_index =
                    self.colour_index(screen_ram, colour_ram, char_rom, background_colour, active_y, active_x);
                let colour = palette(colour_index);
                frame_buffer[frame_buffer_index..frame_buffer_index + 4].copy_from_slice(&colour);
                frame_buffer_index += 4;
            }
        }
    }

    fn colour_index(
        &self,
        screen_ram: &[u8],
        colour_ram: &[u8],
        char_rom: &[u8],
        background_colour: u8,
        active_y: usize,
        active_x: usize,
    ) -> u8 {
        let row = active_y / CHAR_HEIGHT;
        let col = active_x / CHAR_WIDTH;
        let idx = row * TEXT_COLUMNS + col;
        let char_code = screen_ram[idx];
        let fg_color = colour_ram[idx] & 0x0F;
        let bitmap_row =
            &char_rom[char_code as usize * CHAR_HEIGHT..(char_code as usize + 1) * CHAR_HEIGHT][active_y % CHAR_HEIGHT];
        let bit = (bitmap_row >> (7 - (active_x % CHAR_WIDTH))) & 1;
        if bit == 1 { fg_color } else { background_colour }
    }

    pub fn border_rgba(&self) -> [u8; 4] {
        let border_color = self.screen_control & 0x07;
        palette(border_color)
    }

    fn screen_ram_start(&self) -> u16 {
        // S = 4 * (PEEK (36866) AND 128) + 64 * (PEEK (36869) AND 112)
        let m_36866 = self.registers[0x02] as u16;
        let m_36869 = self.registers[0x05] as u16;
        4 * (m_36866 & 0x80) + 64 * (m_36869 & 0x70)
    }

    fn colour_ram_start(&self) -> u16 {
        // C = 37888 + 4 * (PEEK (36866) AND 128)
        let m_36866 = self.registers[0x02] as u16;
        0x9400 + 4 * (m_36866 & 0x80)
    }

    fn background_colour(&self) -> u8 {
        (self.screen_control & 0xF0) >> 4
    }
}

impl Addressable for VIC {
    fn read_byte(&self, address: u16) -> u8 {
        let offset = address as usize - VIC_REGISTERS_START as usize;
        match offset {
            SCREEN_CONTROL_OFFSET => self.screen_control,
            _ => self.registers[offset],
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        let offset = address as usize - VIC_REGISTERS_START as usize;
        match offset {
            SCREEN_CONTROL_OFFSET => self.screen_control = value,
            _ => self.registers[offset] = value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    const SCREEN_COLOR: u8 = 2;
    const BACKGROUND_COLOR: u8 = 4;

    #[fixture]
    fn vic() -> VIC {
        let mut vic = VIC::default();
        vic.screen_control = (BACKGROUND_COLOR << 4) | SCREEN_COLOR;
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
        let idx = (y * ACTIVE_WIDTH + x) * 4;
        framebuffer[idx..idx + 4].try_into().unwrap()
    }

    #[rstest]
    fn reverse_mode_off_char_without_bit7_uses_fg(vic: VIC) {
        let mem = build_memory(0x01, SCREEN_COLOR);
        let mut fb = [0_u8; ACTIVE_HEIGHT * ACTIVE_WIDTH * 4];
        vic.render_active_screen(&mem, &mut fb);

        assert_eq!(pixel_at(&fb, 0, 0), palette(SCREEN_COLOR));
    }

    #[rstest]
    #[case(0, 0x00)]
    #[case(1, 0x00)]
    #[case(2, 0x00)]
    #[case(3, 0x1E)]
    #[case(4, 0x00)]
    #[case(5, 0x80)]
    #[case(6, 0x00)]
    #[case(7, 0x00)]
    #[case(8, 0x00)]
    #[case(9, 0x00)]
    #[case(10, 0x00)]
    #[case(11, 0x00)]
    #[case(12, 0x00)]
    #[case(13, 0x00)]
    #[case(14, 0x00)]
    #[case(SCREEN_CONTROL_OFFSET, 0x0E)]
    fn vic_register_reset_value(#[case] offset: usize, #[case] expected: u8) {
        let vic = VIC::default();
        let address = VIC_REGISTERS_START + (offset as u16);
        assert_eq!(vic.read_byte(address), expected);
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(2)]
    #[case(3)]
    #[case(4)]
    #[case(5)]
    #[case(6)]
    #[case(7)]
    #[case(8)]
    #[case(9)]
    #[case(10)]
    #[case(11)]
    #[case(12)]
    #[case(13)]
    #[case(14)]
    #[case(SCREEN_CONTROL_OFFSET)]
    fn vic_read_returns_last_written_value(mut vic: VIC, #[case] offset: usize) {
        let address = VIC_REGISTERS_START + (offset as u16);
        let value = 50;
        vic.write_byte(address, value);
        assert_eq!(vic.read_byte(address), value);
    }
}
