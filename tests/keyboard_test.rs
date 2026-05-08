use rusty_vic20::ui::keyboard::KeyboardState;

const RESTORE_CENTER: (f32, f32) = (838.5, 91.0);
const RESTORE_TOPLEFT: (f32, f32) = (807.0, 65.0);
const RESTORE_BOTTOMRIGHT: (f32, f32) = (870.0, 117.0);
const OUTSIDE_GAP: (f32, f32) = (0.0, 0.0);

// ── Pixel → key mapping ─────────────────────────────────────────────

#[test]
fn restore_key_pixel_maps_to_label() {
    let state = KeyboardState::new();
    assert_eq!(state.key_at_pixel(RESTORE_CENTER.0, RESTORE_CENTER.1), Some("RESTORE"));
    assert_eq!(
        state.key_at_pixel(RESTORE_TOPLEFT.0, RESTORE_TOPLEFT.1),
        Some("RESTORE")
    );
    assert_eq!(
        state.key_at_pixel(RESTORE_BOTTOMRIGHT.0, RESTORE_BOTTOMRIGHT.1),
        Some("RESTORE")
    );
    assert_eq!(state.key_at_pixel(OUTSIDE_GAP.0, OUTSIDE_GAP.1), None);
}
