use anyhow::{Context, Result};
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use image::codecs::jpeg::JpegEncoder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 根据原始音频字节识别音频格式。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Unknown,
}

impl AudioFormat {
    /// 根据文件头字节判断音频格式。
    ///
    /// # 示例
    ///
    /// ```
    /// use siren_core::AudioFormat;
    ///
    /// assert_eq!(AudioFormat::detect(b"RIFF\0\0\0\0WAVE"), AudioFormat::Wav);
    /// assert_eq!(AudioFormat::detect(b"fLaC\0\0\0\0"), AudioFormat::Flac);
    /// ```
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

    /// 返回当前音频格式对应的默认文件扩展名。
    ///
    /// # 示例
    ///
    /// ```
    /// use siren_core::AudioFormat;
    ///
    /// assert_eq!(AudioFormat::Flac.extension(), "flac");
    /// assert_eq!(AudioFormat::Unknown.extension(), "bin");
    /// ```
    pub fn extension(self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::Unknown => "bin",
        }
    }
}

/// 用户选择的输出格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    /// 保持为 WAV 原始格式，不做转码。
    #[default]
    Wav,
    /// 将 WAV 转码为 FLAC，以便获得更小体积和更完整的标签支持。
    Flac,
    /// 保持为 MP3 原始格式，不做转码。
    Mp3,
}

impl OutputFormat {
    /// 返回用于界面展示的格式名称。
    ///
    /// # 示例
    ///
    /// ```
    /// use siren_core::OutputFormat;
    ///
    /// assert_eq!(OutputFormat::Wav.label(), "WAV (Lossless)");
    /// assert_eq!(OutputFormat::Mp3.label(), "MP3");
    /// ```
    pub fn label(self) -> &'static str {
        match self {
            OutputFormat::Wav => "WAV (Lossless)",
            OutputFormat::Flac => "FLAC (Lossless)",
            OutputFormat::Mp3 => "MP3",
        }
    }
}

/// 写入 FLAC Vorbis Comment 与封面块时使用的元数据。
///
/// 适用于在音频文件已经落盘后补齐标题、艺术家、曲序与封面等信息；调用方应保证
/// 目标文件确实是可写的 FLAC 文件。
pub struct FlacMetadata<'a> {
    /// 曲目标题。
    pub title: &'a str,
    /// 曲目艺术家列表。
    pub artists: &'a [String],
    /// 专辑名称。
    pub album: &'a str,
    /// 专辑艺术家列表。
    pub album_artists: &'a [String],
    /// 曲目序号。
    pub track_number: Option<u32>,
    /// 专辑总曲数。
    pub total_tracks: Option<u32>,
    /// 光盘序号。
    pub disc_number: Option<u32>,
    /// 总光盘数。
    pub total_discs: Option<u32>,
    /// 封面数据，格式为 `(mime_type, bytes)`。
    pub cover: Option<(&'static str, &'a [u8])>,
}

/// 清洗文件名片段中不适合落盘的字符。
///
/// # 示例
///
/// ```
/// use siren_core::audio::sanitize_filename;
///
/// assert_eq!(sanitize_filename("A/B:C"), "A_B_C");
/// assert_eq!(sanitize_filename("  hello  "), "hello");
/// ```
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

/// 将音频字节写入磁盘，并按需要执行 WAV → FLAC 转码。
///
/// 入参 `data` 为完整音频字节，`out_dir` 为输出目录，`base_name` 为目标文件基础名，
/// `output_format` 为调用方期望的输出格式。
///
/// - 当源格式是 WAV 且目标格式为 [`OutputFormat::Flac`] 时，会使用纯 Rust
///   路径完成 FLAC 编码。
/// - 其他情况下直接按检测到的源格式写出。
/// - 该接口会自动创建输出目录，并对文件名执行 [`sanitize_filename`] 清洗。
///
/// 返回最终写入的文件路径。
pub fn save_audio(
    data: &[u8],
    out_dir: &Path,
    base_name: &str,
    output_format: OutputFormat,
) -> Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let detected = AudioFormat::detect(data);
    let safe_name = sanitize_filename(base_name);

    let out_ext = match (detected, output_format) {
        (AudioFormat::Wav, OutputFormat::Flac) => "flac",
        (fmt, _) => fmt.extension(),
    };

    let out_path = out_dir.join(format!("{safe_name}.{out_ext}"));

    if detected == AudioFormat::Wav && output_format == OutputFormat::Flac {
        let cursor = std::io::Cursor::new(data);
        let mut reader = hound::WavReader::new(cursor).context("Failed to read WAV data")?;
        let spec = reader.spec();

        let samples: Vec<i32> = reader
            .samples::<i32>()
            .collect::<Result<_, _>>()
            .context("Failed to read WAV samples")?;

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

/// 根据图片魔数推断 MIME 类型。
///
/// 入参 `data` 为图片开头字节；返回值为识别出的稳定 MIME 类型，当前支持 PNG、
/// JPEG、GIF 与 WEBP，无法识别时返回 `None`。
///
/// # 示例
///
/// ```
/// use siren_core::audio::detect_image_mime;
///
/// assert_eq!(detect_image_mime(&[0xFF, 0xD8, 0xFF, 0x00]), Some("image/jpeg"));
/// assert_eq!(detect_image_mime(b"GIF89a"), Some("image/gif"));
/// ```
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

/// 将嵌入封面统一编码为 JPEG，以提升 FLAC 播放器兼容性。
///
/// 入参 `data` 为原始图片字节；如果输入本身已是 JPEG，会直接返回原始内容副本，
/// 否则会先解码图片再以 JPEG 重新编码。
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

/// 为已写出的 FLAC 文件写入标签与封面元数据。
///
/// 入参 `path` 为目标 FLAC 文件路径，`metadata` 描述要写入的文本标签与封面。
/// 该接口会覆盖已有的前封面块与对应标签字段，因此调用方应在文件内容稳定后再
/// 执行，避免后续写盘覆盖标签结果。
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
