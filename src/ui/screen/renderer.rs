pub const TEXT_COLUMNS: usize = 22;
pub const TEXT_ROWS: usize = 23;
pub const CHAR_WIDTH: usize = 8;
pub const CHAR_HEIGHT: usize = 8;

pub const ACTIVE_HEIGHT: usize = TEXT_ROWS * CHAR_HEIGHT;

pub const BORDER_LEFT: usize = 16;
pub const BORDER_RIGHT: usize = 16;
pub const BORDER_TOP: usize = 16;
pub const BORDER_BOTTOM: usize = 16;

pub const PAL_HEIGHT: usize = ACTIVE_HEIGHT + BORDER_TOP + BORDER_BOTTOM;

pub fn render_vic20_screen(frame: &mut [u8], border_rgba: &[u8; 4], screen_rgba: &[u8], active_width: usize) {
    let pal_width = active_width + BORDER_LEFT + BORDER_RIGHT;
    let pal_height = ACTIVE_HEIGHT + BORDER_TOP + BORDER_BOTTOM;

    let expected_screen_len = active_width * ACTIVE_HEIGHT * 4;
    if screen_rgba.len() != expected_screen_len {
        panic!(
            "Invalid screen buffer length: expected {}, got {}",
            expected_screen_len,
            screen_rgba.len()
        );
    }

    let expected_frame_len = pal_width * pal_height * 4;
    if frame.len() != expected_frame_len {
        panic!(
            "display frame buffer must be exactly {} bytes ({} pixels)",
            expected_frame_len,
            pal_width * pal_height
        );
    }

    for chunk in frame.as_chunks_mut::<4>().0 {
        chunk.copy_from_slice(border_rgba);
    }

    for y in 0..ACTIVE_HEIGHT {
        let src_start = y * active_width * 4;
        let dst_start = ((y + BORDER_TOP) * pal_width + BORDER_LEFT) * 4;
        frame[dst_start..dst_start + active_width * 4]
            .copy_from_slice(&screen_rgba[src_start..src_start + active_width * 4]);
    }
}

const PALETTE: [[u8; 4]; 16] = [
    [0x00, 0x00, 0x00, 0xFF],
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0x88, 0x00, 0x00, 0xFF],
    [0xAA, 0xFF, 0xEE, 0xFF],
    [0xCC, 0x44, 0xCC, 0xFF],
    [0x00, 0xCC, 0x55, 0xFF],
    [0x00, 0x00, 0xAA, 0xFF],
    [0xEE, 0xEE, 0x77, 0xFF],
    [0xDD, 0x88, 0x55, 0xFF],
    [0xFF, 0xBB, 0x77, 0xFF],
    [0xFF, 0x77, 0x77, 0xFF],
    [0xCC, 0xFF, 0xFF, 0xFF],
    [0xFF, 0xBB, 0xFF, 0xFF],
    [0xAA, 0xFF, 0x66, 0xFF],
    [0x77, 0x77, 0xFF, 0xFF],
    [0xFF, 0xFF, 0xBB, 0xFF],
];

pub fn palette(index: u8) -> [u8; 4] {
    PALETTE.get(index as usize).copied().unwrap_or(PALETTE[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_writes_border_and_inner_screen() {
        let active_width = TEXT_COLUMNS * CHAR_WIDTH;
        let pal_width = active_width + BORDER_LEFT + BORDER_RIGHT;
        let mut frame = vec![0_u8; pal_width * PAL_HEIGHT * 4];
        let border = [0x11, 0x22, 0x33, 0x44];
        let screen = vec![0xAA_u8; active_width * ACTIVE_HEIGHT * 4];

        render_vic20_screen(&mut frame, &border, &screen, active_width);

        assert_eq!(&frame[0..4], &[0x11, 0x22, 0x33, 0x44]);

        let first_active = ((BORDER_TOP * pal_width) + BORDER_LEFT) * 4;
        assert_eq!(&frame[first_active..first_active + 4], &[0xAA, 0xAA, 0xAA, 0xAA]);
    }
}
