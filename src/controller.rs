use crate::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
    keyboard::{KeyboardState, display::KeyboardWindow},
    screen::{
        display::{ScreenWindow, SharedVideoState},
        renderer::{ACTIVE_HEIGHT, ACTIVE_WIDTH},
    },
};
use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};

const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / 50);
const FRAME_PUBLISH_INTERVAL: Duration = Duration::from_millis(20);

pub struct Vic20Controller {
    screen: ScreenWindow,
    keyboard: KeyboardWindow,
    shared_video_state: Arc<Mutex<SharedVideoState>>,
    keyboard_state: Arc<Mutex<KeyboardState>>,
    tick_duration: Duration,
    vic_thread: Option<JoinHandle<()>>,
}

impl Vic20Controller {
    pub fn new(tick_duration: Duration) -> Self {
        Self {
            screen: ScreenWindow::default(),
            keyboard: KeyboardWindow::default(),
            shared_video_state: Arc::new(Mutex::new(SharedVideoState {
                screen_rgba: vec![0_u32; ACTIVE_WIDTH * ACTIVE_HEIGHT],
                border_rgba: 0x0044AAFF,
            })),
            keyboard_state: Arc::new(Mutex::new(KeyboardState::new())),
            tick_duration,
            vic_thread: None,
        }
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().expect("failed to create event loop");
        event_loop.run_app(self).expect("event loop run failed");
    }

    fn spawn_emulator(&mut self) {
        let shared_video_state = Arc::clone(&self.shared_video_state);
        let tick_duration = self.tick_duration;

        let handle = thread::Builder::new()
            .name("vic20-core-loop".to_string())
            .spawn(move || Self::run_emulator(shared_video_state, tick_duration))
            .expect("failed to spawn VIC-20 core thread");
        self.vic_thread = Some(handle);
    }

    fn run_emulator(shared_video_state: Arc<Mutex<SharedVideoState>>, tick_duration: Duration) {
        let mut cpu = CPU6502::default();
        let mut bus = Bus::default();
        let mut last_frame_publish = Instant::now();
        let instruction_executor = instruction_executor::DefaultInstructionExecutor;

        bus.load_standard_roms_from_data_dir();
        let reset_vector = bus.read_word(0xFFFC);
        cpu.reset(reset_vector);

        loop {
            cpu.step(&mut bus, &instruction_executor);
            bus.step_devices(&mut cpu);

            if last_frame_publish.elapsed() >= FRAME_PUBLISH_INTERVAL {
                let latest_screen_rgba = bus.render_active_screen();
                let latest_border_rgba = bus.border_rgba();
                let mut shared = match shared_video_state.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                shared.screen_rgba = latest_screen_rgba;
                shared.border_rgba = latest_border_rgba;
                last_frame_publish = Instant::now();
            }

            if !tick_duration.is_zero() {
                thread::sleep(tick_duration);
            }
        }
    }
}

impl ApplicationHandler for Vic20Controller {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.screen.create(event_loop);
        self.keyboard.create(event_loop);
        self.spawn_emulator();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
        if Some(window_id) == self.screen.window_id() {
            match event {
                WindowEvent::RedrawRequested => {
                    let shared = match self.shared_video_state.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    self.screen.draw(event_loop, &shared);
                }
                _ => {
                    self.screen.handle_event(event_loop, event);
                }
            }
        } else if Some(window_id) == self.keyboard.window_id() {
            let mut state = match self.keyboard_state.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            match event {
                WindowEvent::RedrawRequested => {
                    self.keyboard.draw(event_loop, &mut state);
                }
                _ => {
                    self.keyboard.handle_event(event_loop, event, &mut state);
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let screen_deadline = Instant::now() + FRAME_TIME;

        let keyboard_deadline = {
            let state = match self.keyboard_state.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            self.keyboard.next_deadline(&state)
        };

        let nearest = match keyboard_deadline {
            Some(kd) if kd < screen_deadline => kd,
            _ => screen_deadline,
        };

        event_loop.set_control_flow(ControlFlow::WaitUntil(nearest));

        self.screen.request_redraw();
        self.keyboard.request_redraw();
    }
}
