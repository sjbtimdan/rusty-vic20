/*

  VIA1 only $9111 bit 6 (0x40) should be 1 if play button off else 0

*/

#[derive(Default)]
pub struct CassettePlayer {
    play_button: bool,
}

impl CassettePlayer {
    pub fn play_button(&self) -> bool {
        self.play_button
    }
}
