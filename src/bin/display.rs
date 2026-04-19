use rusty_vic20::screen::{display::DisplayApp, renderer::*};
use winit::event_loop::EventLoop;

fn build_demo_screen() -> Vec<u32> {
    let mut screen = vec![0x000000FF; ACTIVE_WIDTH * ACTIVE_HEIGHT];

    for y in 0..ACTIVE_HEIGHT {
        for x in 0..ACTIVE_WIDTH {
            let index = y * ACTIVE_WIDTH + x;

            let checker = ((x / 8) + (y / 8)) % 2 == 0;
            screen[index] = if checker { 0x223366FF } else { 0x001122FF };
        }
    }

    // A simple color strip near the top to verify different RGBA values quickly.
    let swatches = [
        0x000000FF, 0xFFFFFFFF, 0x880000FF, 0x00FFFFFF, 0xAA00AAFF, 0x00AA00FF, 0x0000AAFF, 0xAAAA00FF,
    ];
    let swatch_width = ACTIVE_WIDTH / swatches.len();
    for (i, &color) in swatches.iter().enumerate() {
        let x_start = i * swatch_width;
        let x_end = ((i + 1) * swatch_width).min(ACTIVE_WIDTH);
        for y in 8..32 {
            for x in x_start..x_end {
                screen[y * ACTIVE_WIDTH + x] = color;
            }
        }
    }

    // Center rectangle to make the active display area obvious.
    let rect_x0 = ACTIVE_WIDTH / 4;
    let rect_x1 = ACTIVE_WIDTH * 3 / 4;
    let rect_y0 = ACTIVE_HEIGHT / 3;
    let rect_y1 = ACTIVE_HEIGHT * 2 / 3;
    for y in rect_y0..rect_y1 {
        for x in rect_x0..rect_x1 {
            let edge = x == rect_x0 || x + 1 == rect_x1 || y == rect_y0 || y + 1 == rect_y1;
            screen[y * ACTIVE_WIDTH + x] = if edge { 0xFFD700FF } else { 0x334455FF };
        }
    }

    screen
}

fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");

    let mut app = DisplayApp::default();
    app.set_border_rgba(0x0044AAFF);
    let demo_screen = build_demo_screen();
    app.set_screen_rgba(demo_screen);
    event_loop.run_app(&mut app).expect("event loop run failed");
}
