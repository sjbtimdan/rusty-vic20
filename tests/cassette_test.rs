mod common;

use common::{assert_screen_line, screen_code};
use rusty_vic20::{
    cpu::instruction_executor,
    keyboard::{Keyboard, make_keyboard_channel},
    paste::{new_paste_queue, text_to_petscii},
    ui::cassette_player::CassettePlayer,
};

#[test]
fn load_command_shows_press_play_on_tape() {
    let (mut bus, mut cpu) = common::run_boot();
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;

    common::assert_screen_lines(
        &bus,
        &[
            screen_code("**** CBM BASIC V2 ****"),
            screen_code("                      "),
            screen_code("3583 BYTES FREE       "),
            screen_code("                      "),
            screen_code("READY.                "),
        ],
    );

    let petscii_bytes = text_to_petscii("LOAD\n");
    let paste_queue = new_paste_queue();
    paste_queue.lock().unwrap().extend(petscii_bytes);

    let (_tx, rx) = make_keyboard_channel();
    let mut keyboard = Keyboard::new(rx, Some(paste_queue));

    let mut cassette_player = CassettePlayer::default();

    for _ in 0..2_000_000 {
        keyboard.inject_paste_into_buffer(&mut bus);
        if let Some(port_a) = keyboard.step(bus.via2.port_b()) {
            bus.via2.set_port_a(port_a);
        } else {
            bus.via2.set_port_a(0xFF);
        }
        bus.via1.set_ca1_pin(false);
        bus.step_devices(&mut cpu);
        cpu.step(&mut bus, &instruction_executor);
        cassette_player.step(&mut bus.via1);
    }

    assert_screen_line(&bus, 7, "PRESS PLAY ON TAPE    ");

    cassette_player.set_play_button(true);

    for _ in 0..500_000 {
        bus.via1.set_ca1_pin(false);
        bus.step_devices(&mut cpu);
        cpu.step(&mut bus, &instruction_executor);
        cassette_player.step(&mut bus.via1);
    }

    assert_screen_line(&bus, 8, "OK                    ");
    assert_screen_line(&bus, 10, "SEARCHING             ");
}
