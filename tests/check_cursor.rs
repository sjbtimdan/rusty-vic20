use rusty_vic20::{
    addressable::Addressable,
    bus::Bus,
    cpu::{cpu6502::CPU6502, instruction_executor},
    screen::renderer::{ACTIVE_WIDTH, CHAR_HEIGHT, CHAR_WIDTH, palette},
};

fn pixel_at(fb: &[u8], x: usize, y: usize) -> [u8; 4] {
    let idx = (y * ACTIVE_WIDTH + x) * 4;
    fb[idx..idx + 4].try_into().unwrap()
}

#[test]
fn check_cursor_render() {
    let steps = 3_000_000;
    let mut cpu = CPU6502::default();
    let mut bus = Bus::default();
    let ie = instruction_executor::DefaultInstructionExecutor;
    bus.load_standard_roms_from_data_dir();
    let rv = bus.read_word(0xFFFC);
    cpu.reset(rv);

    for _ in 0..steps {
        cpu.step(&mut bus, &ie);
        bus.step_devices(&mut cpu);
    }

    let fb = bus.render_active_screen();

    // Cursor is on line 5 (0-indexed), at column 0
    let cursor_line = 5;
    let cursor_col = 0;

    for y in 0..8 {
        for x in 0..8 {
            let px = cursor_col * CHAR_WIDTH + x;
            let py = cursor_line * CHAR_HEIGHT + y;
            let color = pixel_at(&fb, px, py);
            let is_white = color == palette(1);
            eprint!(
                "{}",
                if is_white {
                    "W"
                } else if color == palette(1) {
                    "1"
                } else {
                    "."
                }
            );
        }
        eprintln!();
    }

    // Also check a normal text character (first char, '*')
    eprintln!("Text '*' at (0,0):");
    for y in 0..8 {
        for x in 0..8 {
            let color = pixel_at(&fb, x, y);
            eprint!(
                "{}",
                if color == palette(1) {
                    "W"
                } else if color == palette(0) {
                    "B"
                } else {
                    "."
                }
            );
        }
        eprintln!();
    }

    eprintln!("Palette 0 (BLACK): {:02X?}", palette(0));
    eprintln!("Palette 1 (WHITE): {:02X?}", palette(1));
    eprintln!("Palette 6 (BLUE):  {:02X?}", palette(6));
}
