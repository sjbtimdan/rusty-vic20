use crate::{
    addressable::Addressable,
    bus::Bus,
    cpu::cpu6502::CPU6502,
    debug::{
        CassetteAction,
        DebugState,
        PendingRegisterWrites,
        PendingWrites,
        SharedMemory,
        SharedPerfState,
        SharedPerformanceMetrics,
        SharedRegisters,
        SharedRegistersState,
        display::DebugWindow,
    },
    paste::{self, PasteQueue},
    runner::EmulatorRunner,
    ui::{
        self,
        keyboard::{KeyboardState, display::KeyboardWindow},
        screen::{
            display::{ScreenWindow, SharedVideoState},
            renderer::{ACTIVE_HEIGHT, ACTIVE_WIDTH},
        },
    },
};
use arboard::Clipboard;
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex, mpsc::SyncSender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::ModifiersState,
};

const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / 50);
const FRAME_PUBLISH_INTERVAL: Duration = Duration::from_millis(50);
const MEMORY_PUBLISH_INTERVAL: Duration = Duration::from_millis(500);
const PERF_PUBLISH_INTERVAL: Duration = Duration::from_secs(1);

struct SharedState {
    video: Arc<Mutex<SharedVideoState>>,
    memory: SharedMemory,
    pending_writes: PendingWrites,
    registers: SharedRegistersState,
    pending_register_writes: PendingRegisterWrites,
    perf: SharedPerfState,
    keyboard_sender: SyncSender<HashSet<crate::ui::keyboard::key::Key>>,
    paste_queue: PasteQueue,
    load_queue: LoadQueue,
    cassette_sender: SyncSender<bool>,
}

#[derive(Default)]
pub struct Vic20Controller {
    screen: ScreenWindow,
    keyboard: KeyboardWindow,
    debug: DebugWindow,
    shared_state: Option<SharedState>,
    keyboard_state: KeyboardState,
    debug_state: DebugState,
    vic_thread: Option<JoinHandle<()>>,
    modifiers: ModifiersState,
}

impl Vic20Controller {
    fn shared_state(&self) -> &SharedState {
        self.shared_state.as_ref().unwrap()
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().expect("failed to create event loop");
        event_loop.run_app(self).expect("event loop run failed");
    }

    fn spawn_emulator() -> (JoinHandle<()>, SharedState) {
        let video = Arc::new(Mutex::new(SharedVideoState {
            screen_rgba: vec![0_u8; ACTIVE_WIDTH * ACTIVE_HEIGHT * 4],
            border_rgba: [0x00, 0x44, 0xAA, 0xFF],
        }));
        let memory: SharedMemory = Arc::new(Mutex::new([0u8; 65536]));
        let pending_writes: PendingWrites = Arc::new(Mutex::new(Vec::new()));
        let registers: SharedRegistersState = Arc::new(Mutex::new(SharedRegisters {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            status: 0,
        }));
        let pending_register_writes: PendingRegisterWrites = Arc::new(Mutex::new(Vec::new()));
        let perf: SharedPerfState = Arc::new(Mutex::new(SharedPerformanceMetrics::default()));
        let (cassette_sender, cassette_receiver) = ui::cassette_player::make_cassette_channel();
        let load_queue: LoadQueue = new_load_queue();
        let load_queue_for_thread = load_queue.clone();
        let (keyboard_sender, keyboard_receiver) = crate::keyboard::make_keyboard_channel();
        let paste_queue: PasteQueue = paste::new_paste_queue();
        let paste_queue_for_state = paste_queue.clone();

        let handle = thread::Builder::new()
            .name("vic20-core-loop".to_string())
            .spawn({
                let video = Arc::clone(&video);
                let memory = Arc::clone(&memory);
                let pending_writes = Arc::clone(&pending_writes);
                let registers = Arc::clone(&registers);
                let pending_register_writes = Arc::clone(&pending_register_writes);
                let perf = Arc::clone(&perf);
                move || {
                    let runner = EmulatorRunner::from_receiver(keyboard_receiver, paste_queue);
                    Self::run_emulator(
                        runner,
                        video,
                        memory,
                        pending_writes,
                        registers,
                        pending_register_writes,
                        perf,
                        load_queue_for_thread,
                        cassette_receiver,
                    )
                }
            })
            .expect("failed to spawn VIC-20 core thread");

        (
            handle,
            SharedState {
                video,
                memory,
                pending_writes,
                registers,
                pending_register_writes,
                perf,
                keyboard_sender,
                paste_queue: paste_queue_for_state,
                load_queue,
                cassette_sender,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_emulator(
        mut runner: EmulatorRunner,
        shared_video_state: Arc<Mutex<SharedVideoState>>,
        shared_memory: SharedMemory,
        pending_writes: PendingWrites,
        shared_registers: SharedRegistersState,
        pending_register_writes: PendingRegisterWrites,
        shared_perf: SharedPerfState,
        load_queue: LoadQueue,
        cassette_receiver: std::sync::mpsc::Receiver<bool>,
    ) {
        let mut last_frame_publish = Instant::now();
        let mut last_memory_publish = Instant::now();
        let mut last_perf_publish = Instant::now();
        let mut frame_count: u64 = 0;
        let mut last_perf_total_cycles: u64 = 0;
        let mut last_perf_frame_count: u64 = 0;

        runner.bus.via1.set_port_b_callback(Box::new(cassette_motor_control));

        loop {
            runner.step_keyboard();

            if let Ok(pressed) = cassette_receiver.try_recv() {
                runner.cassette_player.set_play_button(pressed);
            }

            // Apply any pending writes from the debugger (non-blocking)
            if let Ok(mut writes) = pending_writes.try_lock() {
                for (addr, value) in writes.drain(..) {
                    runner.bus.write_byte(addr, value);
                }
            }

            // Process any pending .prg load requests
            process_load_queue(&mut runner.bus, &mut runner.cpu, &load_queue);

            // Apply any pending register writes from the debugger (non-blocking)
            if let Ok(mut reg_writes) = pending_register_writes.try_lock() {
                for (field, value) in reg_writes.drain(..) {
                    match field {
                        crate::debug::RegisterField::A => runner.cpu.registers.a = value as u8,
                        crate::debug::RegisterField::X => runner.cpu.registers.x = value as u8,
                        crate::debug::RegisterField::Y => runner.cpu.registers.y = value as u8,
                        crate::debug::RegisterField::SP => runner.cpu.registers.sp = value as u8,
                        crate::debug::RegisterField::PC => runner.cpu.registers.pc = value,
                        crate::debug::RegisterField::Status => runner.cpu.registers.status = value as u8,
                    }
                }
            }

            runner.step();

            if last_frame_publish.elapsed() >= FRAME_PUBLISH_INTERVAL {
                runner.bus.render_active_screen();
                let latest_border_rgba = runner.bus.border_rgba();
                let mut shared = match shared_video_state.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                shared.screen_rgba.copy_from_slice(runner.bus.frame_buffer());
                shared.border_rgba = latest_border_rgba;
                last_frame_publish = Instant::now();
                frame_count += 1;

                // Publish registers alongside the frame
                let mut regs = match shared_registers.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                regs.a = runner.cpu.registers.a;
                regs.x = runner.cpu.registers.x;
                regs.y = runner.cpu.registers.y;
                regs.sp = runner.cpu.registers.sp;
                regs.pc = runner.cpu.registers.pc;
                regs.status = runner.cpu.registers.status;
            }

            if last_memory_publish.elapsed() >= MEMORY_PUBLISH_INTERVAL {
                let mut mem = match shared_memory.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                runner.bus.copy_memory_to(&mut mem);
                last_memory_publish = Instant::now();
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

impl ApplicationHandler for Vic20Controller {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.screen.create(event_loop);
        self.keyboard.create(event_loop);
        self.debug.create(event_loop);

        let (handle, state) = Self::spawn_emulator();
        self.vic_thread = Some(handle);
        self.shared_state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
        if Some(window_id) == self.screen.window_id() {
            match event {
                WindowEvent::RedrawRequested => {
                    let video_ref = Arc::clone(&self.shared_state().video);
                    let shared = match video_ref.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    self.screen.draw(event_loop, &shared);
                }
                WindowEvent::ModifiersChanged(mods) => {
                    self.modifiers = mods.state();
                }
                WindowEvent::KeyboardInput {
                    event: ref key_event, ..
                } => {
                    if key_event.state == ElementState::Pressed && self.is_paste_shortcut(key_event) {
                        self.handle_paste();
                        return;
                    }
                    self.keyboard.handle_event(event_loop, event, &mut self.keyboard_state);
                    let _ = self
                        .shared_state()
                        .keyboard_sender
                        .send(self.keyboard_state.physical_keys.clone());
                }
                _ => {
                    self.screen.handle_event(event_loop, event);
                }
            }
        } else if Some(window_id) == self.keyboard.window_id() {
            match event {
                WindowEvent::RedrawRequested => {
                    self.keyboard.draw(event_loop, &mut self.keyboard_state);
                }
                WindowEvent::ModifiersChanged(mods) => {
                    self.modifiers = mods.state();
                }
                WindowEvent::KeyboardInput {
                    event: ref key_event, ..
                } => {
                    if key_event.state == ElementState::Pressed && self.is_paste_shortcut(key_event) {
                        self.handle_paste();
                        return;
                    }
                    self.keyboard.handle_event(event_loop, event, &mut self.keyboard_state);
                    let _ = self
                        .shared_state()
                        .keyboard_sender
                        .send(self.keyboard_state.physical_keys.clone());
                }
                _ => {
                    self.keyboard.handle_event(event_loop, event, &mut self.keyboard_state);
                    let _ = self
                        .shared_state()
                        .keyboard_sender
                        .send(self.keyboard_state.physical_keys.clone());
                }
            }
        } else if Some(window_id) == self.debug.window_id() {
            let memory = Arc::clone(&self.shared_state().memory);
            let registers = Arc::clone(&self.shared_state().registers);
            let pending_writes = Arc::clone(&self.shared_state().pending_writes);
            let pending_register_writes = Arc::clone(&self.shared_state().pending_register_writes);
            let perf = Arc::clone(&self.shared_state().perf);
            match event {
                WindowEvent::RedrawRequested => {
                    self.debug.draw(&self.debug_state, &memory, &registers, &perf);
                }
                _ => {
                    self.debug.handle_event(
                        event_loop,
                        event,
                        &mut self.debug_state,
                        &memory,
                        &pending_writes,
                        &registers,
                        &pending_register_writes,
                        &perf,
                    );
                    if let Some(action) = self.debug_state.cassette_action_pending.take() {
                        self.handle_cassette_action(action);
                    }
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let screen_deadline = Instant::now() + FRAME_TIME;

        let keyboard_deadline = self.keyboard.next_deadline(&self.keyboard_state);

        let nearest = match keyboard_deadline {
            Some(kd) if kd < screen_deadline => kd,
            _ => screen_deadline,
        };

        event_loop.set_control_flow(ControlFlow::WaitUntil(nearest));

        self.screen.request_redraw();
        self.keyboard.request_redraw();
        self.debug.request_redraw();
    }
}

impl Vic20Controller {
    fn is_paste_shortcut(&self, key_event: &winit::event::KeyEvent) -> bool {
        if key_event.logical_key == winit::keyboard::Key::Character("v".into())
            || key_event.logical_key == winit::keyboard::Key::Character("V".into())
        {
            #[cfg(target_os = "macos")]
            {
                self.modifiers.super_key()
            }
            #[cfg(not(target_os = "macos"))]
            {
                self.modifiers.control_key()
            }
        } else {
            false
        }
    }

    fn handle_cassette_action(&mut self, action: CassetteAction) {
        match action {
            CassetteAction::OpenFile => self.handle_cassette_open_file(),
            CassetteAction::TogglePlay => {
                self.debug_state.cassette_playing = !self.debug_state.cassette_playing;
                let _ = self
                    .shared_state()
                    .cassette_sender
                    .send(self.debug_state.cassette_playing);
            }
        }
    }

    fn handle_cassette_open_file(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("Tape files", &["tap", "prg"])
            .pick_file();
        if let Some(path) = path
            && let Some(path_str) = path.to_str()
        {
            self.debug_state.cassette_file = Some(path_str.to_string());
            match read_prg_file(path_str) {
                Ok(request) => {
                    if let Ok(mut q) = self.shared_state().load_queue.lock() {
                        q.push_back(request);
                    }
                }
                Err(e) => {
                    log::error!("{}", e);
                }
            }
        }
    }

    fn handle_paste(&mut self) {
        let text = match Clipboard::new().and_then(|mut c| c.get_text()) {
            Ok(t) => t,
            Err(_) => return,
        };

        let petscii_bytes = paste::text_to_petscii(&text);
        if let Ok(mut q) = self.shared_state().paste_queue.lock() {
            q.extend(petscii_bytes);
        }
    }
}

struct PrgLoadRequest {
    path: String,
    data: Vec<u8>,
}

type LoadQueue = Arc<Mutex<VecDeque<PrgLoadRequest>>>;

fn new_load_queue() -> LoadQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

fn read_prg_file(path: &str) -> Result<PrgLoadRequest, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    if data.len() < 2 {
        return Err(format!("'{}' is too small to be a valid .prg file", path));
    }
    Ok(PrgLoadRequest {
        path: path.to_string(),
        data,
    })
}

fn process_load_queue(bus: &mut Bus, cpu: &mut CPU6502, queue: &LoadQueue) {
    if let Ok(mut q) = queue.try_lock() {
        while let Some(request) = q.pop_front() {
            apply_prg(bus, cpu, &request);
        }
    }
}

fn apply_prg(bus: &mut Bus, _cpu: &mut CPU6502, request: &PrgLoadRequest) {
    if request.data.len() < 2 {
        log::warn!("Skipping invalid .prg (too small): {}", request.path);
        return;
    }
    let load_address = u16::from_le_bytes([request.data[0], request.data[1]]);
    log::info!("Loading program into memory starting at {}", load_address);
    let program = &request.data[2..];

    let max_len = 65536usize.saturating_sub(load_address as usize);
    if program.len() > max_len {
        log::warn!(
            "Truncating .prg '{}': load ${:04X} + {} bytes exceeds 64KB",
            request.path,
            load_address,
            program.len()
        );
    }
    let len = program.len().min(max_len);
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
