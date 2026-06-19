mod common;

use rusty_vic20::{
    hardware::addressable::Addressable,
    peripherals::joystick::{JoystickDirection, JoystickUpdate},
};

const VIA1_PORT_A: u16 = 0x9111;
const VIA2_PORT_B: u16 = 0x9120;
const VIA2_DDRB: u16 = 0x9122;

#[test]
fn joystick_fire_clears_via1_port_a_bit_5() {
    let mut runner = common::run_boot();

    for _ in 0..100_000 {
        runner.step();
    }

    let port_a = runner.bus.read_byte(VIA1_PORT_A);
    assert_eq!(port_a, 126, "fire not pressed: bit 5 should be 1 (active-low)");

    runner.joystick.set_state(JoystickUpdate {
        direction: None,
        fire: true,
    });

    for _ in 0..100_000 {
        runner.step();
    }

    let port_a = runner.bus.read_byte(VIA1_PORT_A);
    assert_eq!(port_a, 94, "fire pressed: bit 5 should be 0 (active-low)");
}

#[test]
fn joystick_right_only_registers_when_ddrb_bit7_is_input() {
    let mut runner = common::run_boot();

    for _ in 0..100_000 {
        runner.step();
    }

    runner.joystick.set_state(JoystickUpdate {
        direction: Some(JoystickDirection::Right),
        fire: false,
    });

    for _ in 0..100_000 {
        runner.step();
    }

    // DDRB left as set by KERNAL (bit 7 = output for keyboard scan).
    // Joystick right should NOT affect PB read when pin is output.
    let port_b = runner.bus.read_byte(VIA2_PORT_B);
    assert!(
        port_b & 0x80 != 0,
        "DDRB bit 7 = output: joystick right should not pull bit 7 low"
    );

    // Set DDRB bit 7 to input — now the joystick can pull the pin low.
    runner.bus.write_byte(VIA2_DDRB, 0x7F);

    let port_b = runner.bus.read_byte(VIA2_PORT_B);
    assert_eq!(
        port_b & 0x80,
        0,
        "DDRB bit 7 = input: joystick right should pull bit 7 low (active-low)"
    );

    // Set DDRB bit 7 back to output — joystick no longer visible.
    runner.bus.write_byte(VIA2_DDRB, 0xFF);

    let port_b = runner.bus.read_byte(VIA2_PORT_B);
    assert!(
        port_b & 0x80 != 0,
        "DDRB bit 7 = output again: joystick right should not pull bit 7 low"
    );
}
