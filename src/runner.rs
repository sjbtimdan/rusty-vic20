use crate::{
    addressable::Addressable,
    audio::AudioProducer,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
    memory::MemoryExpansion,
    paste::PasteQueue,
    peripherals::{
        brake::{Brake, BrakeSpeed, make_brake_channel},
        cassette_player::CassettePlayer,
        direct_loader::DirectLoad,
        joystick::Joystick,
        keyboard::{Keyboard, RestoreKeyStatus, make_keyboard_channel},
        serial_port::SerialPort,
    },
    ui::keyboard::key::Key,
};
use std::{
    collections::HashSet,
    sync::mpsc::{Receiver, SyncSender},
};

pub struct EmulatorRunner {
    pub bus: Bus,
    pub cpu: CPU6502,
    pub cassette_player: CassettePlayer,
    pub joystick: Joystick,
    serial_port: SerialPort,
    pub direct_loader: DirectLoad,
    keyboard: Keyboard,
    pub keyboard_sender: SyncSender<HashSet<Key>>,
    pub paste_queue: PasteQueue,
    instruction_executor: instruction_executor::DefaultInstructionExecutor,
    pub brake: Brake,
    audio_producer: AudioProducer,
    audio_cycle: u64,
    audio_frac: f64,
}

impl EmulatorRunner {
    fn create_bus_and_cpu(memory_expansion: MemoryExpansion) -> (Bus, CPU6502) {
        let mut bus = Bus::default();
        bus.memory.set_expansion(memory_expansion);
        bus.load_standard_roms_from_data_dir();
        let reset_vector = bus.read_word(0xFFFC);
        let mut cpu = CPU6502::default();
        cpu.reset(reset_vector);
        (bus, cpu)
    }

    pub fn from_receiver(
        keyboard_receiver: Receiver<HashSet<Key>>,
        paste_queue: PasteQueue,
        memory_expansion: MemoryExpansion,
        brake_receiver: Receiver<BrakeSpeed>,
        audio_producer: AudioProducer,
    ) -> Self {
        let (bus, cpu) = Self::create_bus_and_cpu(memory_expansion);
        let (dummy_tx, _) = make_keyboard_channel();
        Self {
            bus,
            cpu,
            cassette_player: CassettePlayer::default(),
            joystick: Joystick::default(),
            serial_port: SerialPort,
            direct_loader: DirectLoad::default(),
            keyboard: Keyboard::new(keyboard_receiver, Some(paste_queue.clone())),
            keyboard_sender: dummy_tx,
            paste_queue,
            instruction_executor: instruction_executor::DefaultInstructionExecutor,
            brake: Brake::new_default(brake_receiver),
            audio_producer,
            audio_cycle: 0,
            audio_frac: 0.0,
        }
    }
}

impl Default for EmulatorRunner {
    fn default() -> Self {
        let (bus, cpu) = Self::create_bus_and_cpu(MemoryExpansion::None);
        let paste_queue = crate::paste::new_paste_queue();
        let (tx, rx) = make_keyboard_channel();
        let (_, brake_rx) = make_brake_channel();
        Self {
            bus,
            cpu,
            cassette_player: CassettePlayer::default(),
            joystick: Joystick::default(),
            serial_port: SerialPort,
            direct_loader: DirectLoad::default(),
            keyboard: Keyboard::new(rx, Some(paste_queue.clone())),
            keyboard_sender: tx,
            paste_queue,
            instruction_executor: instruction_executor::DefaultInstructionExecutor,
            brake: Brake::new_default(brake_rx),
            audio_producer: AudioProducer::noop(),
            audio_cycle: 0,
            audio_frac: 0.0,
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
        self.joystick.step(&mut self.bus.via1, &mut self.bus.via2);
        self.serial_port.step(&mut self.bus.via1);
        self.direct_loader.step(&mut self.bus);
        self.brake.step();
    }

    pub fn generate_audio(&mut self, elapsed_secs: f64) {
        const PHI2_HZ: f64 = 1_108_404.0;
        const AUDIO_HZ: f64 = 44_100.0;
        self.audio_frac += elapsed_secs * AUDIO_HZ;
        while self.audio_frac >= 1.0 {
            self.audio_frac -= 1.0;
            let vic = self.bus.vic.generate_sample(self.audio_cycle);
            let cb2 = self.bus.via2.generate_cb2_sample(self.audio_cycle);
            self.audio_producer.push((vic + cb2).clamp(-1.0, 1.0));
            self.audio_cycle += (PHI2_HZ / AUDIO_HZ) as u64;
        }
    }

    pub fn step_multiple(&mut self, count: usize) {
        for _ in 0..count {
            self.step();
        }
    }
}
