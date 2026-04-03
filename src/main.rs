use rusty_vic20::screen::Screen;
use rusty_vic20::vic::VIC;
use rusty_vic20::memory::Memory;
use winit::error::EventLoopError;
use winit::event_loop::EventLoop;

fn main() -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;
    let mut screen = Screen::new();
    let memory = Memory::default();
    let mut vic = VIC::new(&memory);
    vic.set_border_color(4); // purple border
    let framebuffer = vic.render_frame();
    screen.update_framebuffer(&framebuffer);
    event_loop.run_app(&mut screen)
}
