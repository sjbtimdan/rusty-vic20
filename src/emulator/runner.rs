use crate::{
    cpu::cpu6502::CPU6502,
    hardware::{addressable::Addressable, bus::Bus, memory::MemoryExpansion},
    peripherals::{
        brake::{Brake, BrakeSpeed},
        cassette_player::CassettePlayer,
        direct_loader::DirectLoad,
        joystick::Joystick,
        keyboard::{Keyboard, RestoreKeyStatus, make_keyboard_channel},
        serial_port::SerialPort,
    },
    ui::{
        audio::AudioProducer,
        control::SharedPerformanceMetrics,
        keyboard::key::Key,
        screen::{display::SharedVideoState, renderer::CHAR_WIDTH},
    },
};
use std::{
    collections::HashSet,
    sync::{
        Arc,
        Mutex,
        mpsc::{Receiver, SyncSender, TryRecvError},
    },
    time::{Duration, Instant},
};

use super::{LoadQueue, PrgLoadRequest, paste::PasteQueue};

const FRAME_PUBLISH_INTERVAL: Duration = Duration::from_millis(50);
const PERF_PUBLISH_INTERVAL: Duration = Duration::from_secs(1);

pub struct EmulatorRunner {
    pub bus: Bus,
    pub cpu: CPU6502,
    pub cassette_player: CassettePlayer,
    pub joystick: Joystick,
    pub serial_port: SerialPort,
    pub direct_loader: DirectLoad,
    pub keyboard: Keyboard,
    pub keyboard_sender: SyncSender<HashSet<Key>>,
    pub paste_queue: PasteQueue,
    pub brake: Brake,
    pub audio_producer: AudioProducer,
    pub audio_cycle: u64,
    pub audio_frac: f64,
}

impl EmulatorRunner {
    pub fn create_bus_and_cpu(memory_expansion: MemoryExpansion) -> (Bus, CPU6502) {
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
            brake: Brake::new_default(brake_receiver),
            audio_producer,
            audio_cycle: 0,
            audio_frac: 0.0,
        }
    }

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
        self.cpu.step(&mut self.bus);
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

    #[allow(clippy::too_many_arguments)]
    pub fn run_loop(
        self,
        shared_video_state: Arc<Mutex<SharedVideoState>>,
        shared_perf: Arc<Mutex<SharedPerformanceMetrics>>,
        load_queue: LoadQueue,
        cassette_receiver: Receiver<bool>,
        joystick_receiver: Receiver<crate::peripherals::joystick::JoystickUpdate>,
        direct_loader_receiver: Receiver<Vec<u8>>,
        shutdown_receiver: Receiver<()>,
    ) {
        let mut runner = self;
        let mut last_frame_publish = Instant::now();
        let mut last_perf_publish = Instant::now();
        let mut last_audio_batch = Instant::now();
        let mut frame_count: u64 = 0;
        let mut last_perf_total_cycles: u64 = 0;
        let mut last_perf_frame_count: u64 = 0;

        runner.bus.via1.set_port_b_callback(Box::new(cassette_motor_control));

        loop {
            if shutdown_receiver.try_recv() == Err(TryRecvError::Disconnected) {
                break;
            }

            runner.step_keyboard();

            if let Ok(pressed) = cassette_receiver.try_recv() {
                runner.cassette_player.set_play_button(pressed);
            }
            if let Ok(update) = joystick_receiver.try_recv() {
                runner.joystick.set_state(update);
            }
            if let Ok(data) = direct_loader_receiver.try_recv() {
                runner.direct_loader.set_state(data);
            }
            if let Ok(mut q) = load_queue.try_lock() {
                while let Some(request) = q.pop_front() {
                    load_prg(&mut runner.bus, &mut runner.cpu, &request);
                }
            }

            runner.step();

            if last_frame_publish.elapsed() >= FRAME_PUBLISH_INTERVAL {
                let elapsed = last_audio_batch.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    runner.generate_audio(elapsed);
                }
                last_audio_batch = Instant::now();

                runner.bus.render_active_screen();
                let latest_border_rgba = runner.bus.border_rgba();
                let active_width = runner.bus.columns() * CHAR_WIDTH;
                let mut shared = match shared_video_state.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let fb = runner.bus.frame_buffer();
                if shared.screen_rgba.len() != fb.len() {
                    shared.screen_rgba.resize(fb.len(), 0);
                }
                shared.screen_rgba.copy_from_slice(fb);
                shared.border_rgba = latest_border_rgba;
                shared.active_width = active_width;
                last_frame_publish = Instant::now();
                frame_count += 1;
            }

            if last_perf_publish.elapsed() >= PERF_PUBLISH_INTERVAL {
                let elapsed = last_perf_publish.elapsed().as_secs_f64();
                let cycles_delta = runner.cpu.total_cycles() - last_perf_total_cycles;
                let frames_delta = frame_count - last_perf_frame_count;
                let mut perf = match shared_perf.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                perf.cycles_per_second = cycles_delta as f64 / elapsed;
                perf.frames_per_second = frames_delta as f64 / elapsed;
                perf.total_cycles = runner.cpu.total_cycles();
                perf.total_frames = frame_count;
                last_perf_total_cycles = runner.cpu.total_cycles();
                last_perf_frame_count = frame_count;
                last_perf_publish = Instant::now();
            }
        }
    }
}

fn load_prg(bus: &mut Bus, _cpu: &mut CPU6502, request: &PrgLoadRequest) {
    let load_address = u16::from_le_bytes([request.data[0], request.data[1]]);
    log::info!("Loading program into memory starting at {}", load_address);
    let program = &request.data[2..];
    let len = program.len();
    bus.load_data(load_address as usize, program);
    log::info!(
        "Loaded '{}' at ${:04X} ({} bytes), resetting PC to ${:04X}",
        request.path,
        load_address,
        len,
        load_address
    );
}

fn cassette_motor_control(port_b: u8) {
    if port_b & 0x08 == 0x08 {
        log::debug!("Cassette motor on")
    } else {
        log::debug!("Cassette motor off")
    };
}
