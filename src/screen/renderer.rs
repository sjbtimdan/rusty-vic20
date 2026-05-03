pub const TEXT_COLUMNS: usize = 22;
pub const TEXT_ROWS: usize = 23;
pub const CHAR_WIDTH: usize = 8;
pub const CHAR_HEIGHT: usize = 8;

// Visible VIC-20 active display area at startup.
pub const ACTIVE_WIDTH: usize = TEXT_COLUMNS * CHAR_WIDTH;
pub const ACTIVE_HEIGHT: usize = TEXT_ROWS * CHAR_HEIGHT;

pub const BORDER_LEFT: usize = 16;
pub const BORDER_RIGHT: usize = 16;
pub const BORDER_TOP: usize = 16;
pub const BORDER_BOTTOM: usize = 16;

pub const PAL_WIDTH: usize = ACTIVE_WIDTH + BORDER_LEFT + BORDER_RIGHT;
pub const PAL_HEIGHT: usize = ACTIVE_HEIGHT + BORDER_TOP + BORDER_BOTTOM;

pub fn display_vic20_screen(frame: &mut [u8], border_rgba: &[u8; 4], screen_rgba: &[u8]) {
    let expected_screen_len = ACTIVE_WIDTH * ACTIVE_HEIGHT * 4;
    if screen_rgba.len() != expected_screen_len {
        panic!(
            "Invalid screen buffer length: expected {}, got {}",
            expected_screen_len,
            screen_rgba.len()
        );
    }

    let expected_frame_len = PAL_WIDTH * PAL_HEIGHT * 4;
    if frame.len() != expected_frame_len {
        panic!(
            "display frame buffer must be exactly {} bytes ({} pixels)",
            expected_frame_len,
            PAL_WIDTH * PAL_HEIGHT
        );
    }

    // Fill full output frame with border color first.
    for chunk in frame.chunks_exact_mut(4) {
        chunk.copy_from_slice(border_rgba);
    }

    // Blit active screen into the centered inner area.
    for y in 0..ACTIVE_HEIGHT {
        let src_start = y * ACTIVE_WIDTH * 4;
        let dst_start = ((y + BORDER_TOP) * PAL_WIDTH + BORDER_LEFT) * 4;
        frame[dst_start..dst_start + ACTIVE_WIDTH * 4]
            .copy_from_slice(&screen_rgba[src_start..src_start + ACTIVE_WIDTH * 4]);
    }
}

pub fn palette(index: u8) -> [u8; 4] {
    match index {
        0 => [0x00, 0x00, 0x00, 0xFF],  // black
        1 => [0xFF, 0xFF, 0xFF, 0xFF],  // white
        2 => [0x88, 0x00, 0x00, 0xFF],  // red
        3 => [0xAA, 0xFF, 0xEE, 0xFF],  // cyan
        4 => [0xCC, 0x44, 0xCC, 0xFF],  // purple
        5 => [0x00, 0xCC, 0x55, 0xFF],  // green
        6 => [0x00, 0x00, 0xAA, 0xFF],  // blue
        7 => [0xEE, 0xEE, 0x77, 0xFF],  // yellow
        8 => [0xDD, 0x88, 0x55, 0xFF],  // orange
        9 => [0xFF, 0xBB, 0x77, 0xFF],  // light orange
        10 => [0xFF, 0x77, 0x77, 0xFF], // pink
        11 => [0xCC, 0xFF, 0xFF, 0xFF], // light cyan
        12 => [0xFF, 0xBB, 0xFF, 0xFF], // light purple
        13 => [0xAA, 0xFF, 0x66, 0xFF], // light green
        14 => [0x77, 0x77, 0xFF, 0xFF], // light blue
        15 => [0xFF, 0xFF, 0xBB, 0xFF], // light yellow
        _ => [0x00, 0x00, 0x00, 0xFF],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_writes_border_and_inner_screen() {
        let mut frame = vec![0_u8; PAL_WIDTH * PAL_HEIGHT * 4];
        let border = [0x11, 0x22, 0x33, 0x44];
        let screen = vec![0xAA_u8; ACTIVE_WIDTH * ACTIVE_HEIGHT * 4];

        display_vic20_screen(&mut frame, &border, &screen);

        // Top-left pixel should be border color.
        assert_eq!(&frame[0..4], &[0x11, 0x22, 0x33, 0x44]);

        // First active pixel should be screen color.
        let first_active = ((BORDER_TOP * PAL_WIDTH) + BORDER_LEFT) * 4;
        assert_eq!(&frame[first_active..first_active + 4], &[0xAA, 0xAA, 0xAA, 0xAA]);
    }
}
