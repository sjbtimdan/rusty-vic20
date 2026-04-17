use pixels::{Pixels, SurfaceTexture};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowId};

pub const PAL_WIDTH: usize = 312;
pub const PAL_HEIGHT: usize = 312;

#[derive(Default)]
pub struct Screen {
    window: Option<&'static Window>,
    pixels: Option<Pixels<'static>>,
    framebuffer: Arc<Mutex<Vec<u32>>>,
    running: Arc<AtomicBool>,
    width: u32,
}

impl Screen {
    pub fn new(framebuffer: Arc<Mutex<Vec<u32>>>, running: Arc<AtomicBool>) -> Self {
        Self {
            framebuffer,
            running,
            width: PAL_WIDTH as u32,
            ..Default::default()
        }
    }

    fn draw(&mut self) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let framebuffer = self.framebuffer.lock().expect("framebuffer mutex poisoned");
        let frame = pixels.frame_mut();
        let chunks = frame.chunks_exact_mut(4).zip(framebuffer.iter().copied());
        for (pixel, rgba) in chunks {
            pixel.copy_from_slice(&rgba.to_be_bytes());
        }

        pixels.render().expect("Render failed")
    }
}

impl ApplicationHandler for Screen {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Vic 20 Emulator")
            .with_inner_size(LogicalSize::new(800.0, 600.0));
        let window = event_loop.create_window(attributes).expect("failed to create window");

        // Pixels borrows the window for app lifetime, so keep it alive to shutdown.
        let window: &'static Window = Box::leak(Box::new(window));
        let height = {
            let framebuffer = self.framebuffer.lock().expect("framebuffer mutex poisoned");
            framebuffer.len() as u32 / self.width
        };
        let surface_texture = SurfaceTexture::new(self.width, height, window);
        let pixels = Pixels::new(self.width, height, surface_texture).expect("failed to create pixel surface");

        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let Some(window) = self.window else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.running.store(false, Ordering::Relaxed);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    pixels
                        .resize_surface(size.width, size.height)
                        .expect("Failed to resize");
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_micros(1)));

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }
}
