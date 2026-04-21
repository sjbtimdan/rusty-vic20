use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, interrupt_handler::DefaultInterruptHandler},
    screen::{
        display::{DisplayApp, SharedVideoState},
        renderer::{ACTIVE_HEIGHT, ACTIVE_WIDTH},
    },
};
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use winit::event_loop::EventLoop;

const DEFAULT_TICK_MICROS: u64 = 1;
const FRAME_PUBLISH_INTERVAL: Duration = Duration::from_millis(20);

fn parse_tick_duration() -> Duration {
    let tick_micros = env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TICK_MICROS);

    Duration::from_micros(tick_micros)
}

fn run_vic20_loop(tick_duration: Duration, shared_video_state: Arc<Mutex<SharedVideoState>>) {
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let interrupt_handler = DefaultInterruptHandler;
    let mut last_frame_publish = Instant::now();

    bus.load_standard_roms_from_data_dir();
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);

    loop {
        cpu.step(&mut bus, &interrupt_handler);
        bus.step_devices();

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

fn main() {
    env_logger::init();
    let tick_duration = parse_tick_duration();
    println!(
        "Using tick duration of {:?} ({} microseconds)",
        tick_duration,
        tick_duration.as_micros()
    );

    let shared_video_state = Arc::new(Mutex::new(SharedVideoState {
        screen_rgba: vec![0_u32; ACTIVE_WIDTH * ACTIVE_HEIGHT],
        border_rgba: 0x0044AAFF,
    }));

    let _vic20_thread = thread::Builder::new()
        .name("vic20-core-loop".to_string())
        .spawn({
            let shared_video_state = Arc::clone(&shared_video_state);
            move || run_vic20_loop(tick_duration, shared_video_state)
        })
        .expect("failed to spawn VIC-20 core thread");

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = DisplayApp::default();
    app.set_shared_video_state(Arc::clone(&shared_video_state));
    event_loop.run_app(&mut app).expect("event loop run failed");
}
