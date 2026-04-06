## Development Strategy
- [ ] UI-first: Build a simple window with a framebuffer (using pixels or minifb in Rust, or Avalonia in C#).
- [ ] CPU core: Implement the 6502 instruction set.
- [ ] Video: Start with text mode rendering—map character codes to glyphs.
- [ ] Sound: Add square wave generation later.
- [ ] Input: Hook up keyboard and joystick.
- [ ] Input: Cassette loader.

### UI

- Allocate a framebuffer (e.g. 176×184 pixels if you scale each character 8×8).
- Loop through screen RAM:
- Read the character code.
- Look up the glyph bitmap (from ROM or a placeholder font).
- Copy the glyph’s pixels into the framebuffer.
- Push framebuffer to pixels each frame.
