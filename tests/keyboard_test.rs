use rusty_vic20::ui::keyboard::{KeyboardState, display::KeyboardWindow};
use std::sync::atomic::Ordering;

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

// ── on_key_click directly ────────────────────────────────────────────

#[test]
fn single_click_on_restore_sets_trigger() {
    let mut state = KeyboardState::new();
    let trigger = state.restore_trigger();
    state.on_key_click("RESTORE");
    assert!(trigger.load(Ordering::SeqCst));
    assert_eq!(state.status_message, "CLICK: RESTORE");
}

#[test]
fn chord_with_restore_sets_trigger() {
    let mut state = KeyboardState::new();
    let trigger = state.restore_trigger();
    state.held_key = Some("SHIFT".to_string());
    state.on_key_click("RESTORE");
    assert!(trigger.load(Ordering::SeqCst));
    assert!(state.status_message.contains("CHORD"));
}

#[test]
fn double_click_on_restore_sets_trigger() {
    let mut state = KeyboardState::new();
    let trigger = state.restore_trigger();
    let now = std::time::Instant::now();
    state.last_click = Some(("RESTORE".to_string(), now));
    state.on_key_click("RESTORE");
    assert!(trigger.load(Ordering::SeqCst));
    assert_eq!(state.held_key.as_deref(), Some("RESTORE"));
}

#[test]
fn non_restore_key_click_does_not_trigger() {
    let mut state = KeyboardState::new();
    let trigger = state.restore_trigger();
    state.on_key_click("A");
    assert!(!trigger.load(Ordering::SeqCst));
}

#[test]
fn restore_trigger_reset_after_swap() {
    let mut state = KeyboardState::new();
    let trigger = state.restore_trigger();
    state.on_key_click("RESTORE");
    assert!(trigger.load(Ordering::SeqCst));
    let was_set = trigger.swap(false, Ordering::SeqCst);
    assert!(was_set);
    assert!(!trigger.load(Ordering::SeqCst));
}

// ── handle_mouse_click through KeyboardWindow ────────────────────────

#[test]
fn keyboard_window_click_restore_center_triggers() {
    let mut keyboard_window = KeyboardWindow::default();
    let mut state = KeyboardState::new();
    let trigger = state.restore_trigger();

    keyboard_window.handle_mouse_click(RESTORE_CENTER.0, RESTORE_CENTER.1, &mut state);

    assert!(trigger.load(Ordering::SeqCst));
    assert!(state.status_message.contains("RESTORE"));
}

#[test]
fn keyboard_window_click_restore_corners_trigger() {
    let mut keyboard_window = KeyboardWindow::default();
    let mut state = KeyboardState::new();
    let trigger = state.restore_trigger();

    keyboard_window.handle_mouse_click(RESTORE_TOPLEFT.0, RESTORE_TOPLEFT.1, &mut state);
    assert!(trigger.load(Ordering::SeqCst));

    let trigger2 = state.restore_trigger();
    keyboard_window.handle_mouse_click(RESTORE_BOTTOMRIGHT.0, RESTORE_BOTTOMRIGHT.1, &mut state);
    assert!(trigger2.load(Ordering::SeqCst));
}

#[test]
fn keyboard_window_click_outside_triggers_nothing() {
    let mut keyboard_window = KeyboardWindow::default();
    let mut state = KeyboardState::new();
    let trigger = state.restore_trigger();

    keyboard_window.handle_mouse_click(OUTSIDE_GAP.0, OUTSIDE_GAP.1, &mut state);

    assert!(!trigger.load(Ordering::SeqCst));
}
