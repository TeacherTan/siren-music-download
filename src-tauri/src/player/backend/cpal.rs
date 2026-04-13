#![cfg(not(target_os = "macos"))]

use crate::player::backend::PlaybackBackend;
use crate::player::decode::DecodedAudio;
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, SupportedStreamConfig};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn choose_output_config(
    device: &cpal::Device,
    audio: &DecodedAudio,
) -> Result<SupportedStreamConfig> {
    let configs = device
        .supported_output_configs()
        .context("Failed to query supported output configs")?
        .collect::<Vec<_>>();

    let exact = configs.iter().find(|config| {
        config.channels() == audio.channels
            && matches!(config.sample_format(), SampleFormat::F32)
            && config.min_sample_rate().0 <= audio.sample_rate
            && config.max_sample_rate().0 >= audio.sample_rate
    });

    if let Some(config) = exact {
        return Ok(config.with_sample_rate(cpal::SampleRate(audio.sample_rate)));
    }

    let fallback = configs
        .into_iter()
        .find(|config| matches!(config.sample_format(), SampleFormat::F32))
        .context("No supported f32 output configuration found")?;

    Ok(fallback.with_max_sample_rate())
}

fn remix_and_resample(audio: &DecodedAudio, target_channels: u16, target_rate: u32) -> Vec<f32> {
    let source_channels = audio.channels.max(1) as usize;
    let target_channels = target_channels.max(1) as usize;
    let frame_count = audio.samples.len() / source_channels;

    let ratio = if target_rate == 0 {
        1.0
    } else {
        audio.sample_rate as f64 / target_rate as f64
    };

    let target_frames = ((frame_count as f64) / ratio).ceil().max(0.0) as usize;
    let mut out = Vec::with_capacity(target_frames * target_channels);

    for target_frame in 0..target_frames {
        let source_frame = ((target_frame as f64) * ratio).floor() as usize;
        let source_frame = source_frame.min(frame_count.saturating_sub(1));

        for target_channel in 0..target_channels {
            let sample = if source_channels == target_channels {
                audio.samples[source_frame * source_channels + target_channel]
            } else if source_channels == 1 {
                audio.samples[source_frame * source_channels]
            } else if target_channels == 1 {
                let mut sum = 0.0_f32;
                for channel in 0..source_channels {
                    sum += audio.samples[source_frame * source_channels + channel];
                }
                sum / source_channels as f32
            } else {
                let mapped_channel = target_channel.min(source_channels - 1);
                audio.samples[source_frame * source_channels + mapped_channel]
            };
            out.push(sample);
        }
    }

    out
}

pub struct CpalBackend {
    stream: Option<Stream>,
}

impl CpalBackend {
    pub fn new() -> Result<Self> {
        Ok(Self { stream: None })
    }
}

impl PlaybackBackend for CpalBackend {
    fn play(
        &mut self,
        audio: DecodedAudio,
        stop_flag: Arc<AtomicBool>,
        progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
        finish_callback: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<()> {
        self.stop()?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device available")?;

        let config = choose_output_config(&device, &audio)?;
        let output_sample_rate = config.sample_rate().0;
        let output_channels = config.channels();
        let converted_samples = remix_and_resample(&audio, output_channels, output_sample_rate);
        let stream_config: cpal::StreamConfig = config.clone().into();

        let samples = Arc::new(Mutex::new(VecDeque::from(converted_samples)));
        let total_duration = audio.duration_secs;
        let sample_rate = output_sample_rate;
        let channels = output_channels;
        let frames_rendered = Arc::new(Mutex::new(0_u64));
        let finish_fired = Arc::new(AtomicBool::new(false));

        let samples_for_callback = Arc::clone(&samples);
        let frames_for_callback = Arc::clone(&frames_rendered);
        let stop_for_callback = Arc::clone(&stop_flag);
        let finish_for_callback = Arc::clone(&finish_fired);
        let progress_for_callback = Arc::clone(&progress_callback);
        let finish_callback_for_stream = Arc::clone(&finish_callback);

        let stream = device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                let mut queue = samples_for_callback.lock().unwrap();
                let mut rendered = frames_for_callback.lock().unwrap();

                if stop_for_callback.load(Ordering::SeqCst) {
                    data.fill(0.0);
                    return;
                }

                let mut wrote_samples = 0_usize;
                for sample in data.iter_mut() {
                    if let Some(value) = queue.pop_front() {
                        *sample = value;
                        wrote_samples += 1;
                    } else {
                        *sample = 0.0;
                    }
                }

                *rendered += (wrote_samples as u64) / u64::from(channels.max(1));
                let progress = *rendered as f64 / f64::from(sample_rate.max(1));
                progress_for_callback(progress.min(total_duration), total_duration);

                if queue.is_empty() && !finish_for_callback.swap(true, Ordering::SeqCst) {
                    finish_callback_for_stream();
                }
            },
            move |err| {
                eprintln!("[cpal] output stream error: {err}");
            },
            None,
        )?;

        stream.play().context("Failed to start output stream")?;
        self.stream = Some(stream);
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.stream.take();
        Ok(())
    }
}