//! 基于 CPAL 的默认音频播放后端实现。
//!
//! 该模块负责选择可用输出设备与格式、创建音频输出流，并把播放器解码后的样本缓冲
//! 推送到系统音频设备，供桌面端实际发声使用。

use crate::player::backend::PlaybackBackend;
use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, SupportedStreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn choose_output_config(
    device: &cpal::Device,
    audio_format: AudioFormat,
) -> Result<SupportedStreamConfig> {
    let configs = device
        .supported_output_configs()
        .context("Failed to query supported output configs")?
        .collect::<Vec<_>>();

    let exact = configs.iter().find(|config| {
        config.channels() == audio_format.channels
            && matches!(config.sample_format(), SampleFormat::F32)
            && config.min_sample_rate() <= audio_format.sample_rate
            && config.max_sample_rate() >= audio_format.sample_rate
    });

    if let Some(config) = exact {
        return Ok(config.with_sample_rate(audio_format.sample_rate));
    }

    let fallback = configs
        .into_iter()
        .find(|config| matches!(config.sample_format(), SampleFormat::F32))
        .context("No supported f32 output configuration found")?;

    Ok(fallback.with_max_sample_rate())
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
    fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<AudioFormat> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device available")?;
        let config = choose_output_config(&device, source_format)?;
        Ok(AudioFormat {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            duration_secs: source_format.duration_secs,
        })
    }

    fn play_stream(
        &mut self,
        format: AudioFormat,
        samples: SampleBuffer,
        stop_flag: Arc<AtomicBool>,
        volume: Arc<Mutex<f64>>,
        progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
        finish_callback: Arc<dyn Fn() + Send + Sync>,
        error_handler: PlaybackErrorHandler,
    ) -> Result<()> {
        self.stop()?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device available")?;
        let config = choose_output_config(&device, format)?;
        let stream_config: cpal::StreamConfig = config.clone().into();

        let total_duration = format.duration_secs;
        let output_rate = config.sample_rate();
        let output_channels = config.channels();
        let frames_rendered = Arc::new(Mutex::new(0_u64));
        let finish_fired = Arc::new(AtomicBool::new(false));

        let samples_for_callback = samples.clone();
        let frames_for_callback = Arc::clone(&frames_rendered);
        let stop_for_callback = Arc::clone(&stop_flag);
        let volume_for_callback = Arc::clone(&volume);
        let finish_for_callback = Arc::clone(&finish_fired);
        let progress_for_callback = Arc::clone(&progress_callback);
        let finish_callback_for_stream = Arc::clone(&finish_callback);
        let error_handler_for_callback = Arc::clone(&error_handler);
        let error_handler_for_stream = Arc::clone(&error_handler);
        let buffer_error_reported = Arc::new(AtomicBool::new(false));

        let stream = device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                if stop_for_callback.load(Ordering::SeqCst) {
                    data.fill(0.0);
                    return;
                }

                let status = samples_for_callback.pop_into(data);
                if status.written < data.len() {
                    data[status.written..].fill(0.0);
                }

                let gain = (*volume_for_callback.lock().unwrap()).clamp(0.0, 1.0) as f32;
                if gain < 0.999 {
                    for sample in &mut data[..status.written] {
                        *sample *= gain;
                    }
                }

                if let Some(error) = status.error {
                    if !buffer_error_reported.swap(true, Ordering::SeqCst) {
                        error_handler_for_callback(error);
                    }
                }

                let mut rendered = frames_for_callback.lock().unwrap();
                *rendered += (status.written as u64) / u64::from(output_channels.max(1));
                let progress = *rendered as f64 / f64::from(output_rate.max(1));
                progress_for_callback(progress.min(total_duration.max(progress)), total_duration);

                if status.finished && !finish_for_callback.swap(true, Ordering::SeqCst) {
                    finish_callback_for_stream();
                }
            },
            move |err| {
                error_handler_for_stream(err.to_string());
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

    fn pause(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.pause().context("Failed to pause output stream")?;
        }
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.play().context("Failed to resume output stream")?;
        }
        Ok(())
    }
}
