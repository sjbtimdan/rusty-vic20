use log::error;
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::Window,
};

use crate::ui::screen::renderer::{BORDER_LEFT, BORDER_RIGHT, PAL_HEIGHT, render_vic20_screen};

#[derive(Default)]
pub struct SharedVideoState {
    pub screen_rgba: Vec<u8>,
    pub border_rgba: [u8; 4],
    pub active_width: usize,
}

#[derive(Default)]
pub struct ScreenWindow {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    pal_width: usize,
}

impl ScreenWindow {
    pub fn create(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let scale: f64 = 3.0;
        self.create_with_active_width(event_loop, 22, scale);
    }

    fn create_with_active_width(&mut self, event_loop: &ActiveEventLoop, active_width_chars: usize, scale: f64) {
        let active_width = active_width_chars * crate::ui::screen::renderer::CHAR_WIDTH;
        let pal_width = active_width + BORDER_LEFT + BORDER_RIGHT;
        let inner_width = pal_width as f64 * scale;
        let inner_height = PAL_HEIGHT as f64 * scale;
        let mut window_attributes = Window::default_attributes()
            .with_title("VIC-20")
            .with_inner_size(LogicalSize::new(inner_width, inner_height))
            .with_min_inner_size(LogicalSize::new(pal_width as f64, PAL_HEIGHT as f64));

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
            Pixels::new(pal_width as u32, PAL_HEIGHT as u32, surface_texture).expect("failed to create pixels surface");

        self.pixels = Some(pixels);
        self.window = Some(window);
        self.pal_width = pal_width;
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
        let pal_width = shared.active_width + BORDER_LEFT + BORDER_RIGHT;
        if self.pal_width != pal_width || self.pixels.is_none() {
            self.pixels = None;
            self.window = None;
            self.pal_width = 0;
            self.create_with_active_width(
                event_loop,
                shared.active_width / crate::ui::screen::renderer::CHAR_WIDTH,
                3.0,
            );
            return;
        }

        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let frame = pixels.frame_mut();
        render_vic20_screen(frame, &shared.border_rgba, &shared.screen_rgba, shared.active_width);

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
