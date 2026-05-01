use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
};

const SCREEN_RAM_START: u16 = 0x1E00;
const SCREEN_LINE_LEN: u16 = 22;
const STEPS: usize = 600_000;

/// Read a line of screen RAM (22 bytes) starting at the given address.
fn read_screen_line(bus: &Bus, addr: u16) -> [u8; 22] {
    let mut line = [0u8; 22];
    for (i, b) in line.iter_mut().enumerate() {
        *b = bus.read_byte(addr + i as u16);
    }
    line
}

#[test]
fn splash_screen_shows_on_startup() {
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;
    bus.load_standard_roms_from_data_dir();
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);

    for _ in 0..STEPS {
        cpu.step(&mut bus, &instruction_executor);
        bus.step_devices(&mut cpu);
    }
    // VIC-20 unexpanded start screen (3583 BYTES FREE):
    //
    //   **** CBM BASIC V2 ****
    //
    //   3583 BYTES FREE
    //
    //   READY.
    //   █
    //
    // Screen codes on the VIC-20 map A-Z to 0x01-0x1A and match
    // PETSCII for printable characters (0x20+).

    let expected_lines = vec![
        // Line 1: "**** CBM BASIC V2 ****" at 0x1E00
        [
            0x2A, 0x2A, 0x2A, 0x2A, 0x20, 0x03, 0x02, 0x0D, 0x20, 0x02, 0x01, 0x13, 0x09, 0x03, 0x20, 0x16, 0x32, 0x20,
            0x2A, 0x2A, 0x2A, 0x2A,
        ],
        // Line 2: blank row at 0x1E16
        [0x20; 22],
        // Line 3: "3583 BYTES FREE" at 0x1E2C (15 chars + 7 spaces)
        [
            0x33, 0x35, 0x38, 0x33, 0x20, 0x02, 0x19, 0x14, 0x05, 0x13, 0x20, 0x06, 0x12, 0x05, 0x05, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20,
        ],
        // Line 4: blank row at 0x1E42
        [0x20; 22],
        // Line 5: "READY." at 0x1E58 (6 chars + 16 spaces)
        [
            0x12, 0x05, 0x01, 0x04, 0x19, 0x2E, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20,
        ],
    ];

    for (i, expected_line) in expected_lines.iter().enumerate() {
        let actual_line = read_screen_line(&bus, SCREEN_RAM_START + i as u16 * SCREEN_LINE_LEN);
        assert_eq!(
            actual_line,
            *expected_line,
            "Line {} does not match expected startup content",
            i + 1
        );
    }
}
