use crate::{
    peripherals::{brake::BrakeSpeed, joystick::JoystickUpdate},
    ui::{control::SharedPerformanceMetrics, keyboard::key, screen::display::SharedVideoState},
};
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        Arc,
        Mutex,
        mpsc::{Receiver, SyncSender},
    },
};

pub struct ThreadSenders {
    pub video: Arc<Mutex<SharedVideoState>>,
    pub perf: Arc<Mutex<SharedPerformanceMetrics>>,
    pub keyboard_sender: SyncSender<HashSet<key::Key>>,
    pub paste_queue: Arc<Mutex<VecDeque<u8>>>,
    pub load_queue: LoadQueue,
    pub cassette_sender: SyncSender<bool>,
    pub joystick_sender: SyncSender<JoystickUpdate>,
    pub direct_loader_sender: SyncSender<Vec<u8>>,
    pub brake_sender: SyncSender<BrakeSpeed>,
    pub shutdown_sender: SyncSender<()>,
}

pub struct ThreadReceivers {
    pub video: Arc<Mutex<SharedVideoState>>,
    pub perf: Arc<Mutex<SharedPerformanceMetrics>>,
    pub load_queue: LoadQueue,
    pub paste_queue: Arc<Mutex<VecDeque<u8>>>,
    pub keyboard_receiver: Receiver<HashSet<key::Key>>,
    pub cassette_receiver: Receiver<bool>,
    pub joystick_receiver: Receiver<JoystickUpdate>,
    pub direct_loader_receiver: Receiver<Vec<u8>>,
    pub brake_receiver: Receiver<BrakeSpeed>,
    pub shutdown_receiver: Receiver<()>,
}

pub struct PrgLoadRequest {
    pub path: String,
    pub data: Vec<u8>,
}

pub type LoadQueue = Arc<Mutex<VecDeque<PrgLoadRequest>>>;
