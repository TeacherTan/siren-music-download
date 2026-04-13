//! Shared core library for Siren Music Download
//!
//! This crate contains all platform-independent logic:
//! - API client for Monster Siren Records
//! - Audio processing (WAV/FLAC/MP3)
//! - Download logic and metadata tagging

pub mod api;
pub mod audio;
pub mod downloader;

// Re-export public API for convenience
pub use api::{ApiClient, Album, AlbumDetail, SongDetail, SongEntry};
pub use audio::{save_audio, tag_flac, AudioFormat, OutputFormat};
pub use downloader::{download_song, DownloadProgress, MetaOverride};