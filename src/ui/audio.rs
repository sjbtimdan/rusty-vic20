use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::error;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer, Split},
};

const RING_SIZE: usize = 8192;
const SAMPLE_RATE: u32 = 44100;

pub fn create_audio_channel() -> (AudioProducer, cpal::Stream) {
    let rb = HeapRb::<f32>::new(RING_SIZE);
    let (prod, mut cons) = rb.split();

    let host = cpal::default_host();
    let device = host.default_output_device().expect("no default audio output device");
    let supported = device.default_output_config().expect("no default audio output config");
    let mut config: cpal::StreamConfig = supported.into();
    config.sample_rate = SAMPLE_RATE;
    config.channels = 1;

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for sample in data.iter_mut() {
                    *sample = cons.try_pop().unwrap_or(0.0);
                }
            },
            |err| error!("audio stream error: {}", err),
            None,
        )
        .expect("failed to build audio output stream");

    stream.play().expect("failed to start audio stream");

    (AudioProducer { inner: prod }, stream)
}

pub struct AudioProducer {
    inner: <HeapRb<f32> as Split>::Prod,
}

impl AudioProducer {
    pub fn push(&mut self, sample: f32) {
        let _ = self.inner.try_push(sample);
    }

    pub fn noop() -> Self {
        let rb = HeapRb::<f32>::new(1);
        let (prod, _cons) = rb.split();
        AudioProducer { inner: prod }
    }
}
