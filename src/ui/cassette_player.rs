use log::info;
use std::sync::mpsc::{self, Receiver, SyncSender};

#[derive(Default)]
pub struct CassettePlayer {
    play_button: bool,
}

impl CassettePlayer {
    pub fn play_button(&self) -> bool {
        self.play_button
    }

    pub fn set_play_button(&mut self, pressed: bool) {
        info!("Play button pressed={}", pressed);
        self.play_button = pressed;
    }
}

pub fn make_cassette_channel() -> (SyncSender<bool>, Receiver<bool>) {
    mpsc::sync_channel(2)
}
