use log::error;
use pixels::{Pixels, SurfaceTexture};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{Window, WindowAttributes},
};

use crate::screen::renderer::{ACTIVE_HEIGHT, ACTIVE_WIDTH, PAL_HEIGHT, PAL_WIDTH, display_vic20_screen};

const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / 50);

pub struct SharedVideoState {
    pub screen_rgba: Vec<u32>,
    pub border_rgba: u32,
}

#[derive(Default)]
pub struct DisplayApp {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    shared_video_state: Option<Arc<Mutex<SharedVideoState>>>,
    screen_rgba: Vec<u32>,
    border_rgba: u32,
}

impl DisplayApp {
    pub fn set_shared_video_state(&mut self, shared_video_state: Arc<Mutex<SharedVideoState>>) {
        self.shared_video_state = Some(shared_video_state);
    }

    pub fn set_border_rgba(&mut self, border_rgba: u32) {
        self.border_rgba = border_rgba;
    }

    pub fn set_screen_rgba(&mut self, screen_rgba: Vec<u32>) {
        let expected = ACTIVE_WIDTH * ACTIVE_HEIGHT;
        assert_eq!(
            screen_rgba.len(),
            expected,
            "screen_rgba must match the VIC-20 active pixel area"
        );
        self.screen_rgba = screen_rgba;
    }

    pub fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for DisplayApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let scale: f64 = 3.0;
        let window_attributes: WindowAttributes = Window::default_attributes()
            .with_title("VIC-20")
            .with_inner_size(LogicalSize::new(PAL_WIDTH as f64 * scale, PAL_HEIGHT as f64 * scale))
            .with_min_inner_size(LogicalSize::new(PAL_WIDTH as f64, PAL_HEIGHT as f64));

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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: winit::window::WindowId, event: WindowEvent) {
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
                let Some(pixels) = self.pixels.as_mut() else {
                    return;
                };

                if let Some(shared_video_state) = self.shared_video_state.as_ref() {
                    let state = match shared_video_state.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };

                    self.border_rgba = state.border_rgba;
                    if self.screen_rgba.len() == state.screen_rgba.len() {
                        self.screen_rgba.copy_from_slice(&state.screen_rgba);
                    } else {
                        self.screen_rgba.clone_from(&state.screen_rgba);
                    }
                }

                let frame = pixels.frame_mut();
                display_vic20_screen(frame, self.border_rgba, &self.screen_rgba);

                if let Err(err) = pixels.render() {
                    error!("pixels render failed: {err}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + FRAME_TIME));
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
