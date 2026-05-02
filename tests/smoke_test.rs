use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
};

const SCREEN_RAM_START: u16 = 0x1E00;
const SCREEN_LINE_LEN: u16 = 22;
const STEPS: usize = 600_000;

const JIFFY_LO: u16 = 0x00A0;
const JIFFY_MID: u16 = 0x00A1;
const JIFFY_HI: u16 = 0x00A2;

fn run_steps(steps: usize) -> (Bus, CPU6502) {
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let instruction_executor = instruction_executor::DefaultInstructionExecutor;
    bus.load_standard_roms_from_data_dir();
    let reset_vector = bus.read_word(0xFFFC);
    cpu.reset(reset_vector);

    for _ in 0..steps {
        cpu.step(&mut bus, &instruction_executor);
        bus.step_devices(&mut cpu);
    }
    (bus, cpu)
}

fn read_screen_line(bus: &Bus, addr: u16) -> [u8; 22] {
    let mut line = [0u8; 22];
    for (i, b) in line.iter_mut().enumerate() {
        *b = bus.read_byte(addr + i as u16);
    }
    line
}

fn read_jiffy(bus: &Bus) -> u32 {
    let lo = bus.read_byte(JIFFY_LO) as u32;
    let mid = bus.read_byte(JIFFY_MID) as u32;
    let hi = bus.read_byte(JIFFY_HI) as u32;
    (hi << 16) | (mid << 8) | lo
}

#[test]
fn splash_screen_shows_on_startup() {
    let (bus, _cpu) = run_steps(STEPS);

    let expected_lines = vec![
        // Line 1: "**** CBM BASIC V2 ****" at 0x1E00
        [
            0x2A, 0x2A, 0x2A, 0x2A, 0x20, 0x03, 0x02, 0x0D, 0x20, 0x02, 0x01, 0x13, 0x09, 0x03, 0x20, 0x16, 0x32, 0x20,
            0x2A, 0x2A, 0x2A, 0x2A,
        ],
        // Line 2: blank row at 0x1E16
        [0x20; 22],
        // Line 3: "3583 BYTES FREE" at 0x1E2C
        [
            0x33, 0x35, 0x38, 0x33, 0x20, 0x02, 0x19, 0x14, 0x05, 0x13, 0x20, 0x06, 0x12, 0x05, 0x05, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20,
        ],
        // Line 4: blank row at 0x1E42
        [0x20; 22],
        // Line 5: "READY." at 0x1E58
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

#[test]
fn jiffy_clock_advances_with_irqs() {
    let steps_early = 2_000_000;
    let steps_late = steps_early + 1_000_000;

    let (bus_early, _) = run_steps(steps_early);
    let jiffy_early = read_jiffy(&bus_early);

    let (bus_late, cpu_late) = run_steps(steps_late);
    let jiffy_late = read_jiffy(&bus_late);

    eprintln!(
        "After {:>10} steps: jiffy = {} (I={}, PC={:04X})",
        steps_early,
        jiffy_early,
        if cpu_late.registers.is_flag_set(0x04) { "1" } else { "0" },
        cpu_late.registers.pc,
    );
    eprintln!("After {:>10} steps: jiffy = {}", steps_late, jiffy_late);

    let via_ier = bus_late.read_byte(0x912E);
    let via_ifr = bus_late.read_byte(0x912D);
    eprintln!("VIA2: IER={:02X} IFR={:02X}", via_ier, via_ifr);

    assert!(
        via_ier & 0x40 != 0,
        "KERNAL should enable Timer 1 interrupts (bit 6 of IER)"
    );
    assert!(
        jiffy_late > jiffy_early,
        "Jiffy clock should advance ({} -> {}) after {} additional steps",
        jiffy_early,
        jiffy_late,
        steps_late - steps_early,
    );
}
