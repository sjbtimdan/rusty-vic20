use rusty_vic20::controller::Vic20Controller;
use std::{env, time::Duration};

const DEFAULT_TICK_MICROS: u64 = 0;

fn parse_tick_duration() -> Duration {
    let tick_micros = env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TICK_MICROS);

    Duration::from_micros(tick_micros)
}

fn main() {
    env_logger::init();
    let tick_duration = parse_tick_duration();
    println!(
        "Using tick duration of {:?} ({} microseconds)",
        tick_duration,
        tick_duration.as_micros()
    );

    let mut controller = Vic20Controller::new(tick_duration);
    controller.run();
}
