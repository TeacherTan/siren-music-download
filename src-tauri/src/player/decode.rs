use anyhow::Context;
use hound::WavReader;
use rodio::{Decoder, Source};

#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
    pub duration_secs: f64,
}

enum AudioFmt {
    Wav,
    Flac,
    Mp3,
    Unknown,
}

fn detect_format(data: &[u8]) -> AudioFmt {
    if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WAVE") {
        AudioFmt::Wav
    } else if data.starts_with(b"fLaC") {
        AudioFmt::Flac
    } else if data.starts_with(b"ID3")
        || data.starts_with(&[0xFF, 0xFB])
        || data.starts_with(&[0xFF, 0xF3])
        || data.starts_with(&[0xFF, 0xF2])
    {
        AudioFmt::Mp3
    } else {
        AudioFmt::Unknown
    }
}

fn normalize_int_sample(value: i32, bits_per_sample: u16) -> f32 {
    let effective_bits = bits_per_sample.clamp(1, 32) as u32;
    let max_amplitude = ((1_i64 << (effective_bits - 1)) - 1) as f32;
    if max_amplitude <= 0.0 {
        return 0.0;
    }
    (value as f32 / max_amplitude).clamp(-1.0, 1.0)
}

fn decode_wav(data: &[u8]) -> anyhow::Result<(Vec<f32>, u16, u32)> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let mut reader = WavReader::new(cursor).context("Failed to read WAV header")?;
    let spec = reader.spec();
    let channels = spec.channels;
    let sample_rate = spec.sample_rate;
    let bits_per_sample = spec.bits_per_sample;

    let samples = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|sample| sample.map(|value| normalize_int_sample(value, bits_per_sample)))
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| sample.map(|value| value.clamp(-1.0, 1.0)))
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok((samples, channels, sample_rate))
}

fn decode_with_rodio(data: &[u8]) -> anyhow::Result<(Vec<f32>, u16, u32)> {
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let decoder = Decoder::new(cursor).context("Failed to decode audio")?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples = decoder.convert_samples::<f32>().collect::<Vec<f32>>();

    Ok((samples, channels, sample_rate))
}

pub fn decode_audio(data: &[u8]) -> anyhow::Result<DecodedAudio> {
    let format = detect_format(data);
    let (samples, channels, sample_rate) = match format {
        AudioFmt::Wav => decode_wav(data)?,
        AudioFmt::Flac | AudioFmt::Mp3 => decode_with_rodio(data)?,
        AudioFmt::Unknown => anyhow::bail!("Unsupported audio format"),
    };

    let frame_count = samples.len() as f64 / f64::from(channels.max(1));
    let duration_secs = frame_count / f64::from(sample_rate.max(1));

    Ok(DecodedAudio {
        samples,
        channels,
        sample_rate,
        duration_secs,
    })
}