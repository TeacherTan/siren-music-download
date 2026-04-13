use anyhow::{Context, Result};
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use std::path::{Path, PathBuf};

/// Detected audio format from raw bytes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Unknown,
}

impl AudioFormat {
    pub fn detect(data: &[u8]) -> Self {
        if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WAVE") {
            AudioFormat::Wav
        } else if data.starts_with(b"ID3")
            || data.starts_with(&[0xFF, 0xFB])
            || data.starts_with(&[0xFF, 0xF3])
            || data.starts_with(&[0xFF, 0xF2])
        {
            AudioFormat::Mp3
        } else if data.starts_with(b"fLaC") {
            AudioFormat::Flac
        } else {
            AudioFormat::Unknown
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::Unknown => "bin",
        }
    }
}

/// Output format chosen by user
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OutputFormat {
    /// Keep as WAV (lossless, direct from API — no conversion needed)
    #[default]
    Wav,
    /// Convert WAV → FLAC in pure Rust (lossless, smaller, better metadata)
    Flac,
    /// Keep MP3 as-is
    Mp3,
}

impl OutputFormat {
    pub fn label(self) -> &'static str {
        match self {
            OutputFormat::Wav => "WAV (Lossless)",
            OutputFormat::Flac => "FLAC (Lossless)",
            OutputFormat::Mp3 => "MP3",
        }
    }
}

/// Sanitize a string for use as a filename component
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Save audio bytes to disk.
/// - WAV + OutputFormat::Flac: converts in pure Rust via flacenc
/// - Everything else: written as-is with appropriate extension
///
/// Returns the path of the file written.
pub fn save_audio(
    data: &[u8],
    out_dir: &Path,
    base_name: &str,
    output_format: OutputFormat,
) -> Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let detected = AudioFormat::detect(data);
    let safe_name = sanitize_filename(base_name);

    // Decide actual output extension
    let out_ext = match (detected, output_format) {
        (AudioFormat::Wav, OutputFormat::Flac) => "flac",
        (fmt, _) => fmt.extension(),
    };

    let out_path = out_dir.join(format!("{safe_name}.{out_ext}"));

    if detected == AudioFormat::Wav && output_format == OutputFormat::Flac {
        // Decode WAV samples with hound
        let cursor = std::io::Cursor::new(data);
        let mut reader = hound::WavReader::new(cursor).context("Failed to read WAV data")?;
        let spec = reader.spec();

        let samples: Vec<i32> = reader
            .samples::<i32>()
            .collect::<Result<_, _>>()
            .context("Failed to read WAV samples")?;

        // Encode to FLAC in pure Rust via flacenc
        let config = flacenc::config::Encoder::default()
            .into_verified()
            .map_err(|e| anyhow::anyhow!("FLAC encoder config error: {:?}", e))?;
        let source = flacenc::source::MemSource::from_samples(
            &samples,
            spec.channels as usize,
            spec.bits_per_sample as usize,
            spec.sample_rate as usize,
        );
        let flac_stream =
            flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
                .map_err(|e| anyhow::anyhow!("FLAC encoding failed: {:?}", e))?;
        let mut sink = flacenc::bitsink::ByteSink::new();
        flac_stream
            .write(&mut sink)
            .map_err(|e| anyhow::anyhow!("FLAC write failed: {:?}", e))?;

        std::fs::write(&out_path, sink.as_slice()).context("Failed to write FLAC file")?;
    } else {
        std::fs::write(&out_path, data).context("Failed to write audio file")?;
    }

    Ok(out_path)
}

/// Tag a FLAC file with metadata using metaflac.
pub fn tag_flac(
    path: &Path,
    title: &str,
    artists: &[String],
    album: &str,
    cover_jpeg: Option<&[u8]>,
) -> Result<()> {
    let mut tag = metaflac::Tag::read_from_path(path)
        .with_context(|| format!("Failed to open FLAC for tagging: {}", path.display()))?;

    {
        let vc = tag.vorbis_comments_mut();
        vc.set_title(vec![title.to_string()]);
        vc.set_artist(artists.to_vec());
        vc.set_album(vec![album.to_string()]);
    }

    if let Some(cover) = cover_jpeg {
        tag.add_picture(
            "image/jpeg".to_string(),
            metaflac::block::PictureType::CoverFront,
            cover.to_vec(),
        );
    }

    tag.save()
        .with_context(|| format!("Failed to save FLAC tags: {}", path.display()))?;
    Ok(())
}
