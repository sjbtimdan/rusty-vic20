use crate::{
    addressable::Addressable, bus::VIA2_REGISTERS_START, cpu::interrupt_handler::InterruptHandler,
    cpu::registers::Registers,
};
use std::cell::Cell;

const IFR_TIMER1: u8 = 0x40;
#[allow(dead_code)]
const IFR_TIMER2: u8 = 0x20;
#[allow(dead_code)]
const IFR_CB1: u8 = 0x10;
#[allow(dead_code)]
const IFR_CB2: u8 = 0x08;
#[allow(dead_code)]
const IFR_SR: u8 = 0x04;
#[allow(dead_code)]
const IFR_CA1: u8 = 0x02;
#[allow(dead_code)]
const IFR_CA2: u8 = 0x01;
const IFR_IRQ: u8 = 0x80;

pub struct VIA2 {
    registers: [Cell<u8>; 16],
    t1_counter: Cell<u16>,
    t1_latch: Cell<u16>,
}

impl Default for VIA2 {
    fn default() -> Self {
        Self {
            registers: std::array::from_fn(|_| Cell::new(0)),
            t1_counter: Cell::new(0xFFFF),
            t1_latch: Cell::new(0xFFFF),
        }
    }
}

impl VIA2 {
    pub fn step(
        &mut self,
        registers: &mut Registers,
        memory: &mut dyn Addressable,
        interrupt_handler: &mut dyn InterruptHandler,
    ) {
        self.step_timer1();

        self.update_ifr_irq();

        if self.ifr_byte() & IFR_IRQ != 0 {
            interrupt_handler.handle_interrupt(registers, memory, false);
        }
    }

    fn step_timer1(&self) {
        let counter = self.t1_counter.get();
        if counter == 0 {
            self.t1_counter.set(self.t1_latch.get());
            self.registers[13].set(self.registers[13].get() | IFR_TIMER1);
        } else {
            self.t1_counter.set(counter - 1);
        }
    }

    fn update_ifr_irq(&self) {
        let ifr = self.registers[13].get();
        let ier = self.registers[14].get();
        let active = (ifr & 0x7F) & (ier & 0x7F);
        if active != 0 {
            self.registers[13].set(ifr | IFR_IRQ);
        } else {
            self.registers[13].set(ifr & !IFR_IRQ);
        }
    }

    fn ifr_byte(&self) -> u8 {
        self.registers[13].get()
    }
}

impl Addressable for VIA2 {
    fn read_byte(&self, address: u16) -> u8 {
        let offset = address as usize - VIA2_REGISTERS_START as usize;
        match offset {
            4 => {
                let ifr = self.registers[13].get();
                self.registers[13].set(ifr & !IFR_TIMER1);
                self.update_ifr_irq();
                (self.t1_counter.get() & 0xFF) as u8
            }
            5 => (self.t1_counter.get() >> 8) as u8,
            6 => (self.t1_latch.get() & 0xFF) as u8,
            7 => (self.t1_latch.get() >> 8) as u8,
            13 => {
                let ifr = self.registers[13].get();
                let ier = self.registers[14].get();
                let active = (ifr & 0x7F) & (ier & 0x7F);
                if active != 0 { ifr | IFR_IRQ } else { ifr & !IFR_IRQ }
            }
            _ => self.registers[offset].get(),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        let offset = address as usize - VIA2_REGISTERS_START as usize;
        match offset {
            4 => {
                self.t1_latch.set((self.t1_latch.get() & 0xFF00) | value as u16);
            }
            5 | 7 => {
                if offset == 5 {
                    self.t1_latch
                        .set((self.t1_latch.get() & 0x00FF) | ((value as u16) << 8));
                    self.t1_counter.set(self.t1_latch.get());
                    let ifr = self.registers[13].get();
                    self.registers[13].set(ifr & !IFR_TIMER1);
                    self.update_ifr_irq();
                } else {
                    self.t1_latch
                        .set((self.t1_latch.get() & 0x00FF) | ((value as u16) << 8));
                }
            }
            6 => {
                self.t1_latch.set((self.t1_latch.get() & 0xFF00) | value as u16);
            }
            13 => {
                let ifr = self.registers[13].get();
                if value & IFR_IRQ != 0 {
                    self.registers[13].set(ifr | (value & 0x7F));
                } else {
                    self.registers[13].set(ifr & !(value & 0x7F));
                }
                self.update_ifr_irq();
            }
            14 => {
                let ier = self.registers[14].get();
                if value & IFR_IRQ != 0 {
                    self.registers[14].set(ier | (value & 0x7F));
                } else {
                    self.registers[14].set(ier & !(value & 0x7F));
                }
            }
            _ => {
                self.registers[offset].set(value);
            }
        }
    }
}
