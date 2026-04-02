use rusty_vic20::screen::Screen;
use winit::error::EventLoopError;
use winit::event_loop::EventLoop;

fn main() -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;
    let mut app = Screen::default();
    event_loop.run_app(&mut app)
}
