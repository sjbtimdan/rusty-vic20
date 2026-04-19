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

pub fn display_vic20_screen(frame: &mut [u8], border_rgba: u32, screen_rgba: &[u32]) {
    let expected_screen_len = ACTIVE_WIDTH * ACTIVE_HEIGHT;
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
    for pixel in frame.chunks_exact_mut(4) {
        write_rgba_bytes(pixel, border_rgba);
    }

    // Blit active screen into the centered inner area.
    for y in 0..ACTIVE_HEIGHT {
        let src_row_start = y * ACTIVE_WIDTH;
        let dst_row_start = (y + BORDER_TOP) * PAL_WIDTH + BORDER_LEFT;

        for x in 0..ACTIVE_WIDTH {
            let src_pixel = screen_rgba[src_row_start + x];
            let dst_index = (dst_row_start + x) * 4;
            write_rgba_bytes(&mut frame[dst_index..dst_index + 4], src_pixel);
        }
    }
}

fn write_rgba_bytes(bytes: &mut [u8], rgba: u32) {
    bytes[0] = ((rgba >> 24) & 0xFF) as u8;
    bytes[1] = ((rgba >> 16) & 0xFF) as u8;
    bytes[2] = ((rgba >> 8) & 0xFF) as u8;
    bytes[3] = (rgba & 0xFF) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_writes_border_and_inner_screen() {
        let mut frame = vec![0_u8; PAL_WIDTH * PAL_HEIGHT * 4];
        let border = 0x11223344;
        let screen = vec![0xAABBCCDD; ACTIVE_WIDTH * ACTIVE_HEIGHT];

        display_vic20_screen(&mut frame, border, &screen);

        // Top-left pixel should be border color.
        assert_eq!(&frame[0..4], &[0x11, 0x22, 0x33, 0x44]);

        // First active pixel should be screen color.
        let first_active = ((BORDER_TOP * PAL_WIDTH) + BORDER_LEFT) * 4;
        assert_eq!(&frame[first_active..first_active + 4], &[0xAA, 0xBB, 0xCC, 0xDD]);
    }
}
