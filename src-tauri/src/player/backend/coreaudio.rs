#![cfg(target_os = "macos")]

use crate::player::backend::PlaybackBackend;
use crate::player::decode::DecodedAudio;
use anyhow::{Context, Result};
use coreaudio::audio_unit::audio_format::LinearPcmFlags;
use coreaudio::audio_unit::macos_helpers::{
    get_default_device_id, get_hogging_pid, set_device_sample_rate, toggle_hog_mode,
};
use coreaudio::audio_unit::render_callback::data::Interleaved;
use coreaudio::audio_unit::render_callback::Args;
use coreaudio::audio_unit::{AudioUnit, IOType, SampleFormat, Scope, StreamFormat};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct CoreAudioBackend {
    audio_unit: Option<AudioUnit>,
    is_exclusive: bool,
    hogged_device_id: Option<u32>,
}

impl CoreAudioBackend {
    pub fn new() -> Result<Self> {
        Ok(Self {
            audio_unit: None,
            is_exclusive: false,
            hogged_device_id: None,
        })
    }
}

impl PlaybackBackend for CoreAudioBackend {
    fn play(
        &mut self,
        audio: DecodedAudio,
        stop_flag: Arc<AtomicBool>,
        progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
        finish_callback: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<()> {
        self.stop()?;

        let device_id = get_default_device_id(false).context("No default output device found")?;

        // Best-effort Hog Mode acquisition. Failure is acceptable; we fall back to shared mode.
        self.is_exclusive = match toggle_hog_mode(device_id) {
            Ok(pid) if pid > 0 => {
                self.hogged_device_id = Some(device_id);
                true
            }
            Ok(_) => {
                self.hogged_device_id = None;
                false
            }
            Err(_) => {
                self.hogged_device_id = None;
                false
            }
        };

        let setup_result = (|| -> Result<AudioUnit> {
            let _ = set_device_sample_rate(device_id, f64::from(audio.sample_rate));
            let _ = get_hogging_pid(device_id);

            let mut audio_unit = AudioUnit::new(IOType::DefaultOutput)?;
            let stream_format = StreamFormat {
                sample_rate: f64::from(audio.sample_rate),
                sample_format: SampleFormat::F32,
                flags: LinearPcmFlags::IS_FLOAT | LinearPcmFlags::IS_PACKED,
                channels: u32::from(audio.channels),
            };
            audio_unit.set_stream_format(stream_format, Scope::Input)?;

            let samples = Arc::new(Mutex::new(audio.samples));
            let cursor = Arc::new(Mutex::new(0_usize));
            let finish_fired = Arc::new(AtomicBool::new(false));
            let total_duration = audio.duration_secs;
            let sample_rate = audio.sample_rate;
            let channels = audio.channels;

            let samples_for_callback = Arc::clone(&samples);
            let cursor_for_callback = Arc::clone(&cursor);
            let stop_for_callback = Arc::clone(&stop_flag);
            let finish_for_callback = Arc::clone(&finish_fired);
            let progress_for_callback = Arc::clone(&progress_callback);
            let finish_callback_for_stream = Arc::clone(&finish_callback);

            audio_unit.set_render_callback(move |args: Args<Interleaved<f32>>| {
                let mut cursor = cursor_for_callback.lock().unwrap();
                let samples = samples_for_callback.lock().unwrap();

                for sample in args.data.buffer.iter_mut() {
                    let value = if stop_for_callback.load(Ordering::SeqCst) {
                        0.0
                    } else {
                        samples.get(*cursor).copied().unwrap_or(0.0)
                    };
                    *sample = value;
                    if *cursor < samples.len() {
                        *cursor += 1;
                    }
                }

                let progress_frames = (*cursor / channels as usize) as f64;
                let progress = progress_frames / f64::from(sample_rate.max(1));
                progress_for_callback(progress.min(total_duration), total_duration);

                if *cursor >= samples.len() && !finish_for_callback.swap(true, Ordering::SeqCst) {
                    finish_callback_for_stream();
                }

                Ok(())
            })?;

            audio_unit.start().context("Failed to start CoreAudio output")?;
            Ok(audio_unit)
        })();

        match setup_result {
            Ok(audio_unit) => {
                self.audio_unit = Some(audio_unit);
                Ok(())
            }
            Err(error) => {
                if let Some(hogged_device_id) = self.hogged_device_id.take() {
                    let _ = toggle_hog_mode(hogged_device_id);
                }
                self.is_exclusive = false;
                Err(error)
            }
        }
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut audio_unit) = self.audio_unit.take() {
            let _ = audio_unit.stop();
            let _ = audio_unit.uninitialize();
        }
        if let Some(device_id) = self.hogged_device_id.take() {
            let _ = toggle_hog_mode(device_id);
        }
        self.is_exclusive = false;
        Ok(())
    }
}