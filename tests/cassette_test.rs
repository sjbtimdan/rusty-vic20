mod common;

use common::{UNEXPANDED_SCREEN_RAM_START, assert_screen_line, splash_screen_lines};
use rusty_vic20::emulator::paste::text_to_petscii;

#[test]
fn load_command_shows_press_play_on_tape() {
    let mut runner = common::run_boot();

    common::assert_screen_lines(&runner.bus, UNEXPANDED_SCREEN_RAM_START, &splash_screen_lines());

    let petscii_bytes = text_to_petscii("LOAD\n");
    runner.receivers.paste_queue.lock().unwrap().extend(petscii_bytes);

    for _ in 0..2_000_000 {
        runner.step();
    }

    assert_screen_line(&runner.bus, UNEXPANDED_SCREEN_RAM_START, 7, "PRESS PLAY ON TAPE    ");

    runner.cassette_player.set_play_button(true);

    for _ in 0..500_000 {
        runner.step();
    }

    assert_screen_line(&runner.bus, UNEXPANDED_SCREEN_RAM_START, 8, "OK                    ");
    assert_screen_line(&runner.bus, UNEXPANDED_SCREEN_RAM_START, 10, "SEARCHING             ");
}

#[test]
fn save_command_shows_press_record_and_play_on_tape() {
    let mut runner = common::run_boot();

    common::assert_screen_lines(&runner.bus, UNEXPANDED_SCREEN_RAM_START, &splash_screen_lines());

    let petscii_bytes = text_to_petscii("SAVE\n");
    runner.receivers.paste_queue.lock().unwrap().extend(petscii_bytes);

    for _ in 0..2_000_000 {
        runner.step();
    }

    assert_screen_line(&runner.bus, UNEXPANDED_SCREEN_RAM_START, 6, "PRESS RECORD & PLAY ON");
    assert_screen_line(&runner.bus, UNEXPANDED_SCREEN_RAM_START, 7, " TAPE                 ");

    runner.cassette_player.set_play_button(true);

    for _ in 0..500_000 {
        runner.step();
    }

    assert_screen_line(&runner.bus, UNEXPANDED_SCREEN_RAM_START, 8, "OK                    ");
    assert_screen_line(&runner.bus, UNEXPANDED_SCREEN_RAM_START, 10, "SAVING                ");
}
