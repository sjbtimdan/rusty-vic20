use rusty_vic20::memory::Memory;
use rusty_vic20::screen::Screen;
use rusty_vic20::vic::VIC;
use winit::error::EventLoopError;
use winit::event_loop::EventLoop;

fn main() -> Result<(), EventLoopError> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    let mut screen = Screen::new();
    let mut memory = Memory::default();
    memory.load_standard_roms_from_data_dir();
    let mut vic = VIC::new(&memory);
    vic.set_border_color(4); // purple border
    let framebuffer = vic.render_frame();
    screen.update_framebuffer(&framebuffer);
    event_loop.run_app(&mut screen)
}
