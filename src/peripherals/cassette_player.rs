use log::info;
use std::sync::mpsc::{self, Receiver, SyncSender};

use crate::hardware::via::VIA;

#[derive(Default)]
pub struct CassettePlayer {
    pressed: bool,
}

impl CassettePlayer {
    pub fn step(&self, via: &mut VIA) {
        via.set_cassette_sense(!self.pressed);
    }

    pub fn set_play_button(&mut self, pressed: bool) {
        info!("Play button pressed={}", pressed);
        self.pressed = pressed;
    }
}

pub fn make_cassette_channel() -> (SyncSender<bool>, Receiver<bool>) {
    mpsc::sync_channel(2)
}
