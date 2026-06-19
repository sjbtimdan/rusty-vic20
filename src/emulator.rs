use crate::{
    bus::Bus,
    cpu::cpu6502::CPU6502,
    memory::MemoryExpansion,
    paste::PasteQueue,
    peripherals::{self, brake::BrakeSpeed},
    runner::EmulatorRunner,
    ui::{
        audio,
        control::SharedPerformanceMetrics,
        keyboard::key,
        screen::{display::SharedVideoState, renderer::CHAR_WIDTH},
    },
};
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        Arc,
        Mutex,
        mpsc::{Receiver, SyncSender, TryRecvError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / 50);
const FRAME_PUBLISH_INTERVAL: Duration = Duration::from_millis(50);
const PERF_PUBLISH_INTERVAL: Duration = Duration::from_secs(1);

pub struct ThreadSenders {
    pub video: Arc<Mutex<SharedVideoState>>,
    pub perf: Arc<Mutex<SharedPerformanceMetrics>>,
    pub keyboard_sender: SyncSender<HashSet<key::Key>>,
    pub paste_queue: PasteQueue,
    pub load_queue: LoadQueue,
    pub cassette_sender: SyncSender<bool>,
    pub joystick_sender: SyncSender<peripherals::joystick::JoystickUpdate>,
    pub direct_loader_sender: SyncSender<Vec<u8>>,
    pub brake_sender: SyncSender<BrakeSpeed>,
    pub shutdown_sender: SyncSender<()>,
}

pub struct ThreadReceivers {
    pub video: Arc<Mutex<SharedVideoState>>,
    pub perf: Arc<Mutex<SharedPerformanceMetrics>>,
    pub load_queue: LoadQueue,
    pub paste_queue: PasteQueue,
    pub keyboard_receiver: Receiver<HashSet<key::Key>>,
    pub cassette_receiver: Receiver<bool>,
    pub joystick_receiver: Receiver<peripherals::joystick::JoystickUpdate>,
    pub direct_loader_receiver: Receiver<Vec<u8>>,
    pub brake_receiver: Receiver<BrakeSpeed>,
    pub shutdown_receiver: Receiver<()>,
}

impl EmulatorRunner {
    #[allow(clippy::too_many_arguments)]
    pub fn run_loop(
        self,
        shared_video_state: Arc<Mutex<SharedVideoState>>,
        shared_perf: Arc<Mutex<SharedPerformanceMetrics>>,
        load_queue: LoadQueue,
        cassette_receiver: Receiver<bool>,
        joystick_receiver: Receiver<peripherals::joystick::JoystickUpdate>,
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

pub fn spawn_emulator(
    memory_expansion: MemoryExpansion,
    audio_producer: audio::AudioProducer,
    receivers: ThreadReceivers,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("vic20-core-loop".to_string())
        .spawn(move || {
            let runner = EmulatorRunner::from_receiver(
                receivers.keyboard_receiver,
                receivers.paste_queue,
                memory_expansion,
                receivers.brake_receiver,
                audio_producer,
            );
            runner.run_loop(
                receivers.video,
                receivers.perf,
                receivers.load_queue,
                receivers.cassette_receiver,
                receivers.joystick_receiver,
                receivers.direct_loader_receiver,
                receivers.shutdown_receiver,
            )
        })
        .expect("failed to spawn VIC-20 core thread")
}

pub struct PrgLoadRequest {
    path: String,
    data: Vec<u8>,
}

pub type LoadQueue = Arc<Mutex<VecDeque<PrgLoadRequest>>>;

pub fn read_prg_file(path: &str) -> Result<PrgLoadRequest, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    if data.len() < 2 {
        return Err(format!("'{}' is too small to be a valid .prg file", path));
    }
    Ok(PrgLoadRequest {
        path: path.to_string(),
        data,
    })
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
