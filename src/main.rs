use rusty_vic20::screen::Screen;
use winit::error::EventLoopError;
use winit::event_loop::EventLoop;

fn main() -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;
    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 200;
    let framebuffer = vec![0xAAFFFFEE; (WIDTH * HEIGHT) as usize];
    let mut screen = Screen::new(WIDTH);
    screen.update_framebuffer(&framebuffer);
    event_loop.run_app(&mut screen)
}
