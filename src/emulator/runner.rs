use crate::{
    hardware::bus::Bus,
    peripherals::{
        brake::Brake,
        cassette_player::CassettePlayer,
        direct_loader::DirectLoad,
        joystick::Joystick,
        keyboard::{Keyboard, RestoreKeyStatus},
        serial_port::SerialPort,
    },
    ui::{audio::AudioProducer, screen::renderer::CHAR_WIDTH},
};
use nmos6502::CPU6502;
use std::{
    sync::mpsc::TryRecvError,
    time::{Duration, Instant},
};

use super::{PrgLoadRequest, ThreadReceivers};

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
    pub brake: Brake,
    pub audio_producer: AudioProducer,
    pub audio_cycle: u64,
    pub audio_frac: f64,
    pub receivers: ThreadReceivers,
}

impl EmulatorRunner {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        receivers: ThreadReceivers,
        bus: Bus,
        cpu: CPU6502,
        keyboard: Keyboard,
        brake: Brake,
        cassette_player: CassettePlayer,
        joystick: Joystick,
        serial_port: SerialPort,
        direct_loader: DirectLoad,
        audio_producer: AudioProducer,
    ) -> Self {
        Self {
            bus,
            cpu,
            cassette_player,
            joystick,
            serial_port,
            direct_loader,
            keyboard,
            brake,
            audio_producer,
            audio_cycle: 0,
            audio_frac: 0.0,
            receivers,
        }
    }

    pub fn step(&mut self) {
        self.keyboard
            .inject_paste_into_buffer(&mut self.bus, &self.receivers.paste_queue);
        let port_a = self.keyboard.step(self.bus.via2.port_b()).unwrap_or(0xFF);
        self.bus.via2.set_port_a(port_a);
        let restore = self.keyboard.restore_key_status();
        self.bus.via1.set_ca1_pin(restore == RestoreKeyStatus::Up);

        self.bus.step_devices(&mut self.cpu);
        self.cpu.cycle(&mut self.bus);
        self.cassette_player.step(&mut self.bus.via1);
        self.joystick.step(&mut self.bus.via1, &mut self.bus.via2);
        self.serial_port.step(&mut self.bus.via1);
        self.direct_loader.step(&mut self.bus);
        self.brake.step();
    }

    fn generate_audio(&mut self, elapsed_secs: f64) {
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

    pub fn run_loop(mut self) {
        let mut last_frame_publish = Instant::now();
        let mut last_perf_publish = Instant::now();
        let mut last_audio_batch = Instant::now();
        let mut frame_count: u64 = 0;
        let mut last_perf_total_cycles: u64 = 0;
        let mut last_perf_frame_count: u64 = 0;

        self.bus.via1.set_port_b_callback(Box::new(cassette_motor_control));

        loop {
            if self.receivers.shutdown_receiver.try_recv() == Err(TryRecvError::Disconnected) {
                break;
            }

            if let Ok(pressed) = self.receivers.cassette_receiver.try_recv() {
                self.cassette_player.set_play_button(pressed);
            }
            if let Ok(update) = self.receivers.joystick_receiver.try_recv() {
                self.joystick.set_state(update);
            }
            if let Ok(data) = self.receivers.direct_loader_receiver.try_recv() {
                self.direct_loader.set_state(data);
            }
            if let Ok(mut q) = self.receivers.load_queue.try_lock() {
                while let Some(request) = q.pop_front() {
                    load_prg(&mut self.bus, &request);
                }
            }

            self.step();

            if last_frame_publish.elapsed() >= FRAME_PUBLISH_INTERVAL {
                let elapsed = last_audio_batch.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    self.generate_audio(elapsed);
                }
                last_audio_batch = Instant::now();

                self.bus.render_active_screen();
                let latest_border_rgba = self.bus.border_rgba();
                let active_width = self.bus.columns() * CHAR_WIDTH;
                let mut shared = match self.receivers.video.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let fb = self.bus.frame_buffer();
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
                let cycles_delta = self.cpu.total_cycles - last_perf_total_cycles;
                let frames_delta = frame_count - last_perf_frame_count;
                let mut perf = match self.receivers.perf.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                perf.cycles_per_second = cycles_delta as f64 / elapsed;
                perf.frames_per_second = frames_delta as f64 / elapsed;
                perf.total_cycles = self.cpu.total_cycles;
                perf.total_frames = frame_count;
                last_perf_total_cycles = self.cpu.total_cycles;
                last_perf_frame_count = frame_count;
                last_perf_publish = Instant::now();
            }
        }
    }
}

fn load_prg(bus: &mut Bus, request: &PrgLoadRequest) {
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
