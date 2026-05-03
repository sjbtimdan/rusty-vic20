use log::error;
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::Window,
};

use crate::screen::renderer::{PAL_HEIGHT, PAL_WIDTH, display_vic20_screen};

pub struct SharedVideoState {
    pub screen_rgba: Vec<u32>,
    pub border_rgba: u32,
}

#[derive(Default)]
pub struct ScreenWindow {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
}

impl ScreenWindow {
    pub fn create(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let scale: f64 = 3.0;
        let inner_width = PAL_WIDTH as f64 * scale;
        let inner_height = PAL_HEIGHT as f64 * scale;
        let mut window_attributes = Window::default_attributes()
            .with_title("VIC-20")
            .with_inner_size(LogicalSize::new(inner_width, inner_height))
            .with_min_inner_size(LogicalSize::new(PAL_WIDTH as f64, PAL_HEIGHT as f64));

        if let Some(monitor) = event_loop.available_monitors().next() {
            let sf = monitor.scale_factor();
            let monitor_size = monitor.size().to_logical::<f64>(sf);
            let x = ((monitor_size.width - inner_width) / 2.0).max(0.0);
            let y = ((monitor_size.height / 2.0) - inner_height - 10.0).max(0.0);
            window_attributes = window_attributes.with_position(LogicalPosition::new(x, y));
        }

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create display window"),
        );

        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels =
            Pixels::new(PAL_WIDTH as u32, PAL_HEIGHT as u32, surface_texture).expect("failed to create pixels surface");

        self.pixels = Some(pixels);
        self.window = Some(window);
    }

    pub fn window_id(&self) -> Option<winit::window::WindowId> {
        self.window.as_ref().map(|w| w.id())
    }

    pub fn handle_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut()
                    && let Err(err) = pixels.resize_surface(size.width, size.height)
                {
                    error!("resize_surface failed: {err}");
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                // Drawing is driven by the controller which passes shared state.
                // If somehow we get a redraw before we have pixels, just ignore.
            }
            _ => {}
        }
    }

    pub fn draw(&mut self, event_loop: &ActiveEventLoop, shared: &SharedVideoState) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let frame = pixels.frame_mut();
        display_vic20_screen(frame, shared.border_rgba, &shared.screen_rgba);

        if let Err(err) = pixels.render() {
            error!("pixels render failed: {err}");
            event_loop.exit();
        }
    }

    pub fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
