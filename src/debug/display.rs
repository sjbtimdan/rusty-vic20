use font8x8::{BASIC_FONTS, UnicodeFonts};
use log::error;
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::Key,
    window::Window,
};

use super::{DebugMode, DebugState, SharedMemory};

const WINDOW_TITLE: &str = "VIC-20 Debug";

const CHAR_W: i32 = 8;
const CHAR_H: i32 = 8;
const SCALE: i32 = 1;

const MARGIN: i32 = 8;
const ROW_H: i32 = 10;
const HEADER_Y: i32 = MARGIN + ROW_H;
const DATA_START_Y: i32 = HEADER_Y + ROW_H;
const ADDRESS_BAR_Y: i32 = MARGIN;
const ADDR_COL_X: i32 = MARGIN;
const HEX_COL_X: i32 = ADDR_COL_X + 4 * CHAR_W * SCALE + 4;
const ASCII_COL_X: i32 = HEX_COL_X + 48 * CHAR_W * SCALE + 8;

const COLS: usize = 16;
const ROWS: usize = 16;

const PIXEL_WIDTH: u32 = (ASCII_COL_X + 16 * CHAR_W * SCALE + MARGIN) as u32;
const PIXEL_HEIGHT: u32 = 220;

const BG_COLOR: [u8; 4] = [30, 30, 30, 255];
const HEADER_COLOR: [u8; 4] = [100, 100, 100, 255];
const ADDR_COLOR: [u8; 4] = [80, 120, 200, 255];
const HEX_COLOR: [u8; 4] = [200, 200, 200, 255];
const ASCII_COLOR: [u8; 4] = [160, 200, 160, 255];
const HIGHLIGHT_COLOR: [u8; 4] = [60, 60, 100, 255];
const INPUT_BG: [u8; 4] = [50, 50, 50, 255];

#[derive(Default)]
pub struct DebugWindow {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    cursor_pos: Option<(f64, f64)>,
}

impl DebugWindow {
    pub fn create(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let scale: f64 = 1.5;
        let width = PIXEL_WIDTH as f64 * scale;
        let height = PIXEL_HEIGHT as f64 * scale;

        let mut window_attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(width, height))
            .with_min_inner_size(LogicalSize::new(PIXEL_WIDTH as f64, PIXEL_HEIGHT as f64))
            .with_resizable(true);

        if let Some(monitor) = event_loop.available_monitors().next() {
            let sf = monitor.scale_factor();
            let monitor_size = monitor.size().to_logical::<f64>(sf);
            let screen_x = (monitor_size.width - width) / 2.0 + 400.0;
            let screen_y = ((monitor_size.height / 2.0) - height).max(0.0);
            window_attributes = window_attributes.with_position(LogicalPosition::new(screen_x.max(0.0), screen_y));
        }

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create debug window"),
        );

        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels =
            Pixels::new(PIXEL_WIDTH, PIXEL_HEIGHT, surface_texture).expect("failed to create debug pixels surface");

        self.pixels = Some(pixels);
        self.window = Some(window);
    }

    pub fn window_id(&self) -> Option<winit::window::WindowId> {
        self.window.as_ref().map(|w| w.id())
    }

    pub fn handle_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
        state: &mut DebugState,
        memory: &SharedMemory,
        pending_writes: &super::PendingWrites,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut()
                    && let Err(err) = pixels.resize_surface(size.width, size.height)
                {
                    error!("debug resize_surface failed: {err}");
                }
            }
            WindowEvent::RedrawRequested => self.draw(state, memory),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = Some((position.x, position.y));
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_mouse_click(state);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.handle_key(state, &event.logical_key, event.text.as_deref(), pending_writes);
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_click(&self, state: &mut DebugState) {
        let Some((cursor_x, cursor_y)) = self.cursor_pos else {
            return;
        };
        let Some(pixels) = self.pixels.as_ref() else {
            return;
        };
        let Ok((px, py)) = pixels.window_pos_to_pixel((cursor_x as f32, cursor_y as f32)) else {
            return;
        };

        let row = (py as i32 - DATA_START_Y) / ROW_H;
        let col = (px as i32 - HEX_COL_X) / (3 * CHAR_W * SCALE);
        if row >= 0 && row < ROWS as i32 && col >= 0 && col < COLS as i32 {
            state.selected_offset = Some((row as usize) * COLS + col as usize);
        }
    }

    #[allow(clippy::collapsible_else_if)]
    fn handle_key(&self, state: &mut DebugState, key: &Key, text: Option<&str>, pending_writes: &super::PendingWrites) {
        match state.mode {
            DebugMode::Browse => match key {
                Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                    if let Some(offset) = state.selected_offset.as_mut() {
                        *offset = offset.wrapping_sub(16) & 0xFF;
                    } else {
                        state.selected_offset = Some(0);
                    }
                }
                Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                    if let Some(offset) = state.selected_offset.as_mut() {
                        *offset = offset.wrapping_add(16) & 0xFF;
                    } else {
                        state.selected_offset = Some(0);
                    }
                }
                Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                    if let Some(offset) = state.selected_offset.as_mut() {
                        *offset = offset.wrapping_sub(1) & 0xFF;
                    } else {
                        state.selected_offset = Some(0);
                    }
                }
                Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                    if let Some(offset) = state.selected_offset.as_mut() {
                        *offset = offset.wrapping_add(1) & 0xFF;
                    } else {
                        state.selected_offset = Some(0);
                    }
                }
                Key::Named(winit::keyboard::NamedKey::PageUp) => {
                    state.navigate_address(-256);
                }
                Key::Named(winit::keyboard::NamedKey::PageDown) => {
                    state.navigate_address(256);
                }
                _ => {
                    if let Some(text) = text {
                        for c in text.chars() {
                            match c {
                                'g' | 'G' => {
                                    state.mode = DebugMode::EditingAddress;
                                    state.address_input.clear();
                                }
                                'e' | 'E' => {
                                    if state.selected_offset.is_none() {
                                        state.selected_offset = Some(0);
                                    }
                                    state.mode = DebugMode::EditingByte;
                                    state.edit_byte_input.clear();
                                }
                                _ if c.is_ascii_hexdigit() => {
                                    state.mode = DebugMode::EditingAddress;
                                    state.address_input.push(c);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            },
            DebugMode::EditingAddress => match key {
                Key::Named(winit::keyboard::NamedKey::Enter) => {
                    state.commit_address();
                }
                Key::Named(winit::keyboard::NamedKey::Escape) => {
                    state.cancel_input();
                }
                Key::Named(winit::keyboard::NamedKey::Backspace) => {
                    state.address_input.pop();
                }
                _ => {
                    if let Some(text) = text {
                        for c in text.chars() {
                            if c.is_ascii_hexdigit() {
                                state.address_input.push(c);
                            }
                        }
                    }
                }
            },
            DebugMode::EditingByte => match key {
                Key::Named(winit::keyboard::NamedKey::Enter) => {
                    if let Some((addr, val)) = state.commit_byte_edit()
                        && let Ok(mut writes) = pending_writes.lock()
                    {
                        writes.push((addr, val));
                    }
                }
                Key::Named(winit::keyboard::NamedKey::Escape) => {
                    state.cancel_input();
                }
                Key::Named(winit::keyboard::NamedKey::Backspace) => {
                    state.edit_byte_input.pop();
                }
                _ => {
                    if let Some(text) = text {
                        for c in text.chars() {
                            if c.is_ascii_hexdigit() {
                                state.edit_byte_input.push(c);
                            }
                        }
                    }
                }
            },
        }
    }

    pub fn draw(&mut self, state: &DebugState, memory: &SharedMemory) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let frame = pixels.frame_mut();
        fill_rect(frame, PIXEL_WIDTH as usize, PIXEL_HEIGHT as usize, BG_COLOR);

        let mem = match memory.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        draw_address_bar(frame, state);

        for col in 0..COLS {
            let cx = HEX_COL_X + col as i32 * 3 * CHAR_W * SCALE;
            let h = if col < 10 {
                (b'0' + col as u8) as char
            } else {
                (b'A' + (col as u8 - 10)) as char
            };
            draw_char(frame, cx, HEADER_Y, h, HEADER_COLOR);
        }

        for row in 0..ROWS {
            let y = DATA_START_Y + row as i32 * ROW_H;
            let addr = state.start_address.wrapping_add((row * COLS) as u16);
            let addr_str = format!("{:04X}", addr);
            draw_str(frame, ADDR_COL_X, y, &addr_str, ADDR_COLOR);

            for col in 0..COLS {
                let offset = row * COLS + col;
                let byte_addr = addr.wrapping_add(col as u16);
                let byte = mem[byte_addr as usize];
                let cx = HEX_COL_X + col as i32 * 3 * CHAR_W * SCALE;

                let highlight = state.selected_offset == Some(offset);
                if highlight {
                    let px = cx as usize;
                    let py = y as usize;
                    for dy in 0..CHAR_H as usize * SCALE as usize {
                        for dx in 0..(2 * CHAR_W as usize * SCALE as usize) {
                            let idx = ((py + dy) * PIXEL_WIDTH as usize + px + dx) * 4;
                            if idx + 3 < frame.len() {
                                frame[idx] = HIGHLIGHT_COLOR[0];
                                frame[idx + 1] = HIGHLIGHT_COLOR[1];
                                frame[idx + 2] = HIGHLIGHT_COLOR[2];
                                frame[idx + 3] = HIGHLIGHT_COLOR[3];
                            }
                        }
                    }
                }

                let byte_str = format!("{:02X}", byte);
                let color = if highlight { [255u8, 255, 120, 255] } else { HEX_COLOR };
                draw_str(frame, cx, y, &byte_str, color);

                let ascii_char = if (0x20..=0x7E).contains(&byte) {
                    byte as char
                } else {
                    '.'
                };
                let ax = ASCII_COL_X + col as i32 * CHAR_W * SCALE;
                draw_char(frame, ax, y, ascii_char, ASCII_COLOR);
            }
        }

        draw_status_line(frame, state);

        drop(mem);

        if let Err(err) = pixels.render() {
            error!("debug pixels render failed: {err}");
        }
    }

    pub fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn draw_address_bar(frame: &mut [u8], state: &DebugState) {
    let x = ADDR_COL_X;
    let y = ADDRESS_BAR_Y;

    draw_str(frame, x, y, "Addr:", HEADER_COLOR);
    let input_x = x + 5 * CHAR_W * SCALE + 4;

    let bg_w = 4 * CHAR_W * SCALE + 8;
    let bg_h = ROW_H;
    fill_rect_at(frame, PIXEL_WIDTH as usize, input_x, y, bg_w, bg_h, INPUT_BG);

    let input = if state.mode == DebugMode::EditingAddress {
        &state.address_input
    } else {
        ""
    };
    if input.is_empty() && state.mode == DebugMode::EditingAddress {
    } else if !input.is_empty() {
        draw_str(frame, input_x + 2, y, input, [220u8, 220, 100, 255]);
    } else {
        let addr_str = format!("{:04X}", state.start_address);
        draw_str(frame, input_x + 2, y, &addr_str, ADDR_COLOR);
    }
}

fn draw_status_line(frame: &mut [u8], state: &DebugState) {
    let y = DATA_START_Y + ROWS as i32 * ROW_H + 8;
    let x = ADDR_COL_X;

    let msg = match state.mode {
        DebugMode::Browse => {
            if state.selected_offset.is_some() {
                "E: edit byte   Arrows: move cursor   PgUp/PgDn: scroll"
            } else {
                "G: goto addr   Arrows: move cursor   PgUp/PgDn: scroll"
            }
        }
        DebugMode::EditingAddress => "Enter hex address (Enter: confirm, Esc: cancel)",
        DebugMode::EditingByte => "",
    };
    draw_str(frame, x, y, msg, HEADER_COLOR);

    if state.mode == DebugMode::EditingByte {
        let edit_y = y;
        let edit_x = ADDR_COL_X;
        let prefix = "Edit: $";
        draw_str(frame, edit_x, edit_y, prefix, [220u8, 220, 100, 255]);
        let ex = edit_x + prefix.len() as i32 * CHAR_W * SCALE;
        let val = &state.edit_byte_input;
        let bg_w = (2.max(val.len() as i32 + 1)) * CHAR_W * SCALE + 4;
        fill_rect_at(frame, PIXEL_WIDTH as usize, ex, edit_y - 1, bg_w, ROW_H, INPUT_BG);
        if !val.is_empty() {
            draw_str(frame, ex + 2, edit_y, val, [220u8, 220, 100, 255]);
        }
        draw_str(
            frame,
            ex + bg_w + 4,
            edit_y,
            " (Enter: write, Esc: cancel)",
            HEADER_COLOR,
        );
    }
}

fn fill_rect(pixels: &mut [u8], width: usize, height: usize, color: [u8; 4]) {
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
            pixels[idx + 3] = color[3];
        }
    }
}

fn fill_rect_at(pixels: &mut [u8], frame_width: usize, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px >= 0 && py >= 0 && (px as usize) < frame_width {
                let idx = ((py as usize) * frame_width + (px as usize)) * 4;
                if idx + 3 < pixels.len() {
                    pixels[idx] = color[0];
                    pixels[idx + 1] = color[1];
                    pixels[idx + 2] = color[2];
                    pixels[idx + 3] = color[3];
                }
            }
        }
    }
}

fn draw_str(pixels: &mut [u8], x: i32, y: i32, text: &str, color: [u8; 4]) {
    let mut cx = x;
    for ch in text.chars() {
        draw_char(pixels, cx, y, ch, color);
        cx += CHAR_W * SCALE;
    }
}

fn draw_char(pixels: &mut [u8], x: i32, y: i32, ch: char, color: [u8; 4]) {
    let c = ch.to_ascii_uppercase();
    if let Some(glyph) = BASIC_FONTS.get(c) {
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..8 {
                if (bits >> col) & 1 == 1 {
                    for sy in 0..SCALE {
                        for sx in 0..SCALE {
                            let px = x + col * SCALE + sx;
                            let py = y + row as i32 * SCALE + sy;
                            if px >= 0 && py >= 0 && (px as u32) < PIXEL_WIDTH {
                                let idx = ((py as usize) * PIXEL_WIDTH as usize + (px as usize)) * 4;
                                if idx + 3 < pixels.len() {
                                    pixels[idx] = color[0];
                                    pixels[idx + 1] = color[1];
                                    pixels[idx + 2] = color[2];
                                    pixels[idx + 3] = color[3];
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
