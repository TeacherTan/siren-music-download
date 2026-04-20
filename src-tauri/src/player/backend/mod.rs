use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
use anyhow::Result;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub trait PlaybackBackend: Send {
    fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<AudioFormat>;

    fn play_stream(
        &mut self,
        format: AudioFormat,
        samples: SampleBuffer,
        stop_flag: Arc<AtomicBool>,
        volume: Arc<Mutex<f64>>,
        progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
        finish_callback: Arc<dyn Fn() + Send + Sync>,
        error_handler: PlaybackErrorHandler,
    ) -> Result<()>;

    fn pause(&mut self) -> Result<()>;

    fn resume(&mut self) -> Result<()>;

    fn stop(&mut self) -> Result<()>;
}

pub mod cpal;

pub fn create_backend() -> Result<Box<dyn PlaybackBackend>> {
    Ok(Box::new(cpal::CpalBackend::new()?))
}
