use font8x8::{BASIC_FONTS, UnicodeFonts};
use image::{ImageFormat, load_from_memory_with_format};
use log::error;
use pixels::{Pixels, SurfaceTexture};
use std::{sync::Arc, time::Instant};
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::ui::keyboard::{Key, KeyRegion, KeyboardState};

const KEYBOARD_PNG: &[u8] = include_bytes!("../../../data/vic20-c64-layout.png");

#[derive(Copy, Clone)]
struct FrameSize {
    width: usize,
    height: usize,
}

#[derive(Copy, Clone)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Copy, Clone)]
struct Rect {
    origin: Point,
    width: i32,
    height: i32,
}

pub struct KeyboardWindow {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    image_rgba: Vec<u8>,
    image_width: u32,
    image_height: u32,
    cursor_pos: Option<(f64, f64)>,
}

impl Default for KeyboardWindow {
    fn default() -> Self {
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
        }
    }
}

impl KeyboardWindow {
    pub fn create(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let inner_width = self.image_width as f64;
        let inner_height = self.image_height as f64;
        let mut window_attributes = Window::default_attributes()
            .with_title("VIC-20 Keyboard")
            .with_inner_size(LogicalSize::new(inner_width, inner_height))
            .with_min_inner_size(LogicalSize::new(inner_width, inner_height))
            .with_resizable(true);

        if let Some(monitor) = event_loop.available_monitors().next() {
            let sf = monitor.scale_factor();
            let monitor_size = monitor.size().to_logical::<f64>(sf);
            let x = ((monitor_size.width - inner_width) / 2.0).max(0.0);
            let y = ((monitor_size.height / 2.0) + 10.0).max(0.0);
            window_attributes = window_attributes.with_position(LogicalPosition::new(x, y));
        }

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

    pub fn window_id(&self) -> Option<winit::window::WindowId> {
        self.window.as_ref().map(|w| w.id())
    }

    pub fn handle_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent, state: &mut KeyboardState) {
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
                if let Some((img_x, img_y)) = self.window_pos_to_image_pos() {
                    self.handle_mouse_click(img_x, img_y, state);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    for vic_key in keycode_to_vickeys(keycode) {
                        self.handle_physical_key_event(pressed, vic_key, state);
                    }
                }
            }
            WindowEvent::RedrawRequested => self.draw(event_loop, state),
            _ => {}
        }
    }

    pub fn draw(&mut self, event_loop: &ActiveEventLoop, state: &mut KeyboardState) {
        state.tick_flash();

        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let frame_size = FrameSize {
            width: self.image_width as usize,
            height: self.image_height as usize,
        };

        let frame = pixels.frame_mut();
        frame.copy_from_slice(&self.image_rgba);

        let w = frame_size.width;

        if let Some(held) = state.held_key {
            let regions: Vec<_> = state.key_regions.iter().filter(|r| r.label == held).cloned().collect();
            for region in &regions {
                tint_region(frame, w, region, [30, 80, 220], 160);
            }
        }

        let phys: Vec<_> = state
            .key_regions
            .iter()
            .filter(|r| state.physical_keys.contains(&r.label))
            .cloned()
            .collect();
        for region in &phys {
            tint_region(frame, w, region, [60, 180, 255], 150);
        }

        if let Some((flash, _)) = state.flash_key {
            let regions: Vec<_> = state.key_regions.iter().filter(|r| r.label == flash).cloned().collect();
            for region in &regions {
                tint_region(frame, w, region, [120, 200, 255], 130);
            }
        }

        draw_rect(
            frame,
            frame_size,
            Rect {
                origin: Point {
                    x: 8,
                    y: (self.image_height as i32) - 58,
                },
                width: (self.image_width as i32) - 16,
                height: 50,
            },
            [12, 12, 12, 220],
        );

        draw_text(
            frame,
            frame_size,
            Point {
                x: 16,
                y: (self.image_height as i32) - 49,
            },
            &state.status_message.clone(),
            [255, 255, 255, 255],
            2,
        );

        let held = state.held_key.map_or_else(|| "(none)".to_string(), |k| k.to_string());
        draw_text(
            frame,
            frame_size,
            Point {
                x: 16,
                y: (self.image_height as i32) - 28,
            },
            &format!("HELD: {held}"),
            [255, 220, 120, 255],
            2,
        );

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

    pub fn next_deadline(&self, state: &KeyboardState) -> Option<Instant> {
        state.flash_remaining().map(|d| Instant::now() + d)
    }

    fn window_pos_to_image_pos(&self) -> Option<(f32, f32)> {
        let (cursor_x, cursor_y) = self.cursor_pos?;
        let pixels = self.pixels.as_ref()?;
        let (img_x, img_y) = pixels.window_pos_to_pixel((cursor_x as f32, cursor_y as f32)).ok()?;
        Some((img_x as f32, img_y as f32))
    }

    pub fn handle_mouse_click(&mut self, image_x: f32, image_y: f32, state: &mut KeyboardState) {
        if let Some(key) = state.key_at_pixel(image_x, image_y) {
            state.on_key_click(key);
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    pub fn handle_physical_key_event(&mut self, pressed: bool, vic_key: Key, state: &mut KeyboardState) {
        if pressed {
            state.physical_key_pressed(vic_key);
        } else {
            state.physical_key_released(vic_key);
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

/// Blend every pixel in a key region toward `color` by `alpha/255`.
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

fn draw_rect(frame: &mut [u8], frame_size: FrameSize, rect: Rect, color: [u8; 4]) {
    for dy in 0..rect.height {
        let py = rect.origin.y + dy;
        if py < 0 || py >= frame_size.height as i32 {
            continue;
        }
        for dx in 0..rect.width {
            let px = rect.origin.x + dx;
            if px < 0 || px >= frame_size.width as i32 {
                continue;
            }
            let idx = ((py as usize) * frame_size.width + (px as usize)) * 4;
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}

fn draw_text(frame: &mut [u8], frame_size: FrameSize, origin: Point, text: &str, color: [u8; 4], scale: i32) {
    let mut cursor_x = origin.x;
    for ch in text.chars() {
        let c = ch.to_ascii_uppercase();
        if c == '\n' {
            cursor_x = origin.x;
            continue;
        }

        if let Some(glyph) = BASIC_FONTS.get(c) {
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if (bits >> col) & 1 == 1 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cursor_x + (col * scale) + sx;
                                let py = origin.y + (row as i32 * scale) + sy;
                                if px < 0 || py < 0 || px >= frame_size.width as i32 || py >= frame_size.height as i32 {
                                    continue;
                                }
                                let idx = ((py as usize) * frame_size.width + (px as usize)) * 4;
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

fn keycode_to_vickeys(key: KeyCode) -> Vec<Key> {
    match key {
        KeyCode::Backquote => vec![Key::Left],
        KeyCode::Digit1 => vec![Key::Single('1')],
        KeyCode::Digit2 => vec![Key::Single('2')],
        KeyCode::Digit3 => vec![Key::Single('3')],
        KeyCode::Digit4 => vec![Key::Single('4')],
        KeyCode::Digit5 => vec![Key::Single('5')],
        KeyCode::Digit6 => vec![Key::Single('6')],
        KeyCode::Digit7 => vec![Key::Single('7')],
        KeyCode::Digit8 => vec![Key::Single('8')],
        KeyCode::Digit9 => vec![Key::Single('9')],
        KeyCode::Digit0 => vec![Key::Single('0')],
        KeyCode::Minus => vec![Key::Single('-')],
        KeyCode::Equal => vec![Key::Single('+')],
        KeyCode::Tab => vec![Key::RunStop],
        KeyCode::AltLeft => vec![Key::Cbm],
        KeyCode::Backslash => vec![Key::Single('£')],
        KeyCode::Home => vec![Key::ClrHome],
        KeyCode::Backspace => vec![Key::InsDel],
        KeyCode::Delete => vec![Key::InsDel],
        KeyCode::ControlLeft => vec![Key::Ctrl],
        KeyCode::ControlRight => vec![Key::Ctrl],
        KeyCode::KeyQ => vec![Key::Single('Q')],
        KeyCode::KeyW => vec![Key::Single('W')],
        KeyCode::KeyE => vec![Key::Single('E')],
        KeyCode::KeyR => vec![Key::Single('R')],
        KeyCode::KeyT => vec![Key::Single('T')],
        KeyCode::KeyY => vec![Key::Single('Y')],
        KeyCode::KeyU => vec![Key::Single('U')],
        KeyCode::KeyI => vec![Key::Single('I')],
        KeyCode::KeyO => vec![Key::Single('O')],
        KeyCode::KeyP => vec![Key::Single('P')],
        KeyCode::BracketLeft => vec![Key::Single('@')],
        KeyCode::BracketRight => vec![Key::Single('*')],
        KeyCode::CapsLock => vec![Key::ShiftLock],
        KeyCode::KeyA => vec![Key::Single('A')],
        KeyCode::KeyS => vec![Key::Single('S')],
        KeyCode::KeyD => vec![Key::Single('D')],
        KeyCode::KeyF => vec![Key::Single('F')],
        KeyCode::KeyG => vec![Key::Single('G')],
        KeyCode::KeyH => vec![Key::Single('H')],
        KeyCode::KeyJ => vec![Key::Single('J')],
        KeyCode::KeyK => vec![Key::Single('K')],
        KeyCode::KeyL => vec![Key::Single('L')],
        KeyCode::Semicolon => vec![Key::Single('[')],
        KeyCode::Quote => vec![Key::Single(']')],
        KeyCode::Enter => vec![Key::Return],
        KeyCode::ShiftLeft => vec![Key::Shift],
        KeyCode::KeyZ => vec![Key::Single('Z')],
        KeyCode::KeyX => vec![Key::Single('X')],
        KeyCode::KeyC => vec![Key::Single('C')],
        KeyCode::KeyV => vec![Key::Single('V')],
        KeyCode::KeyB => vec![Key::Single('B')],
        KeyCode::KeyN => vec![Key::Single('N')],
        KeyCode::KeyM => vec![Key::Single('M')],
        KeyCode::Comma => vec![Key::Single(',')],
        KeyCode::Period => vec![Key::Single('.')],
        KeyCode::Slash => vec![Key::Single('/')],
        KeyCode::ShiftRight => vec![Key::Shift],
        KeyCode::ArrowUp => vec![Key::CrsrUD, Key::Shift],
        KeyCode::ArrowDown => vec![Key::CrsrUD],
        KeyCode::ArrowLeft => vec![Key::CrsrLR, Key::Shift],
        KeyCode::ArrowRight => vec![Key::CrsrLR],
        KeyCode::Space => vec![Key::Single(' ')],
        KeyCode::F1 => vec![Key::F1F2],
        KeyCode::F2 => vec![Key::Shift, Key::F1F2],
        KeyCode::F3 => vec![Key::F3F4],
        KeyCode::F4 => vec![Key::Shift, Key::F3F4],
        KeyCode::F5 => vec![Key::F5F6],
        KeyCode::F6 => vec![Key::Shift, Key::F5F6],
        KeyCode::F7 => vec![Key::F7F8],
        KeyCode::F8 => vec![Key::Shift, Key::F7F8],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(KeyCode::KeyA, vec![Key::Single('A')])]
    #[case(KeyCode::Digit1, vec![Key::Single('1')])]
    #[case(KeyCode::Space, vec![Key::Single(' ')])]
    #[case(KeyCode::Minus, vec![Key::Single('-')])]
    #[case(KeyCode::Equal, vec![Key::Single('+')])]
    #[case(KeyCode::Comma, vec![Key::Single(',')])]
    fn single_key_mapping(#[case] keycode: KeyCode, #[case] expected: Vec<Key>) {
        assert_eq!(keycode_to_vickeys(keycode), expected);
    }

    #[rstest]
    #[case(KeyCode::ArrowUp, vec![Key::CrsrUD, Key::Shift])]
    #[case(KeyCode::ArrowLeft, vec![Key::CrsrLR, Key::Shift])]
    #[case(KeyCode::ArrowDown, vec![Key::CrsrUD])]
    #[case(KeyCode::ArrowRight, vec![Key::CrsrLR])]
    fn multi_key_mapping(#[case] keycode: KeyCode, #[case] expected: Vec<Key>) {
        assert_eq!(keycode_to_vickeys(keycode), expected);
    }

    #[rstest]
    #[case(KeyCode::F1, vec![Key::F1F2])]
    #[case(KeyCode::F2, vec![Key::Shift, Key::F1F2])]
    fn f1_f2_mapping(#[case] keycode: KeyCode, #[case] expected: Vec<Key>) {
        assert_eq!(keycode_to_vickeys(keycode), expected);
    }

    #[rstest]
    #[case(KeyCode::End)]
    #[case(KeyCode::PageUp)]
    fn unmapped_key_returns_empty(#[case] keycode: KeyCode) {
        assert_eq!(keycode_to_vickeys(keycode), vec![]);
    }
}
