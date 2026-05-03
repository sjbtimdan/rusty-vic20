# rusty-vic20

Still a WIP.

## Plan

- [x] 6502 CPU core.
- [x] VIC chip, rendering VIC-20 text from screen memory.
- [x] On screen VIC-20 keyboard
- [X] VIA 2 chip (driving interrupts, etc)
- [X] Integration testing.
- [ ] Keyboard interaction.
- [ ] Speed control.
- [ ] Sound.
- [ ] Lightpen
- [ ] Load programs from cassette or binary data.
- [ ] Accurate emulation down to the cycle and raster level.
- [ ] VIA 1 chip
- [ ] Joystick support

## Vic 20
```
cargo run --bin vic20
```

![Start screen](start-screen.png)

## Disassembler

Run the disassembler on a binary file:


```
cargo run --bin disassembler -- data/somefile.bin
```

