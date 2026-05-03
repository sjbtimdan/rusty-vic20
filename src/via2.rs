use crate::{
    addressable::Addressable, bus::VIA2_REGISTERS_START, cpu::interrupt_handler::InterruptHandler,
    cpu::registers::Registers,
};
use std::cell::Cell;

const IFR_TIMER1: u8 = 0x40;
const IFR_IRQ: u8 = 0x80;

// Offsets
const PORT_B_OFFSET: usize = 0x00;
const PORT_A_OFFSET: usize = 0x01;
const DATA_DIRECTION_B_OFFSET: usize = 0x02;
const DATA_DIRECTION_A_OFFSET: usize = 0x03;
const TIMER1_LATCH_LO_OFFSET: usize = 0x04;
const TIMER1_LATCH_HI_OFFSET: usize = 0x05;
const TIMER1_COUNTER_LO_OFFSET: usize = 0x06;
const TIMER1_COUNTER_HI_OFFSET: usize = 0x07;
const TIMER2_COUNTER_LO_OFFSET: usize = 0x08;
const TIMER2_COUNTER_HI_OFFSET: usize = 0x09;
const SHIFT_REGISTER_OFFSET: usize = 0x0A;
const AUXILIARY_CONTROL_OFFSET: usize = 0x0B;
const PERIPHERAL_CONTROL_OFFSET: usize = 0x0C;
const IFR_OFFSET: usize = 0x0D;
const IER_OFFSET: usize = 0x0E;
const PORTA_HANDSHAKE_OFFSET: usize = 0x0F;

pub struct VIA2 {
    pb: u8,
    pa: u8,
    ddrb: u8,
    ddra: u8,
    timer2_counter_lo: u8,
    timer2_counter_hi: u8,
    shift_register: u8,
    auxiliary_control: u8,
    peripheral_control: u8,
    ifr: Cell<u8>,
    ier: u8,
    port_a_handshake: u8,
    t1_counter: Cell<u16>,
    t1_latch: Cell<u16>,
}

impl Default for VIA2 {
    fn default() -> Self {
        Self {
            pb: 0,
            pa: 0,
            ddrb: 0,
            ddra: 0,
            timer2_counter_lo: 0,
            timer2_counter_hi: 0,
            shift_register: 0,
            auxiliary_control: 0,
            peripheral_control: 0,
            ifr: Cell::new(0),
            ier: 0,
            port_a_handshake: 0,
            t1_counter: Cell::new(0x0000),
            t1_latch: Cell::new(0x0000),
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
            self.ifr.set(self.ifr.get() | IFR_TIMER1);
        } else {
            self.t1_counter.set(counter - 1);
        }
    }

    fn update_ifr_irq(&self) {
        let ifr = self.ifr.get();
        let active = (ifr & 0x7F) & (self.ier & 0x7F);
        let next_irq = if active != 0 { ifr | IFR_IRQ } else { ifr & !IFR_IRQ };
        self.ifr.set(next_irq);
    }

    fn ifr_byte(&self) -> u8 {
        self.ifr.get()
    }
}

impl Addressable for VIA2 {
    fn read_byte(&self, address: u16) -> u8 {
        let offset = address as usize - VIA2_REGISTERS_START as usize;
        match offset {
            PORT_A_OFFSET => self.pa,
            PORT_B_OFFSET => self.pb,
            DATA_DIRECTION_A_OFFSET => self.ddra,
            DATA_DIRECTION_B_OFFSET => self.ddrb,
            TIMER1_LATCH_LO_OFFSET => {
                let ifr = self.ifr.get();
                self.ifr.set(ifr & !IFR_TIMER1);
                self.update_ifr_irq();
                (self.t1_counter.get() & 0xFF) as u8
            }
            TIMER1_LATCH_HI_OFFSET => (self.t1_counter.get() >> 8) as u8,
            TIMER1_COUNTER_LO_OFFSET => (self.t1_latch.get() & 0xFF) as u8,
            TIMER1_COUNTER_HI_OFFSET => (self.t1_latch.get() >> 8) as u8,
            TIMER2_COUNTER_LO_OFFSET => self.timer2_counter_lo,
            TIMER2_COUNTER_HI_OFFSET => self.timer2_counter_hi,
            SHIFT_REGISTER_OFFSET => self.shift_register,
            AUXILIARY_CONTROL_OFFSET => self.auxiliary_control,
            PERIPHERAL_CONTROL_OFFSET => self.peripheral_control,
            IFR_OFFSET => {
                let ifr = self.ifr.get();
                let active = (ifr & 0x7F) & (self.ier & 0x7F);
                if active != 0 { ifr | IFR_IRQ } else { ifr & !IFR_IRQ }
            }
            IER_OFFSET => self.ier,
            PORTA_HANDSHAKE_OFFSET => self.port_a_handshake,
            _ => panic!("Invalid VIA2 register read at address {:04X}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        let offset = address as usize - VIA2_REGISTERS_START as usize;
        match offset {
            PORT_A_OFFSET => self.pa = value,
            PORT_B_OFFSET => self.pb = value,
            DATA_DIRECTION_A_OFFSET => self.ddra = value,
            DATA_DIRECTION_B_OFFSET => self.ddrb = value,
            TIMER1_LATCH_LO_OFFSET => {
                self.t1_latch.set((self.t1_latch.get() & 0xFF00) | value as u16);
            }
            TIMER1_LATCH_HI_OFFSET => {
                self.t1_latch
                    .set((self.t1_latch.get() & 0x00FF) | ((value as u16) << 8));
                self.t1_counter.set(self.t1_latch.get());
                let ifr = self.ifr.get();
                self.ifr.set(ifr & !IFR_TIMER1);
                self.update_ifr_irq();
            }
            TIMER1_COUNTER_HI_OFFSET => {
                self.t1_latch
                    .set((self.t1_latch.get() & 0x00FF) | ((value as u16) << 8));
            }
            TIMER1_COUNTER_LO_OFFSET => {
                self.t1_latch.set((self.t1_latch.get() & 0xFF00) | value as u16);
            }
            TIMER2_COUNTER_LO_OFFSET => self.timer2_counter_lo = value,
            TIMER2_COUNTER_HI_OFFSET => self.timer2_counter_hi = value,
            SHIFT_REGISTER_OFFSET => self.shift_register = value,
            AUXILIARY_CONTROL_OFFSET => self.auxiliary_control = value,
            PERIPHERAL_CONTROL_OFFSET => self.peripheral_control = value,
            IFR_OFFSET => {
                let ifr = self.ifr.get();
                if value & IFR_IRQ != 0 {
                    self.ifr.set(ifr | (value & 0x7F));
                } else {
                    self.ifr.set(ifr & !(value & 0x7F));
                }
                self.update_ifr_irq();
            }
            IER_OFFSET => {
                if value & IFR_IRQ != 0 {
                    self.ier |= value & 0x7F;
                } else {
                    self.ier &= !(value & 0x7F);
                }
            }
            PORTA_HANDSHAKE_OFFSET => self.port_a_handshake = value,
            _ => panic!("Invalid VIA2 register write at address {:04X}", address),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        addressable::UnimplementedAddressable,
        cpu::interrupt_handler::{InterruptHandlerMock, NoOpInterruptHandler},
        cpu::registers::Registers,
    };
    use rstest::{fixture, rstest};
    use unimock::{MockFn, Unimock, matching};

    fn addr(offset: usize) -> u16 {
        VIA2_REGISTERS_START + offset as u16
    }

    #[fixture]
    fn via() -> VIA2 {
        VIA2::default()
    }

    #[rstest]
    #[case(PORT_B_OFFSET)]
    #[case(PORT_A_OFFSET)]
    #[case(DATA_DIRECTION_B_OFFSET)]
    #[case(DATA_DIRECTION_A_OFFSET)]
    #[case(TIMER1_COUNTER_LO_OFFSET)]
    #[case(TIMER1_COUNTER_HI_OFFSET)]
    #[case(TIMER2_COUNTER_LO_OFFSET)]
    #[case(TIMER2_COUNTER_HI_OFFSET)]
    #[case(SHIFT_REGISTER_OFFSET)]
    #[case(AUXILIARY_CONTROL_OFFSET)]
    #[case(PERIPHERAL_CONTROL_OFFSET)]
    #[case(IFR_OFFSET)]
    #[case(PORTA_HANDSHAKE_OFFSET)]
    fn read_byte_returns_default_zero(via: VIA2, #[case] offset: usize) {
        assert_eq!(via.read_byte(addr(offset)), 0);
    }

    #[rstest]
    #[case(PORT_B_OFFSET)]
    #[case(PORT_A_OFFSET)]
    #[case(DATA_DIRECTION_B_OFFSET)]
    #[case(DATA_DIRECTION_A_OFFSET)]
    #[case(TIMER1_COUNTER_LO_OFFSET)]
    #[case(TIMER1_COUNTER_HI_OFFSET)]
    #[case(TIMER2_COUNTER_LO_OFFSET)]
    #[case(TIMER2_COUNTER_HI_OFFSET)]
    #[case(SHIFT_REGISTER_OFFSET)]
    #[case(AUXILIARY_CONTROL_OFFSET)]
    #[case(PERIPHERAL_CONTROL_OFFSET)]
    #[case(PORTA_HANDSHAKE_OFFSET)]
    // #[case(IFR_OFFSET)]
    fn write_byte_stores_value_readable_back(mut via: VIA2, #[case] offset: usize) {
        via.write_byte(addr(offset), 0xAB);
        assert_eq!(via.read_byte(addr(offset)), 0xAB);
    }

    #[rstest]
    fn read_byte_timer1_counter_lo_returns_counter_and_clears_ifr_timer1(via: VIA2) {
        via.t1_counter.set(0x1234);
        via.ifr.set(IFR_TIMER1);
        let value = via.read_byte(addr(TIMER1_LATCH_LO_OFFSET));
        assert_eq!(value, 0x34);
        assert_eq!(via.ifr.get() & IFR_TIMER1, 0);
    }

    #[rstest]
    fn write_byte_timer1_latch_hi_sets_counter_and_clears_ifr_timer1(mut via: VIA2) {
        via.t1_latch.set(0x5678);
        via.ifr.set(IFR_TIMER1);
        via.write_byte(addr(TIMER1_LATCH_HI_OFFSET), 0x56);
        assert_eq!(via.t1_counter.get(), 0x5678);
        assert_eq!(via.ifr.get() & IFR_TIMER1, 0);
    }

    /// Each step decrements the running timer1 counter by one cycle.
    #[rstest]
    fn step_decrements_timer1_counter(mut via: VIA2) {
        via.t1_counter.set(5);
        via.t1_latch.set(100);
        via.step(
            &mut Registers::default(),
            &mut UnimplementedAddressable,
            &mut NoOpInterruptHandler,
        );
        assert_eq!(via.t1_counter.get(), 4);
    }

    /// When the counter reaches zero, the next step reloads it from the latch
    /// and sets the timer1 interrupt flag (IFR bit 6).
    #[rstest]
    fn step_reloads_timer1_and_sets_flag_on_underflow(mut via: VIA2) {
        via.t1_counter.set(0);
        via.t1_latch.set(100);
        via.step(
            &mut Registers::default(),
            &mut UnimplementedAddressable,
            &mut NoOpInterruptHandler,
        );
        assert_eq!(via.t1_counter.get(), 100);
        assert_eq!(
            via.ifr.get() & IFR_TIMER1,
            IFR_TIMER1,
            "IFR_TIMER1 should be set after underflow"
        );
    }

    /// When timer1 underflows and IER has the timer1 bit enabled,
    /// the interrupt handler is invoked with `is_break = false`.
    #[rstest]
    fn step_calls_interrupt_handler_when_timer1_irq_enabled(mut via: VIA2) {
        via.t1_counter.set(0);
        via.t1_latch.set(100);
        via.ier |= IFR_TIMER1;
        let mut handler = Unimock::new(
            InterruptHandlerMock::handle_interrupt
                .each_call(matching!(_, _, false))
                .returns(()),
        );
        via.step(&mut Registers::default(), &mut UnimplementedAddressable, &mut handler);
        handler.verify();
    }

    /// When timer1 underflows but IER does not have the timer1 bit enabled,
    /// the IRQ line stays low and the interrupt handler is never called.
    #[rstest]
    fn step_does_not_raise_irq_without_ier_enable(mut via: VIA2) {
        via.t1_counter.set(0);
        via.t1_latch.set(100);
        // IER timer1 bit is clear (default).
        via.step(
            &mut Registers::default(),
            &mut UnimplementedAddressable,
            &mut NoOpInterruptHandler,
        );
        // IFR_TIMER1 is flagged but IFR_IRQ should be clear (no enabled sources).
        assert_eq!(via.ifr.get() & IFR_TIMER1, IFR_TIMER1);
        assert_eq!(via.ifr.get() & IFR_IRQ, 0);
    }

    /// Even when IFR_TIMER1 is already set from a previous underflow,
    /// step calls the interrupt handler if the IER timer1 bit is enabled.
    #[rstest]
    fn step_handles_already_pending_timer1_irq(mut via: VIA2) {
        via.t1_counter.set(1); // won't underflow this step
        via.ifr.set(IFR_TIMER1); // left over from earlier event
        via.ier |= IFR_TIMER1;
        let mut handler = Unimock::new(
            InterruptHandlerMock::handle_interrupt
                .each_call(matching!(_, _, false))
                .returns(()),
        );
        via.step(&mut Registers::default(), &mut UnimplementedAddressable, &mut handler);
        handler.verify();
    }
}
