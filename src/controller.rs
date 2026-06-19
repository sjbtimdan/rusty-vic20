use crate::{
    emulator::{FRAME_TIME, ThreadReceivers, ThreadSenders, read_prg_file, spawn_emulator},
    paste::{self, PasteQueue},
    peripherals,
    ui::{
        audio,
        control::{
            BrakeAction,
            ControlState,
            IoAction,
            JoystickAction,
            MemoryAction,
            SharedPerformanceMetrics,
            display::ControlWindow,
        },
        keyboard::{KeyboardState, display::KeyboardWindow},
        screen::{
            display::{ScreenWindow, SharedVideoState},
            renderer::{ACTIVE_HEIGHT, CHAR_WIDTH, TEXT_COLUMNS},
        },
    },
};
use arboard::Clipboard;
use cpal::Stream;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Instant,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::ModifiersState,
};

#[derive(Default)]
pub struct Vic20Controller {
    screen: ScreenWindow,
    keyboard_window: KeyboardWindow,
    control_window: ControlWindow,
    shared_state: Option<ThreadSenders>,
    _emulator_thread: Option<JoinHandle<()>>,
    _audio_stream: Option<Stream>,
    keyboard_state: KeyboardState,
    control_state: ControlState,
    modifiers: ModifiersState,
}

impl Vic20Controller {
    fn shared_state(&self) -> &ThreadSenders {
        self.shared_state.as_ref().unwrap()
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().expect("failed to create event loop");
        event_loop.run_app(self).expect("event loop run failed");
    }
}

impl ApplicationHandler for Vic20Controller {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.screen.create(event_loop);
        self.keyboard_window.create(event_loop);
        self.control_window.create(event_loop);

        self.restart_emulator();
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
                    self.keyboard_window
                        .handle_event(event_loop, event, &mut self.keyboard_state);
                    let _ = self
                        .shared_state()
                        .keyboard_sender
                        .send(self.keyboard_state.physical_keys.clone());
                }
                _ => {
                    self.screen.handle_event(event_loop, event);
                }
            }
        } else if Some(window_id) == self.keyboard_window.window_id() {
            match event {
                WindowEvent::RedrawRequested => {
                    self.keyboard_window.draw(event_loop, &mut self.keyboard_state);
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
                    self.keyboard_window
                        .handle_event(event_loop, event, &mut self.keyboard_state);
                    let _ = self
                        .shared_state()
                        .keyboard_sender
                        .send(self.keyboard_state.physical_keys.clone());
                }
                _ => {
                    self.keyboard_window
                        .handle_event(event_loop, event, &mut self.keyboard_state);
                    let _ = self
                        .shared_state()
                        .keyboard_sender
                        .send(self.keyboard_state.physical_keys.clone());
                }
            }
        } else if Some(window_id) == self.control_window.window_id() {
            let perf = Arc::clone(&self.shared_state().perf);
            match event {
                WindowEvent::RedrawRequested => {
                    self.control_window.draw(&self.control_state, &perf);
                }
                _ => {
                    self.control_window
                        .handle_event(event_loop, event, &mut self.control_state, &perf);
                    if let Some(action) = self.control_state.io_action_pending.take() {
                        self.handle_io_action(action);
                    }
                    if let Some(action) = self.control_state.joystick_action_pending.take() {
                        self.handle_joystick_action(action);
                    }
                    if let Some(action) = self.control_state.memory_action_pending.take() {
                        self.handle_memory_action(action);
                    }
                    if let Some(action) = self.control_state.brake_action_pending.take() {
                        self.handle_brake_action(action);
                    }
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let screen_deadline = Instant::now() + FRAME_TIME;

        let keyboard_deadline = self.keyboard_window.next_deadline(&self.keyboard_state);

        let nearest = match keyboard_deadline {
            Some(kd) if kd < screen_deadline => kd,
            _ => screen_deadline,
        };

        event_loop.set_control_flow(ControlFlow::WaitUntil(nearest));

        self.screen.request_redraw();
        self.keyboard_window.request_redraw();
        self.control_window.request_redraw();
    }
}

impl Vic20Controller {
    fn restart_emulator(&mut self) {
        let (audio_producer, audio_stream) = audio::create_audio_channel();
        let (state, receivers) = self.create_channels();
        let handle = spawn_emulator(self.control_state.memory_expansion, audio_producer, receivers);
        self.shared_state = Some(state);
        self._emulator_thread = Some(handle);
        self._audio_stream = Some(audio_stream);
    }

    fn create_channels(&self) -> (ThreadSenders, ThreadReceivers) {
        let default_active_width = TEXT_COLUMNS * CHAR_WIDTH;
        let video = Arc::new(Mutex::new(SharedVideoState {
            screen_rgba: vec![0_u8; default_active_width * ACTIVE_HEIGHT * 4],
            border_rgba: [0x00, 0x44, 0xAA, 0xFF],
            active_width: default_active_width,
        }));
        let perf = Arc::new(Mutex::new(SharedPerformanceMetrics::default()));
        let (cassette_sender, cassette_receiver) = peripherals::cassette_player::make_cassette_channel();
        let (joystick_sender, joystick_receiver) = peripherals::joystick::make_joystick_channel();
        let (direct_loader_sender, direct_loader_receiver) = peripherals::direct_loader::make_direct_loader_channel();
        let load_queue = Arc::new(Mutex::new(VecDeque::new()));
        let (keyboard_sender, keyboard_receiver) = crate::peripherals::keyboard::make_keyboard_channel();
        let paste_queue: PasteQueue = paste::new_paste_queue();
        let (shutdown_sender, shutdown_receiver) = std::sync::mpsc::sync_channel::<()>(0);
        let (brake_sender, brake_receiver) = peripherals::brake::make_brake_channel();

        let state = ThreadSenders {
            video: Arc::clone(&video),
            perf: Arc::clone(&perf),
            keyboard_sender,
            paste_queue: paste_queue.clone(),
            load_queue: load_queue.clone(),
            cassette_sender,
            joystick_sender,
            direct_loader_sender,
            brake_sender,
            shutdown_sender,
        };
        let receivers = ThreadReceivers {
            video,
            perf,
            load_queue,
            paste_queue,
            keyboard_receiver,
            cassette_receiver,
            joystick_receiver,
            direct_loader_receiver,
            brake_receiver,
            shutdown_receiver,
        };
        (state, receivers)
    }

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

    fn handle_io_action(&mut self, action: IoAction) {
        match action {
            IoAction::OpenFile => self.handle_cassette_open_file(),
            IoAction::TogglePlay => {
                self.control_state.cassette_playing = !self.control_state.cassette_playing;
                let _ = self
                    .shared_state()
                    .cassette_sender
                    .send(self.control_state.cassette_playing);
            }
            IoAction::DirectLoad => {
                let path = rfd::FileDialog::new().add_filter("PRG files", &["prg"]).pick_file();
                if let Some(path) = path {
                    match std::fs::read(&path) {
                        Ok(data) => {
                            let _ = self.shared_state().direct_loader_sender.send(data);
                        }
                        Err(e) => {
                            log::error!("DirectLoad: failed to read '{}': {}", path.display(), e);
                        }
                    }
                }
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
            self.control_state.cassette_file = Some(path_str.to_string());
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

    fn handle_joystick_action(&mut self, action: JoystickAction) {
        match action {
            JoystickAction::StateChanged => {
                let update = peripherals::joystick::JoystickUpdate {
                    direction: self.control_state.joystick_direction,
                    fire: self.control_state.joystick_fire,
                };
                let _ = self.shared_state().joystick_sender.send(update);
            }
        }
    }

    fn handle_memory_action(&mut self, action: MemoryAction) {
        match action {
            MemoryAction::SetExpansion(expansion) => {
                log::info!("Memory expansion set to {:?}", expansion);
                self.control_state.memory_expansion = expansion;
            }
            MemoryAction::Reboot => {
                log::info!("Rebooting emulator");
                self.restart_emulator();
                log::info!("Emulator rebooted");
            }
        }
    }

    fn handle_brake_action(&mut self, action: BrakeAction) {
        match action {
            BrakeAction::SetSpeed(speed) => {
                self.control_state.brake_speed = speed;
                let _ = self.shared_state().brake_sender.send(speed);
            }
        }
    }

    fn handle_paste(&mut self) {
        let text = match Clipboard::new().and_then(|mut c| c.get_text()) {
            Ok(t) => t,
            Err(e) => {
                log::error!("Error getting clipboard text: {:?}", e);
                return;
            }
        };
        let petscii_bytes = paste::text_to_petscii(&text);
        if let Ok(mut q) = self.shared_state().paste_queue.lock() {
            q.extend(petscii_bytes);
        }
    }
}
