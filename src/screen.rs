use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

#[derive(Default)]
pub struct Screen {
    window: Option<&'static Window>,
    pixels: Option<Pixels<'static>>,
    framebuffer: Vec<u32>,
    width: u32,
}

impl Screen {
    pub fn new(width: u32) -> Self {
        Self { width, ..Default::default() }
    }

    pub fn update_framebuffer(&mut self, framebuffer: &Vec<u32>) {
        self.framebuffer = framebuffer.clone();
    }

    fn draw(&mut self) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let frame = pixels.frame_mut();
        let chunks = frame.chunks_exact_mut(4).zip(self.framebuffer.iter().copied());
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

        let window = event_loop
            .create_window(Window::default_attributes().with_title("Vic 20 Cyan Screen"))
            .expect("failed to create window");

        // Pixels borrows the window for app lifetime, so keep it alive to shutdown.
        let window: &'static Window = Box::leak(Box::new(window));
        let height = self.framebuffer.len() as u32 / self.width;
        let surface_texture = SurfaceTexture::new(self.width, height, window);
        let pixels = Pixels::new(self.width, height, surface_texture)
            .expect("failed to create pixel surface");

        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    let _ = pixels.resize_surface(size.width, size.height);
                }
            }
            _ => {}
        }
    }
}
