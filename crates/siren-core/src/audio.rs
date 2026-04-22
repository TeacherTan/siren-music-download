use anyhow::{Context, Result};
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use image::codecs::jpeg::JpegEncoder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Detected audio format from raw bytes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Metadata written into FLAC Vorbis comments and picture blocks.
pub struct FlacMetadata<'a> {
    pub title: &'a str,
    pub artists: &'a [String],
    pub album: &'a str,
    pub album_artists: &'a [String],
    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u32>,
    pub total_discs: Option<u32>,
    pub cover: Option<(&'static str, &'a [u8])>,
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
        let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
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

/// Guess the MIME type of embedded image data from its magic bytes.
pub fn detect_image_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        None
    }
}

/// Normalize embedded cover art to JPEG for broader FLAC player compatibility.
pub fn encode_cover_as_jpeg(data: &[u8]) -> Result<Vec<u8>> {
    if detect_image_mime(data) == Some("image/jpeg") {
        return Ok(data.to_vec());
    }

    let image = image::load_from_memory(data).context("Failed to decode cover image")?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);

    for pixel in rgba.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        let alpha = alpha as u16;
        let inv_alpha = 255_u16.saturating_sub(alpha);
        rgb.push(((red as u16 * alpha + 255 * inv_alpha) / 255) as u8);
        rgb.push(((green as u16 * alpha + 255 * inv_alpha) / 255) as u8);
        rgb.push(((blue as u16 * alpha + 255 * inv_alpha) / 255) as u8);
    }

    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 92);
    encoder
        .encode(&rgb, width, height, image::ColorType::Rgb8.into())
        .context("Failed to encode cover image as JPEG")?;
    Ok(jpeg)
}

/// Tag a FLAC file with metadata using metaflac.
pub fn tag_flac(path: &Path, metadata: &FlacMetadata<'_>) -> Result<()> {
    let mut tag = metaflac::Tag::read_from_path(path)
        .with_context(|| format!("Failed to open FLAC for tagging: {}", path.display()))?;

    {
        let vc = tag.vorbis_comments_mut();
        vc.set_title(vec![metadata.title.to_string()]);
        vc.set_album(vec![metadata.album.to_string()]);

        if metadata.artists.is_empty() {
            vc.remove_artist();
        } else {
            vc.set_artist(metadata.artists.to_vec());
        }

        if metadata.album_artists.is_empty() {
            vc.remove_album_artist();
        } else {
            vc.set_album_artist(metadata.album_artists.to_vec());
        }

        if let Some(track_number) = metadata.track_number {
            vc.set_track(track_number);
        } else {
            vc.remove_track();
        }

        if let Some(total_tracks) = metadata.total_tracks {
            vc.set_total_tracks(total_tracks);
            vc.set("TRACKTOTAL", vec![total_tracks.to_string()]);
        } else {
            vc.remove_total_tracks();
            vc.remove("TRACKTOTAL");
        }

        if let Some(disc_number) = metadata.disc_number {
            vc.set("DISCNUMBER", vec![disc_number.to_string()]);
        } else {
            vc.remove("DISCNUMBER");
        }

        if let Some(total_discs) = metadata.total_discs {
            vc.set("TOTALDISCS", vec![total_discs.to_string()]);
            vc.set("DISCTOTAL", vec![total_discs.to_string()]);
        } else {
            vc.remove("TOTALDISCS");
            vc.remove("DISCTOTAL");
        }
    }

    tag.remove_picture_type(metaflac::block::PictureType::CoverFront);

    if let Some((mime_type, cover)) = metadata.cover {
        tag.add_picture(
            mime_type.to_string(),
            metaflac::block::PictureType::CoverFront,
            cover.to_vec(),
        );
    }

    tag.save()
        .with_context(|| format!("Failed to save FLAC tags: {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        detect_image_mime, encode_cover_as_jpeg, save_audio, tag_flac, FlacMetadata, OutputFormat,
    };
    use anyhow::Result;

    fn build_test_wav() -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: 44_100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::new(&mut cursor, spec).expect("wav writer");
            for sample in [0_i16, 1024, -1024, 512, -512] {
                writer.write_sample(sample).expect("sample");
            }
            writer.finalize().expect("finalize");
        }
        cursor.into_inner()
    }

    #[test]
    fn writes_flac_vorbis_comments_after_wav_conversion() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let wav_bytes = build_test_wav();
        let flac_path = save_audio(&wav_bytes, temp_dir.path(), "test-song", OutputFormat::Flac)?;

        let artists = vec![String::from("Test Artist")];
        let album_artists = vec![String::from("Test Album Artist")];

        tag_flac(
            &flac_path,
            &FlacMetadata {
                title: "Test Song",
                artists: &artists,
                album: "Test Album",
                album_artists: &album_artists,
                track_number: Some(2),
                total_tracks: Some(9),
                disc_number: Some(1),
                total_discs: Some(1),
                cover: None,
            },
        )?;

        let tag = metaflac::Tag::read_from_path(&flac_path)?;
        let comments = tag
            .vorbis_comments()
            .ok_or_else(|| anyhow::anyhow!("missing vorbis comments"))?;

        assert_eq!(
            comments.title().map(|items| items.as_slice()),
            Some([String::from("Test Song")].as_slice())
        );
        assert_eq!(
            comments.artist().map(|items| items.as_slice()),
            Some([String::from("Test Artist")].as_slice())
        );
        assert_eq!(
            comments.album().map(|items| items.as_slice()),
            Some([String::from("Test Album")].as_slice())
        );
        assert_eq!(
            comments.album_artist().map(|items| items.as_slice()),
            Some([String::from("Test Album Artist")].as_slice())
        );
        assert_eq!(comments.track(), Some(2));
        assert_eq!(comments.total_tracks(), Some(9));
        assert_eq!(comments.get("DISCNUMBER"), Some(&vec![String::from("1")]));
        assert_eq!(comments.get("TOTALDISCS"), Some(&vec![String::from("1")]));

        Ok(())
    }

    #[test]
    fn detects_cover_mime_from_magic_bytes() {
        assert_eq!(
            detect_image_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            Some("image/png")
        );
        assert_eq!(
            detect_image_mime(&[0xFF, 0xD8, 0xFF, 0xDB]),
            Some("image/jpeg")
        );
        assert_eq!(detect_image_mime(b"GIF89a123"), Some("image/gif"));
        assert_eq!(detect_image_mime(b"RIFFxxxxWEBPvp8 "), Some("image/webp"));
        assert_eq!(detect_image_mime(b"not-an-image"), None);
    }

    #[test]
    fn converts_png_cover_to_jpeg() -> Result<()> {
        let rgba = image::RgbaImage::from_fn(2, 2, |x, y| match (x, y) {
            (0, 0) => image::Rgba([255, 0, 0, 255]),
            (1, 0) => image::Rgba([0, 255, 0, 128]),
            (0, 1) => image::Rgba([0, 0, 255, 255]),
            _ => image::Rgba([255, 255, 255, 0]),
        });
        let dynamic = image::DynamicImage::ImageRgba8(rgba);
        let mut png = std::io::Cursor::new(Vec::new());
        dynamic.write_to(&mut png, image::ImageFormat::Png)?;

        let jpeg = encode_cover_as_jpeg(&png.into_inner())?;

        assert_eq!(detect_image_mime(&jpeg), Some("image/jpeg"));
        assert!(!jpeg.is_empty(), "jpeg bytes should not be empty");

        Ok(())
    }
}
