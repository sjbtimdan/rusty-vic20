use rusty_vic20::{bus::Bus, screen::Screen};
use winit::{error::EventLoopError, event_loop::EventLoop};

fn main() -> Result<(), EventLoopError> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    let mut bus = Bus::default();
    bus.load_standard_roms_from_data_dir();
    let mut screen = Screen::new();
    bus.vic.set_border_color(4); // purple border
    let framebuffer = bus.vic.render_frame();
    screen.update_framebuffer(&framebuffer);
    bus.cpu.reset(&mut bus.memory);
    screen.set_step_callback(move || bus.step());
    event_loop.run_app(&mut screen)
}
