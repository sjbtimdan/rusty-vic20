use crate::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
    debug::{
        DebugState, PendingRegisterWrites, PendingWrites, SharedMemory, SharedRegisters, SharedRegistersState,
        display::DebugWindow,
    },
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
const MEMORY_PUBLISH_INTERVAL: Duration = Duration::from_millis(200);

struct SharedState {
    video: Arc<Mutex<SharedVideoState>>,
    memory: SharedMemory,
    pending_writes: PendingWrites,
    registers: SharedRegistersState,
    pending_register_writes: PendingRegisterWrites,
}

pub struct Vic20Controller {
    screen: ScreenWindow,
    keyboard: KeyboardWindow,
    debug: DebugWindow,
    shared_state: Option<SharedState>,
    keyboard_state: KeyboardState,
    debug_state: DebugState,
    tick_duration: Duration,
    vic_thread: Option<JoinHandle<()>>,
}

impl Vic20Controller {
    pub fn new(tick_duration: Duration) -> Self {
        Self {
            screen: ScreenWindow::default(),
            keyboard: KeyboardWindow::default(),
            debug: DebugWindow::default(),
            shared_state: None,
            keyboard_state: KeyboardState::new(),
            debug_state: DebugState::new(),
            tick_duration,
            vic_thread: None,
        }
    }

    fn shared_state(&self) -> &SharedState {
        self.shared_state.as_ref().unwrap()
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().expect("failed to create event loop");
        event_loop.run_app(self).expect("event loop run failed");
    }

    fn spawn_emulator(tick_duration: Duration) -> (JoinHandle<()>, SharedState) {
        let video = Arc::new(Mutex::new(SharedVideoState {
            screen_rgba: vec![0_u32; ACTIVE_WIDTH * ACTIVE_HEIGHT],
            border_rgba: 0x0044AAFF,
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

        let handle = thread::Builder::new()
            .name("vic20-core-loop".to_string())
            .spawn({
                let video = Arc::clone(&video);
                let memory = Arc::clone(&memory);
                let pending_writes = Arc::clone(&pending_writes);
                let registers = Arc::clone(&registers);
                let pending_register_writes = Arc::clone(&pending_register_writes);
                move || {
                    Self::run_emulator(
                        video,
                        memory,
                        pending_writes,
                        registers,
                        pending_register_writes,
                        tick_duration,
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
            },
        )
    }

    fn run_emulator(
        shared_video_state: Arc<Mutex<SharedVideoState>>,
        shared_memory: SharedMemory,
        pending_writes: PendingWrites,
        shared_registers: SharedRegistersState,
        pending_register_writes: PendingRegisterWrites,
        tick_duration: Duration,
    ) {
        let mut cpu = CPU6502::default();
        let mut bus = Bus::default();
        let mut last_frame_publish = Instant::now();
        let mut last_memory_publish = Instant::now();
        let instruction_executor = instruction_executor::DefaultInstructionExecutor;

        bus.load_standard_roms_from_data_dir();
        let reset_vector = bus.read_word(0xFFFC);
        cpu.reset(reset_vector);

        loop {
            // Apply any pending writes from the debugger
            {
                let mut writes = match pending_writes.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                for (addr, value) in writes.drain(..) {
                    bus.write_byte(addr, value);
                }
            }

            // Apply any pending register writes from the debugger
            {
                let mut reg_writes = match pending_register_writes.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                for (field, value) in reg_writes.drain(..) {
                    match field {
                        crate::debug::RegisterField::A => cpu.registers.a = value as u8,
                        crate::debug::RegisterField::X => cpu.registers.x = value as u8,
                        crate::debug::RegisterField::Y => cpu.registers.y = value as u8,
                        crate::debug::RegisterField::SP => cpu.registers.sp = value as u8,
                        crate::debug::RegisterField::PC => cpu.registers.pc = value,
                        crate::debug::RegisterField::Status => cpu.registers.status = value as u8,
                    }
                }
            }

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

                // Publish registers alongside the frame
                let mut regs = match shared_registers.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                regs.a = cpu.registers.a;
                regs.x = cpu.registers.x;
                regs.y = cpu.registers.y;
                regs.sp = cpu.registers.sp;
                regs.pc = cpu.registers.pc;
                regs.status = cpu.registers.status;
            }

            if last_memory_publish.elapsed() >= MEMORY_PUBLISH_INTERVAL {
                let mut mem = match shared_memory.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                bus.copy_memory_to(&mut mem);
                last_memory_publish = Instant::now();
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
        self.debug.create(event_loop);

        let (handle, state) = Self::spawn_emulator(self.tick_duration);
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
                _ => {
                    self.screen.handle_event(event_loop, event);
                }
            }
        } else if Some(window_id) == self.keyboard.window_id() {
            match event {
                WindowEvent::RedrawRequested => {
                    self.keyboard.draw(event_loop, &mut self.keyboard_state);
                }
                _ => {
                    self.keyboard.handle_event(event_loop, event, &mut self.keyboard_state);
                }
            }
        } else if Some(window_id) == self.debug.window_id() {
            let memory = Arc::clone(&self.shared_state().memory);
            let registers = Arc::clone(&self.shared_state().registers);
            let pending_writes = Arc::clone(&self.shared_state().pending_writes);
            let pending_register_writes = Arc::clone(&self.shared_state().pending_register_writes);
            match event {
                WindowEvent::RedrawRequested => {
                    self.debug.draw(&self.debug_state, &memory, &registers);
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
                    );
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
