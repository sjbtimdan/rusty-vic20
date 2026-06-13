use font8x8::{BASIC_FONTS, UnicodeFonts};
use log::error;
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::Key,
    window::Window,
};

use super::{
    ControlState,
    ControlTab,
    IoAction,
    JoystickAction,
    JoystickDirection,
    MemoryAction,
    MemoryExpansion,
    SharedPerfState,
};

const WINDOW_TITLE: &str = "VIC-20 Performance";

const CHAR_W: i32 = 8;
const CHAR_H: i32 = 8;
const SCALE: i32 = 1;

const MARGIN: i32 = 8;
const ROW_H: i32 = 10;

const TAB_BAR_H: i32 = 16;
const CONTENT_START_Y: i32 = TAB_BAR_H + 4;

const PIXEL_WIDTH: u32 = (IO_BTN_DIRECT_LOAD_X + IO_BTN_DIRECT_LOAD_W + MARGIN) as u32;
const PIXEL_HEIGHT: u32 = 220;

const PERF_VALUE_COLOR: [u8; 4] = [140, 200, 140, 255];

const TAB_PERF_X: i32 = MARGIN;
const TAB_IO_X: i32 = TAB_PERF_X + TAB_W + 12;
const TAB_JOYSTICK_X: i32 = TAB_IO_X + TAB_W + 12;
const TAB_MEMORY_X: i32 = TAB_JOYSTICK_X + TAB_W + 12;
const TAB_W: i32 = 10 * CHAR_W * SCALE;
const TAB_H: i32 = 12;
const TAB_LABEL_Y: i32 = 2;

const TAB_ACTIVE_BG: [u8; 4] = [50, 50, 60, 255];
const TAB_INACTIVE_BG: [u8; 4] = [35, 35, 42, 255];
const TAB_BORDER_COLOR: [u8; 4] = [70, 70, 70, 255];
const TAB_TEXT_COLOR: [u8; 4] = [200, 200, 200, 255];

const IO_SECTION_Y: i32 = CONTENT_START_Y + ROW_H;
const IO_BTN_Y: i32 = IO_SECTION_Y + ROW_H + 4;
const IO_BTN_H: i32 = ROW_H + 4;
const IO_BTN_OPEN_X: i32 = MARGIN;
const IO_BTN_OPEN_W: i32 = 16 * CHAR_W * SCALE;
const IO_BTN_PLAY_X: i32 = IO_BTN_OPEN_X + IO_BTN_OPEN_W + 12;
const IO_BTN_PLAY_W: i32 = 8 * CHAR_W * SCALE;
const IO_BTN_DIRECT_LOAD_X: i32 = IO_BTN_PLAY_X + IO_BTN_PLAY_W + 12;
const IO_BTN_DIRECT_LOAD_W: i32 = 18 * CHAR_W * SCALE;

const JOY_CENTER_X: i32 = PIXEL_WIDTH as i32 / 2;
const JOY_CENTER_Y: i32 = 110;
const JOY_PAD_SIZE: i32 = 16;
const JOY_GAP: i32 = 2;
const JOY_STEP: i32 = JOY_PAD_SIZE + JOY_GAP;

const JOY_PAD_COLOR: [u8; 4] = [50, 50, 70, 255];
const JOY_PAD_ACTIVE_COLOR: [u8; 4] = [90, 90, 140, 255];
const JOY_ARROW_COLOR: [u8; 4] = [200, 200, 200, 255];
const JOY_FIRE_COLOR: [u8; 4] = [160, 50, 40, 255];
const JOY_FIRE_ACTIVE_COLOR: [u8; 4] = [220, 70, 50, 255];

type JoyCell = (Option<JoystickDirection>, char);

// 3x3 grid: (direction or None for fire, arrow glyph)
// Row 0: NW N NE, Row 1: W FIRE E, Row 2: SW S SE
const JOY_GRID: [[JoyCell; 3]; 3] = [
    [
        (Some(JoystickDirection::UpLeft), '\\'),
        (Some(JoystickDirection::Up), '^'),
        (Some(JoystickDirection::UpRight), '/'),
    ],
    [
        (Some(JoystickDirection::Left), '<'),
        (None, 'o'),
        (Some(JoystickDirection::Right), '>'),
    ],
    [
        (Some(JoystickDirection::DownLeft), '/'),
        (Some(JoystickDirection::Down), 'v'),
        (Some(JoystickDirection::DownRight), '\\'),
    ],
];

fn joy_cell_x(col: usize) -> i32 {
    JOY_CENTER_X + (col as i32 - 1) * JOY_STEP
}
fn joy_cell_y(row: usize) -> i32 {
    JOY_CENTER_Y + (row as i32 - 1) * JOY_STEP
}

const JOY_CHECKBOX_X: i32 = MARGIN;
const JOY_CHECKBOX_Y: i32 = 185;
const JOY_CHECKBOX_SIZE: i32 = 12;

const BTN_COLOR: [u8; 4] = [55, 55, 80, 255];
const BTN_TEXT_COLOR: [u8; 4] = [220, 220, 220, 255];

const RADIO_SIZE: i32 = 10;
const RADIO_INNER: i32 = 4;
const RADIO_SPACING: i32 = 14;
const RADIO_COLOR: [u8; 4] = [180, 180, 180, 255];
const RADIO_FILL_COLOR: [u8; 4] = [140, 200, 140, 255];

const MEM_HEADER_Y: i32 = CONTENT_START_Y + ROW_H;
const MEM_RADIO_X: i32 = MARGIN;
const MEM_RADIO_START_Y: i32 = MEM_HEADER_Y + ROW_H + 4;
const MEM_RADIO_LABEL_X: i32 = MEM_RADIO_X + RADIO_SIZE + 8;
const MEM_BTN_Y: i32 = MEM_RADIO_START_Y + 5 * RADIO_SPACING + ROW_H;
const MEM_BTN_W: i32 = 12 * CHAR_W * SCALE;
const MEM_BTN_H: i32 = ROW_H + 4;

const MEM_RADIO_OPTIONS: [(MemoryExpansion, &str); 5] = [
    (MemoryExpansion::None, "None"),
    (MemoryExpansion::ThreeK, "3K"),
    (MemoryExpansion::EightK, "8K"),
    (MemoryExpansion::SixteenK, "16K"),
    (MemoryExpansion::ThirtyTwoK, "32K"),
];

const BG_COLOR: [u8; 4] = [30, 30, 30, 255];
const HEADER_COLOR: [u8; 4] = [100, 100, 100, 255];

#[derive(Default)]
pub struct ControlWindow {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    cursor_pos: Option<(f64, f64)>,
    mouse_down: bool,
}

impl ControlWindow {
    pub fn create(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let scale: f64 = 2.0;
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
                .expect("failed to create performance window"),
        );

        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels = Pixels::new(PIXEL_WIDTH, PIXEL_HEIGHT, surface_texture)
            .expect("failed to create performance pixels surface");

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
        state: &mut ControlState,
        perf: &SharedPerfState,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut()
                    && let Err(err) = pixels.resize_surface(size.width, size.height)
                {
                    error!("perf resize_surface failed: {err}");
                }
            }
            WindowEvent::RedrawRequested => self.draw(state, perf),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = Some((position.x, position.y));
                if self.mouse_down {
                    self.update_joystick_from_cursor(state);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: element_state,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_mouse(state, element_state);
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard(state, &event);
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, state: &mut ControlState, element_state: ElementState) {
        let Some((cursor_x, cursor_y)) = self.cursor_pos else {
            return;
        };
        let Some(pixels) = self.pixels.as_ref() else {
            return;
        };
        let Ok((px, py)) = pixels.window_pos_to_pixel((cursor_x as f32, cursor_y as f32)) else {
            return;
        };

        match element_state {
            ElementState::Pressed => {
                self.mouse_down = true;
                if state.current_tab == ControlTab::Joystick {
                    apply_joystick_cell(state, px as i32, py as i32);
                }
            }
            ElementState::Released => {
                self.mouse_down = false;
                state.joystick_direction = None;
                state.joystick_fire = false;
                if state.current_tab == ControlTab::Joystick {
                    state.joystick_action_pending = Some(JoystickAction::StateChanged);
                }

                if let Some(tab) = tab_at(px as i32, py as i32) {
                    if tab != state.current_tab {
                        state.current_tab = tab;
                    }
                    return;
                }

                match state.current_tab {
                    ControlTab::Perf => {}
                    ControlTab::Io => {
                        if let Some(action) = io_button_at(px as i32, py as i32) {
                            state.io_action_pending = Some(action);
                        }
                    }
                    ControlTab::Joystick => {
                        if px as i32 >= JOY_CHECKBOX_X
                            && (px as i32) < JOY_CHECKBOX_X + 220
                            && py as i32 >= JOY_CHECKBOX_Y
                            && (py as i32) < JOY_CHECKBOX_Y + JOY_CHECKBOX_SIZE
                        {
                            state.use_arrow_keys = !state.use_arrow_keys;
                        }
                    }
                    ControlTab::Memory => {
                        if let Some(action) = memory_action_at(px as i32, py as i32) {
                            state.memory_action_pending = Some(action);
                        }
                    }
                }
            }
        }
    }
}

fn mask_to_direction(mask: u8) -> Option<JoystickDirection> {
    match mask {
        0b0001 => Some(JoystickDirection::Up),
        0b0010 => Some(JoystickDirection::Down),
        0b0100 => Some(JoystickDirection::Left),
        0b1000 => Some(JoystickDirection::Right),
        0b0101 => Some(JoystickDirection::UpLeft),
        0b1001 => Some(JoystickDirection::UpRight),
        0b0110 => Some(JoystickDirection::DownLeft),
        0b1010 => Some(JoystickDirection::DownRight),
        _ => None,
    }
}

fn apply_joystick_cell(state: &mut ControlState, px: i32, py: i32) -> bool {
    for (row, cells) in JOY_GRID.iter().enumerate() {
        for (col, &(dir, _)) in cells.iter().enumerate() {
            let x = joy_cell_x(col);
            let y = joy_cell_y(row);
            if px >= x && px < x + JOY_PAD_SIZE && py >= y && py < y + JOY_PAD_SIZE {
                match dir {
                    Some(d) => {
                        state.joystick_direction = Some(d);
                        state.joystick_fire = false;
                    }
                    None => {
                        state.joystick_direction = None;
                        state.joystick_fire = true;
                    }
                }
                state.joystick_action_pending = Some(JoystickAction::StateChanged);
                return true;
            }
        }
    }
    false
}

impl ControlWindow {
    fn update_joystick_from_cursor(&self, state: &mut ControlState) {
        if state.current_tab != ControlTab::Joystick {
            return;
        }
        let Some((cx, cy)) = self.cursor_pos else {
            return;
        };
        let Some(pixels) = self.pixels.as_ref() else {
            return;
        };
        let Ok((px, py)) = pixels.window_pos_to_pixel((cx as f32, cy as f32)) else {
            return;
        };

        if !apply_joystick_cell(state, px as i32, py as i32) {
            state.joystick_direction = None;
            state.joystick_fire = false;
        }
    }

    fn handle_keyboard(&self, state: &mut ControlState, event: &KeyEvent) {
        if !state.use_arrow_keys {
            return;
        }
        if event.state == ElementState::Pressed {
            match event.logical_key {
                Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                    state.arrow_keys_mask |= 0b0001;
                    state.joystick_direction = mask_to_direction(state.arrow_keys_mask);
                    state.joystick_action_pending = Some(JoystickAction::StateChanged);
                }
                Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                    state.arrow_keys_mask |= 0b0010;
                    state.joystick_direction = mask_to_direction(state.arrow_keys_mask);
                    state.joystick_action_pending = Some(JoystickAction::StateChanged);
                }
                Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                    state.arrow_keys_mask |= 0b0100;
                    state.joystick_direction = mask_to_direction(state.arrow_keys_mask);
                    state.joystick_action_pending = Some(JoystickAction::StateChanged);
                }
                Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                    state.arrow_keys_mask |= 0b1000;
                    state.joystick_direction = mask_to_direction(state.arrow_keys_mask);
                    state.joystick_action_pending = Some(JoystickAction::StateChanged);
                }
                Key::Named(winit::keyboard::NamedKey::Space) => {
                    state.joystick_fire = true;
                    state.joystick_action_pending = Some(JoystickAction::StateChanged);
                }
                _ => {}
            }
        } else {
            match event.logical_key {
                Key::Named(winit::keyboard::NamedKey::ArrowUp) => state.arrow_keys_mask &= !0b0001,
                Key::Named(winit::keyboard::NamedKey::ArrowDown) => state.arrow_keys_mask &= !0b0010,
                Key::Named(winit::keyboard::NamedKey::ArrowLeft) => state.arrow_keys_mask &= !0b0100,
                Key::Named(winit::keyboard::NamedKey::ArrowRight) => state.arrow_keys_mask &= !0b1000,
                Key::Named(winit::keyboard::NamedKey::Space) => state.joystick_fire = false,
                _ => return,
            }
            state.joystick_direction = mask_to_direction(state.arrow_keys_mask);
            state.joystick_action_pending = Some(JoystickAction::StateChanged);
        }
    }

    pub fn draw(&mut self, state: &ControlState, perf: &SharedPerfState) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let frame = pixels.frame_mut();
        fill_rect(frame, PIXEL_WIDTH as usize, PIXEL_HEIGHT as usize, BG_COLOR);

        draw_tab_bar(frame, state.current_tab);

        match state.current_tab {
            ControlTab::Perf => draw_perf_tab(frame, perf),
            ControlTab::Io => draw_io_tab(frame, state),
            ControlTab::Joystick => draw_joystick_tab(frame, state),
            ControlTab::Memory => draw_memory_tab(frame, state),
        }

        if let Err(err) = pixels.render() {
            error!("perf pixels render failed: {err}");
        }
    }

    pub fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn draw_tab_bar(frame: &mut [u8], active_tab: ControlTab) {
    for y in 0..TAB_BAR_H {
        for x in 0..PIXEL_WIDTH as i32 {
            let idx = (y as usize * PIXEL_WIDTH as usize + x as usize) * 4;
            if idx + 3 < frame.len() {
                frame[idx] = 25;
                frame[idx + 1] = 25;
                frame[idx + 2] = 30;
                frame[idx + 3] = 255;
            }
        }
    }

    for y in (TAB_BAR_H - 1)..TAB_BAR_H {
        for x in 0..PIXEL_WIDTH as i32 {
            let idx = (y as usize * PIXEL_WIDTH as usize + x as usize) * 4;
            if idx + 3 < frame.len() {
                frame[idx] = TAB_BORDER_COLOR[0];
                frame[idx + 1] = TAB_BORDER_COLOR[1];
                frame[idx + 2] = TAB_BORDER_COLOR[2];
                frame[idx + 3] = TAB_BORDER_COLOR[3];
            }
        }
    }

    let tabs = [
        (ControlTab::Perf, "Perf", TAB_PERF_X),
        (ControlTab::Io, "I/O", TAB_IO_X),
        (ControlTab::Joystick, "JOYSTICK", TAB_JOYSTICK_X),
        (ControlTab::Memory, "MEMORY", TAB_MEMORY_X),
    ];
    for &(tab, label, tx) in &tabs {
        let bg = if active_tab == tab {
            TAB_ACTIVE_BG
        } else {
            TAB_INACTIVE_BG
        };
        fill_rect_at(frame, PIXEL_WIDTH as usize, tx, 0, TAB_W, TAB_H, bg);
        let text_color = if active_tab == tab {
            [255u8, 255, 255, 255]
        } else {
            TAB_TEXT_COLOR
        };
        let text_x = tx + (TAB_W - label.len() as i32 * CHAR_W * SCALE) / 2;
        draw_str(frame, text_x, TAB_LABEL_Y, label, text_color);
    }
}

fn draw_perf_tab(frame: &mut [u8], perf: &SharedPerfState) {
    let metrics = match perf.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    let cycles_str = if metrics.cycles_per_second >= 1_000_000.0 {
        format!("{:.2} MHz", metrics.cycles_per_second / 1_000_000.0)
    } else if metrics.cycles_per_second >= 1_000.0 {
        format!("{:.1} KHz", metrics.cycles_per_second / 1_000.0)
    } else {
        format!("{:.0} Hz", metrics.cycles_per_second)
    };

    let fps_str = format!("{:.1}", metrics.frames_per_second);
    let total_cycles_str = format_total(metrics.total_cycles);
    let total_frames_str = format_total(metrics.total_frames);

    let heading = "Performance";
    let line1 = format!("CPU: {cycles_str}");
    let line2 = format!("FPS: {fps_str}");
    let line3 = format!("Total: {total_cycles_str} cycles, {total_frames_str} frames");

    let y = CONTENT_START_Y + ROW_H;
    draw_str(frame, MARGIN, y, heading, HEADER_COLOR);
    draw_str(frame, MARGIN, y + ROW_H + 4, &line1, PERF_VALUE_COLOR);
    draw_str(frame, MARGIN, y + 2 * (ROW_H + 4), &line2, PERF_VALUE_COLOR);
    draw_str(frame, MARGIN, y + 3 * (ROW_H + 4), &line3, PERF_VALUE_COLOR);
}

fn draw_io_tab(frame: &mut [u8], state: &ControlState) {
    draw_str(frame, MARGIN, IO_SECTION_Y, "Cassette Tape", HEADER_COLOR);

    fill_rect_at(
        frame,
        PIXEL_WIDTH as usize,
        IO_BTN_OPEN_X,
        IO_BTN_Y,
        IO_BTN_OPEN_W,
        IO_BTN_H,
        BTN_COLOR,
    );
    draw_str(frame, IO_BTN_OPEN_X + CHAR_W, IO_BTN_Y + 2, "Open File", BTN_TEXT_COLOR);

    fill_rect_at(
        frame,
        PIXEL_WIDTH as usize,
        IO_BTN_PLAY_X,
        IO_BTN_Y,
        IO_BTN_PLAY_W,
        IO_BTN_H,
        BTN_COLOR,
    );
    let play_text = if state.cassette_playing { "Stop" } else { "Play" };
    draw_str(frame, IO_BTN_PLAY_X + CHAR_W, IO_BTN_Y + 2, play_text, BTN_TEXT_COLOR);

    fill_rect_at(
        frame,
        PIXEL_WIDTH as usize,
        IO_BTN_DIRECT_LOAD_X,
        IO_BTN_Y,
        IO_BTN_DIRECT_LOAD_W,
        IO_BTN_H,
        BTN_COLOR,
    );
    draw_str(
        frame,
        IO_BTN_DIRECT_LOAD_X + CHAR_W,
        IO_BTN_Y + 2,
        "Direct Load (.prg)",
        BTN_TEXT_COLOR,
    );

    let info_y = IO_BTN_Y + IO_BTN_H + ROW_H;
    if let Some(ref path) = state.cassette_file {
        let fname = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        draw_str(frame, MARGIN, info_y, fname, [140u8, 160, 200, 255]);
    } else {
        draw_str(frame, MARGIN, info_y, "(no file)", HEADER_COLOR);
    }

    let status = if state.cassette_playing { "Playing" } else { "Stopped" };
    draw_str(frame, MARGIN, info_y + ROW_H, status, PERF_VALUE_COLOR);
}

fn draw_joystick_tab(frame: &mut [u8], state: &ControlState) {
    draw_str(frame, MARGIN, CONTENT_START_Y + ROW_H, "Joystick", HEADER_COLOR);

    let rw = PIXEL_WIDTH as usize;

    for (row, cells) in JOY_GRID.iter().enumerate() {
        for (col, &(dir, label)) in cells.iter().enumerate() {
            let x = joy_cell_x(col);
            let y = joy_cell_y(row);

            let active = match dir {
                Some(d) => state.joystick_direction == Some(d),
                None => state.joystick_fire,
            };

            let color = if active {
                if dir.is_some() {
                    JOY_PAD_ACTIVE_COLOR
                } else {
                    JOY_FIRE_ACTIVE_COLOR
                }
            } else if dir.is_none() {
                JOY_FIRE_COLOR
            } else {
                JOY_PAD_COLOR
            };
            fill_rect_at(frame, rw, x, y, JOY_PAD_SIZE, JOY_PAD_SIZE, color);
            let cx = x + (JOY_PAD_SIZE - CHAR_W) / 2;
            let cy = y + (JOY_PAD_SIZE - CHAR_H) / 2;
            draw_char(frame, cx, cy, label, JOY_ARROW_COLOR);
        }
    }

    draw_checkbox(frame, JOY_CHECKBOX_X, JOY_CHECKBOX_Y, state.use_arrow_keys);
    draw_str(
        frame,
        JOY_CHECKBOX_X + JOY_CHECKBOX_SIZE + 6,
        JOY_CHECKBOX_Y + 2,
        "Arrow keys as joystick",
        BTN_TEXT_COLOR,
    );
}

fn draw_checkbox(frame: &mut [u8], x: i32, y: i32, checked: bool) {
    let color = [180u8, 180, 180, 255];
    let size = JOY_CHECKBOX_SIZE;
    for dx in 0..size {
        set_pixel(frame, x + dx, y, color);
        set_pixel(frame, x + dx, y + size - 1, color);
    }
    for dy in 0..size {
        set_pixel(frame, x, y + dy, color);
        set_pixel(frame, x + size - 1, y + dy, color);
    }
    if checked {
        for dy in 2..size - 2 {
            for dx in 2..size - 2 {
                set_pixel(frame, x + dx, y + dy, color);
            }
        }
    }
}

fn set_pixel(frame: &mut [u8], x: i32, y: i32, color: [u8; 4]) {
    if x >= 0 && y >= 0 && (x as u32) < PIXEL_WIDTH {
        let idx = (y as usize * PIXEL_WIDTH as usize + x as usize) * 4;
        if idx + 3 < frame.len() {
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
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
    let lookup = if ch.is_ascii() { ch.to_ascii_uppercase() } else { ch };
    if let Some(glyph) = BASIC_FONTS.get(lookup) {
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

fn tab_at(px: i32, py: i32) -> Option<ControlTab> {
    if !(0..TAB_BAR_H).contains(&py) {
        return None;
    }
    if (TAB_PERF_X..TAB_PERF_X + TAB_W).contains(&px) {
        Some(ControlTab::Perf)
    } else if (TAB_IO_X..TAB_IO_X + TAB_W).contains(&px) {
        Some(ControlTab::Io)
    } else if (TAB_JOYSTICK_X..TAB_JOYSTICK_X + TAB_W).contains(&px) {
        Some(ControlTab::Joystick)
    } else if (TAB_MEMORY_X..TAB_MEMORY_X + TAB_W).contains(&px) {
        Some(ControlTab::Memory)
    } else {
        None
    }
}

fn io_button_at(px: i32, py: i32) -> Option<IoAction> {
    if !(IO_BTN_Y..IO_BTN_Y + IO_BTN_H).contains(&py) {
        return None;
    }
    if (IO_BTN_OPEN_X..IO_BTN_OPEN_X + IO_BTN_OPEN_W).contains(&px) {
        Some(IoAction::OpenFile)
    } else if (IO_BTN_PLAY_X..IO_BTN_PLAY_X + IO_BTN_PLAY_W).contains(&px) {
        Some(IoAction::TogglePlay)
    } else if (IO_BTN_DIRECT_LOAD_X..IO_BTN_DIRECT_LOAD_X + IO_BTN_DIRECT_LOAD_W).contains(&px) {
        Some(IoAction::DirectLoad)
    } else {
        None
    }
}

fn draw_memory_tab(frame: &mut [u8], state: &ControlState) {
    draw_str(frame, MARGIN, MEM_HEADER_Y, "Memory Expansion", HEADER_COLOR);

    for (i, &(exp, label)) in MEM_RADIO_OPTIONS.iter().enumerate() {
        let y = MEM_RADIO_START_Y + i as i32 * RADIO_SPACING;
        let selected = state.memory_expansion == exp;
        draw_radio(frame, MEM_RADIO_X, y, RADIO_SIZE, selected);
        draw_str(frame, MEM_RADIO_LABEL_X, y + 1, label, RADIO_COLOR);
    }

    fill_rect_at(
        frame,
        PIXEL_WIDTH as usize,
        MARGIN,
        MEM_BTN_Y,
        MEM_BTN_W,
        MEM_BTN_H,
        BTN_COLOR,
    );
    draw_str(frame, MARGIN + CHAR_W, MEM_BTN_Y + 2, "Reboot", BTN_TEXT_COLOR);
}

fn draw_radio(frame: &mut [u8], x: i32, y: i32, size: i32, selected: bool) {
    let color = RADIO_COLOR;
    for dx in 0..size {
        set_pixel(frame, x + dx, y, color);
        set_pixel(frame, x + dx, y + size - 1, color);
    }
    for dy in 0..size {
        set_pixel(frame, x, y + dy, color);
        set_pixel(frame, x + size - 1, y + dy, color);
    }
    if selected {
        let inner = RADIO_INNER;
        let offset = (size - inner) / 2;
        for dy in 0..inner {
            for dx in 0..inner {
                set_pixel(frame, x + offset + dx, y + offset + dy, RADIO_FILL_COLOR);
            }
        }
    }
}

fn memory_action_at(px: i32, py: i32) -> Option<MemoryAction> {
    for (i, &(exp, _)) in MEM_RADIO_OPTIONS.iter().enumerate() {
        let y = MEM_RADIO_START_Y + i as i32 * RADIO_SPACING;
        if (y..y + RADIO_SIZE).contains(&py) && (MEM_RADIO_X..MEM_RADIO_X + RADIO_SIZE + 96).contains(&px) {
            return Some(MemoryAction::SetExpansion(exp));
        }
    }
    if (MEM_BTN_Y..MEM_BTN_Y + MEM_BTN_H).contains(&py) && (MARGIN..MARGIN + MEM_BTN_W).contains(&px) {
        return Some(MemoryAction::Reboot);
    }
    None
}

fn format_total(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{}", n)
    }
}
