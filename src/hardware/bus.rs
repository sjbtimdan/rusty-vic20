use crate::{
    hardware::{addressable::Addressable, memory::Memory, via::VIA, vic::VIC},
    tools::debug::MemoryWriteWatchpoint,
    ui::screen::renderer::{ACTIVE_HEIGHT, CHAR_WIDTH},
};
use nmos6502::CPU6502;

#[derive(Default)]
pub struct Bus {
    pub memory: Memory,
    pub vic: VIC,
    pub via1: VIA,
    pub via2: VIA,
    watchpoints: Vec<MemoryWriteWatchpoint>,
    frame_buffer: Vec<u8>,
}

pub const SCREEN_RAM_SIZE: usize = 512;
pub const CHARSET_SIZE: usize = 0x0FFF;
pub const VIC_REGISTERS_START: u16 = 0x9000;
pub const VIC_REGISTERS_END: u16 = 0x9010;
pub const VIA1_REGISTERS_START: u16 = 0x9110;
pub const VIA1_REGISTERS_END: u16 = 0x9120;
pub const VIA2_REGISTERS_START: u16 = 0x9120;
pub const VIA2_REGISTERS_END: u16 = 0x9130;

impl Addressable for Bus {
    fn read_byte(&self, address: u16) -> u8 {
        if !(VIC_REGISTERS_START..VIA2_REGISTERS_END).contains(&address) {
            return self.memory.read_byte(address);
        }
        match address {
            VIC_REGISTERS_START..VIC_REGISTERS_END => self.vic.read_byte(address - VIC_REGISTERS_START),
            VIA1_REGISTERS_START..VIA1_REGISTERS_END => self.via1.read_byte(address - VIA1_REGISTERS_START),
            VIA2_REGISTERS_START..VIA2_REGISTERS_END => self.via2.read_byte(address - VIA2_REGISTERS_START),
            _ => self.memory.read_byte(address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.watchpoints
            .iter()
            .for_each(|watchpoint| watchpoint.on_write(address, value));
        if !(VIC_REGISTERS_START..VIA2_REGISTERS_END).contains(&address) {
            self.memory.write_byte(address, value);
            if self.vic.is_address_in_screen_memory(address) || self.vic.is_address_in_colour_memory(address) {
                self.vic.mark_screen_dirty();
            }
            return;
        }
        match address {
            VIC_REGISTERS_START..VIC_REGISTERS_END => self.vic.write_byte(address - VIC_REGISTERS_START, value),
            VIA1_REGISTERS_START..VIA1_REGISTERS_END => self.via1.write_byte(address - VIA1_REGISTERS_START, value),
            VIA2_REGISTERS_START..VIA2_REGISTERS_END => self.via2.write_byte(address - VIA2_REGISTERS_START, value),
            _ => {
                self.memory.write_byte(address, value);
                if self.vic.is_address_in_screen_memory(address) || self.vic.is_address_in_colour_memory(address) {
                    self.vic.mark_screen_dirty();
                }
            }
        }
    }
}

impl nmos6502::Addressable for Bus {
    fn read_byte(&mut self, address: u16) -> u8 {
        Addressable::read_byte(self, address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        Addressable::write_byte(self, address, value);
    }
}

impl Bus {
    pub fn add_watchpoint(&mut self, watchpoint: MemoryWriteWatchpoint) {
        self.watchpoints.push(watchpoint);
    }

    pub fn step_devices(&mut self, cpu: &mut CPU6502) {
        self.via1.step_internal();
        cpu.nmi_latch.set_level(self.via1.ca1_pin_high());
        self.via2.step_internal();
        cpu.irq_line_low = self.via2.irq_active();
    }

    pub fn render_active_screen(&mut self) {
        let columns = self.vic.columns();
        if columns == 0 {
            return;
        }
        let active_width = columns * CHAR_WIDTH;
        let required_len = ACTIVE_HEIGHT * active_width * 4;
        if self.frame_buffer.len() != required_len {
            self.frame_buffer.resize(required_len, 0);
        }
        self.vic
            .render_active_screen(self.memory.as_bytes(), &mut self.frame_buffer)
    }

    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    pub fn screen_ram_start(&self) -> u16 {
        self.vic.screen_ram_start()
    }

    pub fn border_rgba(&self) -> [u8; 4] {
        self.vic.border_rgba()
    }

    pub fn columns(&self) -> usize {
        self.vic.columns()
    }

    pub fn load_data(&mut self, start_address: usize, data: &[u8]) {
        let len = data.len().min(65536 - start_address);
        self.memory
            .copy_from_slice(start_address, start_address + len, &data[..len]);
        self.vic.mark_screen_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn bus() -> Bus {
        Bus::default()
    }

    #[rstest]
    fn test_write_byte_and_read_zero_page(mut bus: Bus) {
        bus.write_byte(0x42, 0xAB);
        assert_eq!(bus.read_zero_page_byte(0x42), 0xAB);
    }

    #[rstest]
    fn test_read_word_little_endian(mut bus: Bus) {
        bus.write_byte(0x0200, 0x34);
        bus.write_byte(0x0201, 0x12);
        assert_eq!(bus.read_word(0x0200), 0x1234);
    }

    #[rstest]
    fn test_set_word_little_endian(mut bus: Bus) {
        bus.write_word(0x0300, 0xABCD);
        assert_eq!(bus.read_word(0x0300), 0xABCD);
    }
}
