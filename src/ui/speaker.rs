use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct Speaker {
    _cb2: Arc<Mutex<u8>>,
}

impl Speaker {
    pub fn new(cb2: Arc<Mutex<u8>>) -> Self {
        Self { _cb2: cb2 }
    }

    pub fn start(&self) {
        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();
        let config = device.default_output_config().unwrap();

        let sample_rate = config.sample_rate() as f32;
        let mut t = 0.0;

        // Replace this with your VIA timer emulation logic:
        let mut next_cb2 = move || {
            t += 1.0f32 / sample_rate;
            if (t * 440.0).sin() > 0.0 { 1.0 } else { -1.0 }
        };

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for sample in data.iter_mut() {
                        *sample = next_cb2();
                    }
                },
                |err| {
                    eprintln!("Stream error: {}", err);
                },
                None,
            )
            .unwrap();

        stream.play().unwrap();
    }
}
