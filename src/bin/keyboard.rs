use font8x8::{BASIC_FONTS, UnicodeFonts};
use image::{ImageFormat, load_from_memory_with_format};
use log::{error, info};
use pixels::{Pixels, SurfaceTexture};
use rusty_vic20::keyboard::{KeyRegion, KeyboardState};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

const KEYBOARD_PNG: &[u8] = include_bytes!("../../data/vic20-c64-layout.png");

struct KeyboardApp {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    image_rgba: Vec<u8>,
    image_width: u32,
    image_height: u32,
    cursor_pos: Option<(f64, f64)>,
    state: KeyboardState,
}

impl KeyboardApp {
    fn new() -> Self {
        let keyboard = load_from_memory_with_format(KEYBOARD_PNG, ImageFormat::Png)
            .expect("failed to decode data/vic20-c64-layout.png")
            .to_rgba8();
        let image_width = keyboard.width();
        let image_height = keyboard.height();

        Self {
            window: None,
            pixels: None,
            image_rgba: keyboard.into_raw(),
            image_width,
            image_height,
            cursor_pos: None,
            state: KeyboardState::new(),
        }
    }

    fn key_at_cursor(&self) -> Option<&'static str> {
        let (cursor_x, cursor_y) = self.cursor_pos?;
        let pixels = self.pixels.as_ref()?;
        // window_pos_to_pixel accounts for the pixels crate's surface scaling and
        // any letterbox/pillarbox, so this is always correct regardless of window size.
        let (image_x, image_y) = pixels.window_pos_to_pixel((cursor_x as f32, cursor_y as f32)).ok()?;
        self.state.key_at_pixel(image_x as f32, image_y as f32)
    }

    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        self.state.tick_flash();

        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let frame = pixels.frame_mut();
        frame.copy_from_slice(&self.image_rgba);

        let w = self.image_width as usize;

        // Solid blue on mouse-held key.
        if let Some(held) = self.state.held_key.clone() {
            let regions: Vec<_> = self
                .state
                .key_regions
                .iter()
                .filter(|r| r.label == held)
                .cloned()
                .collect();
            for region in &regions {
                tint_region(frame, w, region, [30, 80, 220], 160);
            }
        }

        // Light cyan on each physically pressed key.
        let phys: Vec<_> = self
            .state
            .key_regions
            .iter()
            .filter(|r| self.state.physical_keys.contains(r.label))
            .cloned()
            .collect();
        for region in &phys {
            tint_region(frame, w, region, [60, 180, 255], 150);
        }

        // Bright flash on the last mouse-clicked key.
        if let Some((flash, _)) = self.state.flash_key.clone() {
            let regions: Vec<_> = self
                .state
                .key_regions
                .iter()
                .filter(|r| r.label == flash)
                .cloned()
                .collect();
            for region in &regions {
                tint_region(frame, w, region, [120, 200, 255], 130);
            }
        }

        draw_rect(
            frame,
            self.image_width as usize,
            self.image_height as usize,
            8,
            (self.image_height as i32) - 58,
            (self.image_width as i32) - 16,
            50,
            [12, 12, 12, 220],
        );

        draw_text(
            frame,
            self.image_width as usize,
            self.image_height as usize,
            16,
            (self.image_height as i32) - 49,
            &self.state.status_message.clone(),
            [255, 255, 255, 255],
            2,
        );

        let held = self.state.held_key.as_deref().unwrap_or("(none)");
        draw_text(
            frame,
            self.image_width as usize,
            self.image_height as usize,
            16,
            (self.image_height as i32) - 28,
            &format!("HELD: {held}"),
            [255, 220, 120, 255],
            2,
        );

        if let Err(err) = pixels.render() {
            error!("pixels render failed: {err}");
            event_loop.exit();
        }
    }
}

impl ApplicationHandler for KeyboardApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes: WindowAttributes = Window::default_attributes()
            .with_title("VIC-20 Keyboard")
            .with_inner_size(LogicalSize::new(self.image_width as f64, self.image_height as f64))
            .with_min_inner_size(LogicalSize::new(self.image_width as f64, self.image_height as f64))
            .with_resizable(true);

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create keyboard window"),
        );

        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels =
            Pixels::new(self.image_width, self.image_height, surface_texture).expect("failed to create pixels surface");

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
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = Some((position.x, position.y));
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(key) = self.key_at_cursor() {
                    self.state.on_key_click(key);
                    info!("{}", self.state.status_message);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key
                    && let Some(vic_key) = keycode_to_vic20(keycode)
                {
                    match event.state {
                        ElementState::Pressed => {
                            if self.state.physical_key_pressed(vic_key) {
                                info!("{}", self.state.status_message);
                            }
                        }
                        ElementState::Released => {
                            self.state.physical_key_released(vic_key);
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => self.draw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // If a flash is active, wake up as soon as it should expire so we repaint promptly.
        if let Some(remaining) = self.state.flash_remaining() {
            use std::time::Instant;
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + remaining));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

/// Blend every pixel in a key region toward `color` by `alpha/255`.
/// A small inset is applied so the highlight stays inside the key cap border.
fn tint_region(frame: &mut [u8], width: usize, region: &KeyRegion, color: [u8; 3], alpha: u8) {
    const INSET: i32 = 3;
    let x0 = region.x as i32 + INSET;
    let y0 = region.y as i32 + INSET;
    let x1 = (region.x + region.w) as i32 - INSET;
    let y1 = (region.y + region.h) as i32 - INSET;
    let a = alpha as u32;
    for y in y0..y1 {
        for x in x0..x1 {
            if x < 0 || y < 0 || x >= width as i32 {
                continue;
            }
            let idx = (y as usize * width + x as usize) * 4;
            if idx + 3 >= frame.len() {
                continue;
            }
            frame[idx] = ((frame[idx] as u32 * (255 - a) + color[0] as u32 * a) / 255) as u8;
            frame[idx + 1] = ((frame[idx + 1] as u32 * (255 - a) + color[1] as u32 * a) / 255) as u8;
            frame[idx + 2] = ((frame[idx + 2] as u32 * (255 - a) + color[2] as u32 * a) / 255) as u8;
        }
    }
}

fn draw_rect(frame: &mut [u8], width: usize, height: usize, x: i32, y: i32, rect_w: i32, rect_h: i32, color: [u8; 4]) {
    for dy in 0..rect_h {
        let py = y + dy;
        if py < 0 || py >= height as i32 {
            continue;
        }
        for dx in 0..rect_w {
            let px = x + dx;
            if px < 0 || px >= width as i32 {
                continue;
            }
            let idx = ((py as usize) * width + (px as usize)) * 4;
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}

fn draw_text(frame: &mut [u8], width: usize, height: usize, x: i32, y: i32, text: &str, color: [u8; 4], scale: i32) {
    let mut cursor_x = x;
    for ch in text.chars() {
        let c = ch.to_ascii_uppercase();
        if c == '\n' {
            cursor_x = x;
            continue;
        }

        if let Some(glyph) = BASIC_FONTS.get(c) {
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if (bits >> col) & 1 == 1 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cursor_x + (col * scale) + sx;
                                let py = y + (row as i32 * scale) + sy;
                                if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                                    continue;
                                }
                                let idx = ((py as usize) * width + (px as usize)) * 4;
                                frame[idx] = color[0];
                                frame[idx + 1] = color[1];
                                frame[idx + 2] = color[2];
                                frame[idx + 3] = color[3];
                            }
                        }
                    }
                }
            }
        }

        cursor_x += 8 * scale + scale;
    }
}

/// Map a physical key code to the VIC-20 key label it corresponds to.
/// CBM, RUN/STOP, and RESTORE are intentionally omitted (no natural mapping).
fn keycode_to_vic20(key: KeyCode) -> Option<&'static str> {
    Some(match key {
        // ── Number row ────────────────────────────────────────────────────
        KeyCode::Backquote => "LEFT", // ← arrow key
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::Digit0 => "0",
        KeyCode::Minus => "+",         // VIC-20 + is where US - sits
        KeyCode::Equal => "-",         // VIC-20 - is where US = sits
        KeyCode::Backslash => "POUND", // closest available key
        KeyCode::Home => "CLR/HOME",
        KeyCode::Backspace => "INS/DEL",
        KeyCode::Delete => "INS/DEL",

        // ── CTRL / Q row ─────────────────────────────────────────────────
        KeyCode::ControlLeft => "CTRL",
        KeyCode::ControlRight => "CTRL",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyW => "W",
        KeyCode::KeyE => "E",
        KeyCode::KeyR => "R",
        KeyCode::KeyT => "T",
        KeyCode::KeyY => "Y",
        KeyCode::KeyU => "U",
        KeyCode::KeyI => "I",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::BracketLeft => "@",  // VIC-20 @ is where US [ sits
        KeyCode::BracketRight => "*", // VIC-20 * is where US ] sits
        // US \ already used for POUND above
        // VIC-20 UP (↑) arrow key — no great physical home; omit

        // ── RUN/STOP / A row ──────────────────────────────────────────────
        KeyCode::CapsLock => "SHIFT LOCK",
        KeyCode::KeyA => "A",
        KeyCode::KeyS => "S",
        KeyCode::KeyD => "D",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::Semicolon => "[", // VIC-20 [ is where US ; sits
        KeyCode::Quote => "]",     // VIC-20 ] is where US ' sits
        // VIC-20 = key — no remaining physical key in home row; omit
        KeyCode::Enter => "RETURN",

        // ── CBM / Z row ───────────────────────────────────────────────────
        KeyCode::ShiftLeft => "SHIFT",
        KeyCode::KeyZ => "Z",
        KeyCode::KeyX => "X",
        KeyCode::KeyC => "C",
        KeyCode::KeyV => "V",
        KeyCode::KeyB => "B",
        KeyCode::KeyN => "N",
        KeyCode::KeyM => "M",
        KeyCode::Comma => ",",
        KeyCode::Period => ".",
        KeyCode::Slash => "/",
        KeyCode::ShiftRight => "SHIFT",
        KeyCode::ArrowUp => "CRSR UD",
        KeyCode::ArrowDown => "CRSR UD",
        KeyCode::ArrowLeft => "CRSR LR",
        KeyCode::ArrowRight => "CRSR LR",

        // ── Space bar ─────────────────────────────────────────────────────
        KeyCode::Space => "SPACE",

        // ── F-keys ────────────────────────────────────────────────────────
        KeyCode::F1 | KeyCode::F2 => "F1/F2",
        KeyCode::F3 | KeyCode::F4 => "F3/F4",
        KeyCode::F5 | KeyCode::F6 => "F5/F6",
        KeyCode::F7 | KeyCode::F8 => "F7/F8",

        _ => return None,
    })
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = KeyboardApp::new();
    event_loop.run_app(&mut app).expect("event loop run failed");
}
