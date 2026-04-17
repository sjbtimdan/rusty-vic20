use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, interrupt_handler::DefaultInterruptHandler},
    screen::{PAL_HEIGHT, PAL_WIDTH, Screen},
};
use std::env;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};
use winit::{error::EventLoopError, event_loop::EventLoop};

const DEFAULT_TICK_MICROS: u64 = 1;
const TARGET_FRAME_MICROS: u64 = 16_667;

fn parse_tick_duration() -> Duration {
    let tick_micros = env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TICK_MICROS);

    Duration::from_micros(tick_micros)
}

fn frame_duration_from_tick(tick_duration: Duration) -> Duration {
    let tick_micros = tick_duration.as_micros().max(1);
    // Keep frame updates close to 60 Hz while aligning to whole tick intervals.
    let ticks_per_frame = ((TARGET_FRAME_MICROS as u128) + (tick_micros / 2)) / tick_micros;
    let ticks_per_frame = ticks_per_frame.max(1);
    let frame_micros = tick_micros.saturating_mul(ticks_per_frame);

    Duration::from_micros(frame_micros as u64)
}

fn main() {
    env_logger::init();
    let tick_duration = parse_tick_duration();
    println!(
        "Using tick duration of {:?} ({} microseconds)",
        tick_duration,
        tick_duration.as_micros()
    );
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let interrupt_handler = DefaultInterruptHandler;
    bus.load_standard_roms_from_data_dir();
    bus.vic.set_border_color(4); // purple border
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);
    // cpu.add_breakpoint_address(0xE5E8);
    loop {
        cpu.step(&mut bus, &interrupt_handler);
        if !tick_duration.is_zero() {
            thread::sleep(tick_duration);
        }
    }
}

pub fn old_main() -> Result<(), EventLoopError> {
    env_logger::init();
    let tick_duration = parse_tick_duration();
    let frame_duration = frame_duration_from_tick(tick_duration);

    let framebuffer = Arc::new(Mutex::new(vec![0; PAL_WIDTH * PAL_HEIGHT]));
    let running = Arc::new(AtomicBool::new(true));

    let step_thread_framebuffer = Arc::clone(&framebuffer);
    let step_thread_running = Arc::clone(&running);
    let step_thread = thread::spawn(move || {
        let mut bus = Bus::default();
        bus.load_standard_roms_from_data_dir();
        bus.vic.set_border_color(4); // purple border
        // bus.cpu.reset(&mut bus);

        let mut next_frame_deadline = Instant::now();

        while step_thread_running.load(Ordering::Relaxed) {
            let tick_start = Instant::now();
            // bus.step();

            if tick_start >= next_frame_deadline {
                let frame = bus.vic.render_frame();
                {
                    let mut shared_framebuffer = step_thread_framebuffer.lock().expect("framebuffer mutex poisoned");
                    *shared_framebuffer = frame;
                }

                next_frame_deadline += frame_duration;
                if tick_start > next_frame_deadline {
                    next_frame_deadline = tick_start + frame_duration;
                }
            }

            let tick_elapsed = tick_start.elapsed();
            if tick_elapsed < tick_duration {
                // thread::sleep(tick_duration - tick_elapsed);
            }
        }
    });

    let event_loop = EventLoop::new()?;
    let mut screen = Screen::new(Arc::clone(&framebuffer), Arc::clone(&running));
    let run_result = event_loop.run_app(&mut screen);

    running.store(false, Ordering::Relaxed);
    if step_thread.join().is_err() {
        panic!("step thread panicked");
    }

    run_result
}
