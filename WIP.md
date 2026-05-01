## Missing Screen Rendering Features

### 1. Reverse Mode (`0x900F` bit 3)
When bit 3 of register `0x900F` is set, characters with bit 7 of their char code set should display in **reverse video** — foreground and background colors are swapped.
- `vic.rs:46`: never checks bit 3 of register `[0x0F]` or bit 7 of `char_code`.
- Fix: if `(registers[0x0F] & 0x08) != 0 && (char_code & 0x80) != 0`, swap fg/bg.

### 2. Multi-Color Mode (`0x900F` bit 3 = 0 + character bit 7 set)
When reverse mode is off but a character has bit 7 set, render in multi-color: 4 colors per character using pixel pairs from the character bitmap.
- Colors: background (`0x900F` bits 7-4), border (`0x900F` bits 2-0), auxiliary (`0x900E` bits 7-4), color RAM.
- `vic.rs:49`: currently treats every pixel as 1-bit monochrome.
- Fix: decode 2-bit pixel pairs, select from the 4-color set.

### 3. Double-Height Characters (`0x9003` bit 0)
When set, characters are **16×8** instead of 8×8. Each character row's pixels repeat vertically over two text rows.
- `screen/renderer.rs:4`: `CHAR_HEIGHT` hardcoded to 8; `TEXT_ROWS` should be halved when active.
- `vic.rs:42`: register `[0x03]` bit 0 never read during rendering.
- Fix: when active, divide `active_y` by 2 for bitmap row lookup.

### Future / Not Yet Implemented
- Sound registers (`0x900A`–`0x900D`, volume `0x900E` bits 3-0)
- Raster line register (`0x9004`)
- Light pen registers (`0x9006`–`0x9007`)
- Paddle registers (`0x9008`–`0x9009`)
