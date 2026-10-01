//! Microphone capture via cpal. Samples are down-mixed to mono and kept in a
//! bounded ring buffer that the render thread reads from each frame.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, Mutex};

pub struct MicCapture {
    _stream: Stream,
    buffer: Arc<Mutex<VecDeque<f32>>>,
    capacity: usize,
    pub sample_rate: f32,
    pub device_name: String,
}

impl MicCapture {
    /// Open the default input device and start streaming. `capacity` is the
    /// number of most recent mono samples retained.
    pub fn start(capacity: usize) -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("no default input (microphone) device found")?;
        let device_name = device.name().unwrap_or_else(|_| "unknown".into());
        let supported = device.default_input_config()?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.into();

        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(capacity)));
        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, &config, buffer.clone(), capacity)?,
            SampleFormat::I16 => build::<i16>(&device, &config, buffer.clone(), capacity)?,
            SampleFormat::U16 => build::<u16>(&device, &config, buffer.clone(), capacity)?,
            SampleFormat::I32 => build::<i32>(&device, &config, buffer.clone(), capacity)?,
            SampleFormat::U8 => build::<u8>(&device, &config, buffer.clone(), capacity)?,
            SampleFormat::F64 => build::<f64>(&device, &config, buffer.clone(), capacity)?,
            other => return Err(format!("unsupported sample format {other:?}").into()),
        };
        stream.play()?;

        Ok(Self { _stream: stream, buffer, capacity, sample_rate: config.sample_rate.0 as f32, device_name })
    }

    /// Copy the latest (up to `n`) samples into `out`.
    pub fn latest(&self, n: usize, out: &mut Vec<f32>) {
        out.clear();
        let buf = self.buffer.lock().unwrap();
        let n = n.min(buf.len()).min(self.capacity);
        out.extend(buf.range(buf.len() - n..));
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    buffer: Arc<Mutex<VecDeque<f32>>>,
    capacity: usize,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels.max(1) as usize;
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            let mut buf = buffer.lock().unwrap();
            for frame in data.chunks(channels) {
                let mono = frame.iter().map(|&s| s.to_sample::<f32>()).sum::<f32>() / frame.len() as f32;
                if buf.len() == capacity {
                    buf.pop_front();
                }
                buf.push_back(mono);
            }
        },
        |err| eprintln!("audio input error: {err}"),
        None,
    )
}
