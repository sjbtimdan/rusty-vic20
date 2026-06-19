use super::{
    BrakeSpeed,
    display::{
        BTN_COLOR,
        BTN_TEXT_COLOR,
        CHAR_W,
        HEADER_COLOR,
        MARGIN,
        PIXEL_WIDTH,
        ROW_H,
        SCALE,
        draw_raised_rect,
        draw_str,
    },
};

const BTN_ACTIVE_COLOR: [u8; 4] = [70, 130, 70, 255];

const BRAKE_SECTION_Y: i32 = 86;
const BRAKE_BTN_Y: i32 = BRAKE_SECTION_Y + ROW_H + 4;
const BRAKE_BTN_W: i32 = 7 * CHAR_W * SCALE;
const BRAKE_BTN_H: i32 = ROW_H + 4;
const BRAKE_BTN_GAP: i32 = 6;

const BRAKE_BTNS_TOTAL_W: i32 = 5 * BRAKE_BTN_W + 4 * BRAKE_BTN_GAP;
const BRAKE_BTNS_START_X: i32 = (PIXEL_WIDTH as i32 - BRAKE_BTNS_TOTAL_W) / 2;

const BRAKE_SPEEDS: [(BrakeSpeed, &str); 5] = [
    (BrakeSpeed::Quarter, "25%"),
    (BrakeSpeed::Half, "50%"),
    (BrakeSpeed::Normal, "100%"),
    (BrakeSpeed::TwoX, "200%"),
    (BrakeSpeed::Max, "Max"),
];

fn brake_btn_x(index: usize) -> i32 {
    BRAKE_BTNS_START_X + index as i32 * (BRAKE_BTN_W + BRAKE_BTN_GAP)
}

pub fn draw_brake_controls(frame: &mut [u8], active_speed: BrakeSpeed, pressed_speed: Option<BrakeSpeed>) {
    draw_str(frame, MARGIN, BRAKE_SECTION_Y, "Emulator Speed", HEADER_COLOR);

    for (i, &(speed, label)) in BRAKE_SPEEDS.iter().enumerate() {
        let x = brake_btn_x(i);
        let color = if active_speed == speed {
            BTN_ACTIVE_COLOR
        } else {
            BTN_COLOR
        };
        let pressed = pressed_speed == Some(speed);
        draw_raised_rect(frame, x, BRAKE_BTN_Y, BRAKE_BTN_W, BRAKE_BTN_H, color, pressed);
        let text_x = x + (BRAKE_BTN_W - label.len() as i32 * CHAR_W * SCALE) / 2;
        draw_str(frame, text_x, BRAKE_BTN_Y + 2, label, BTN_TEXT_COLOR);
    }
}

pub fn brake_button_at(px: i32, py: i32) -> Option<BrakeSpeed> {
    if !(BRAKE_BTN_Y..BRAKE_BTN_Y + BRAKE_BTN_H).contains(&py) {
        return None;
    }
    for (i, &(speed, _)) in BRAKE_SPEEDS.iter().enumerate() {
        let x = brake_btn_x(i);
        if (x..x + BRAKE_BTN_W).contains(&px) {
            return Some(speed);
        }
    }
    None
}
