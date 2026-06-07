mod common;

use rusty_vic20::{addressable::Addressable, peripherals::joystick::JoystickUpdate};

const VIA1_PORT_A: u16 = 0x9111;

#[test]
fn joystick_fire_clears_via1_port_a_bit_5() {
    let mut runner = common::run_boot();

    for _ in 0..100_000 {
        runner.step_keyboard();
        runner.step();
    }

    let port_a = runner.bus.read_byte(VIA1_PORT_A);
    assert_eq!(port_a, 126, "fire not pressed: bit 5 should be 1 (active-low)");

    runner.joystick.set_state(JoystickUpdate {
        direction: None,
        fire: true,
    });

    for _ in 0..100_000 {
        runner.step_keyboard();
        runner.step();
    }

    let port_a = runner.bus.read_byte(VIA1_PORT_A);
    assert_eq!(port_a, 94, "fire pressed: bit 5 should be 0 (active-low)");
}
