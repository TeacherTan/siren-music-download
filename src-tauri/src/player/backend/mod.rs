use crate::player::decode::DecodedAudio;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub trait PlaybackBackend: Send {
    fn play(
        &mut self,
        audio: DecodedAudio,
        stop_flag: Arc<AtomicBool>,
        progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
        finish_callback: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<()>;

    fn stop(&mut self) -> Result<()>;
}

#[cfg(target_os = "macos")]
pub mod coreaudio;
#[cfg(not(target_os = "macos"))]
pub mod cpal;

#[cfg(target_os = "macos")]
pub fn create_backend() -> Result<Box<dyn PlaybackBackend>> {
    Ok(Box::new(coreaudio::CoreAudioBackend::new()?))
}

#[cfg(not(target_os = "macos"))]
pub fn create_backend() -> Result<Box<dyn PlaybackBackend>> {
    Ok(Box::new(cpal::CpalBackend::new()?))
}