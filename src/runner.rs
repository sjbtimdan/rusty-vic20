use crate::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
    keyboard::{Keyboard, RestoreKeyStatus, make_keyboard_channel},
    paste::PasteQueue,
    ui::{cassette_player::CassettePlayer, keyboard::key::Key},
};
use std::{
    collections::HashSet,
    sync::mpsc::{Receiver, SyncSender},
};

pub struct EmulatorRunner {
    pub bus: Bus,
    pub cpu: CPU6502,
    pub cassette_player: CassettePlayer,
    keyboard: Keyboard,
    pub keyboard_sender: SyncSender<HashSet<Key>>,
    pub paste_queue: PasteQueue,
    instruction_executor: instruction_executor::DefaultInstructionExecutor,
}

impl EmulatorRunner {
    fn create_bus_and_cpu() -> (Bus, CPU6502) {
        let mut bus = Bus::default();
        bus.load_standard_roms_from_data_dir();
        let reset_vector = bus.read_word(0xFFFC);
        let mut cpu = CPU6502::default();
        cpu.reset(reset_vector);
        (bus, cpu)
    }

    pub fn from_receiver(keyboard_receiver: Receiver<HashSet<Key>>, paste_queue: PasteQueue) -> Self {
        let (bus, cpu) = Self::create_bus_and_cpu();
        let (dummy_tx, _) = make_keyboard_channel();
        Self {
            bus,
            cpu,
            cassette_player: CassettePlayer::default(),
            keyboard: Keyboard::new(keyboard_receiver, Some(paste_queue.clone())),
            keyboard_sender: dummy_tx,
            paste_queue,
            instruction_executor: instruction_executor::DefaultInstructionExecutor,
        }
    }
}

impl Default for EmulatorRunner {
    fn default() -> Self {
        let (bus, cpu) = Self::create_bus_and_cpu();
        let paste_queue = crate::paste::new_paste_queue();
        let (tx, rx) = make_keyboard_channel();
        Self {
            bus,
            cpu,
            cassette_player: CassettePlayer::default(),
            keyboard: Keyboard::new(rx, Some(paste_queue.clone())),
            keyboard_sender: tx,
            paste_queue,
            instruction_executor: instruction_executor::DefaultInstructionExecutor,
        }
    }
}

impl EmulatorRunner {
    pub fn step_keyboard(&mut self) {
        self.keyboard.inject_paste_into_buffer(&mut self.bus);
        if let Some(port_a) = self.keyboard.step(self.bus.via2.port_b()) {
            self.bus.via2.set_port_a(port_a);
        } else {
            self.bus.via2.set_port_a(0xFF);
        }
        let restore = self.keyboard.restore_key_status();
        self.bus.via1.set_ca1_pin(restore == RestoreKeyStatus::Up);
    }

    pub fn step(&mut self) {
        self.bus.step_devices(&mut self.cpu);
        self.cpu.step(&mut self.bus, &self.instruction_executor);
        self.cassette_player.step(&mut self.bus.via1);
    }

    pub fn step_multiple(&mut self, count: usize) {
        for _ in 0..count {
            self.step();
        }
    }
}
