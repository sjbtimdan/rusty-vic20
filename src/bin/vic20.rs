use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, interrupt_handler::DefaultInterruptHandler},
    screen::display::DisplayApp,
};
use std::env;
use std::thread;
use std::time::Duration;
use winit::event_loop::EventLoop;

const DEFAULT_TICK_MICROS: u64 = 1;

fn parse_tick_duration() -> Duration {
    let tick_micros = env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TICK_MICROS);

    Duration::from_micros(tick_micros)
}

fn run_vic20_loop(tick_duration: Duration) {
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let interrupt_handler = DefaultInterruptHandler;

    bus.load_standard_roms_from_data_dir();
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);

    loop {
        cpu.step(&mut bus, &interrupt_handler);
        bus.step_devices();

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

    let _vic20_thread = thread::Builder::new()
        .name("vic20-core-loop".to_string())
        .spawn(move || run_vic20_loop(tick_duration))
        .expect("failed to spawn VIC-20 core thread");

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = DisplayApp::default();
    let frame = vec![0_u32; 32384];
    app.set_screen_rgba(frame);
    app.set_border_rgba(0x0044AAFF);
    event_loop.run_app(&mut app).expect("event loop run failed");
}
