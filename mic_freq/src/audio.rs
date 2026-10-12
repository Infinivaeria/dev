//! Microphone capture via cpal. Samples are down-mixed to mono and kept in a
//! bounded ring buffer that the render thread reads from each frame.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use std::collections::VecDeque;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// A repeating, softly enveloped notification tone. Never blocks the UI.
pub struct AlertTone {
    _stream: Stream,
    enabled: Arc<AtomicBool>,
}

impl AlertTone {
    pub fn start() -> Result<Self, Box<dyn Error>> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no default output device")?;
        let supported = device.default_output_config()?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let enabled = Arc::new(AtomicBool::new(false));
        let stream = match format {
            SampleFormat::F32 => output::<f32>(&device, &config, enabled.clone())?,
            SampleFormat::I16 => output::<i16>(&device, &config, enabled.clone())?,
            SampleFormat::U16 => output::<u16>(&device, &config, enabled.clone())?,
            SampleFormat::I32 => output::<i32>(&device, &config, enabled.clone())?,
            SampleFormat::U8 => output::<u8>(&device, &config, enabled.clone())?,
            SampleFormat::F64 => output::<f64>(&device, &config, enabled.clone())?,
            other => return Err(format!("unsupported output format {other:?}").into()),
        };
        stream.play()?;
        Ok(Self {
            _stream: stream,
            enabled,
        })
    }
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

fn output<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &StreamConfig,
    enabled: Arc<AtomicBool>,
) -> Result<Stream, cpal::BuildStreamError> {
    let rate = config.sample_rate.0 as f32;
    let channels = config.channels.max(1) as usize;
    let period = config.sample_rate.0.max(1) as u64;
    let mut position = 0u64;
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for frame in data.chunks_mut(channels) {
                let active = enabled.load(Ordering::Relaxed);
                let t = position as f32 / rate;
                let envelope = (t / 0.02).min(1.0) * ((0.35 - t) / 0.02).clamp(0.0, 1.0);
                let sample = if active {
                    (t * 880.0 * std::f32::consts::TAU).sin() * envelope * 0.15
                } else {
                    0.0
                };
                frame.fill(T::from_sample(sample));
                position = if active { (position + 1) % period } else { 0 };
            }
        },
        |err| eprintln!("alert audio error: {err}"),
        None,
    )
}

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
        let device = host
            .default_input_device()
            .ok_or("no default input (microphone) device found")?;
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

        Ok(Self {
            _stream: stream,
            buffer,
            capacity,
            sample_rate: config.sample_rate.0 as f32,
            device_name,
        })
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
                let mono =
                    frame.iter().map(|&s| s.to_sample::<f32>()).sum::<f32>() / frame.len() as f32;
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
