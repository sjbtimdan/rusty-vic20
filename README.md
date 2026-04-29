# rusty-vic20

Still a WIP.

## Plan

- [x] 6502 CPU core.
- [x] VIC chip, rendering VIC-20 text from screen memory.
- [x] On screen VIC-20 keyboard
- [ ] VIA1 and 2 chips (driving interrupts, etc)
- [ ] Joystick support
- [ ] Lightpen
- [ ] Load programs from cassette or binary data.
- [ ] Sound.
- [ ] Accurate emulation down to the cycle and raster level.
- [ ] Integration testing.

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

