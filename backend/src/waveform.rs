use std::path::Path;

use anyhow::{Context, Result};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::logger::warn;

/// Read the duration of an audio file in milliseconds from its container
/// metadata (no full decode). Returns 0 if the file can't be probed or the
/// codec doesn't expose a frame count + sample rate.
pub fn get_audio_duration_ms(file_path: &str) -> i32 {
    let path = Path::new(file_path);
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            warn(&format!("get_audio_duration_ms: open failed {}: {}", file_path, e));
            return 0;
        }
    };

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = match symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
    {
        Ok(p) => p,
        Err(e) => {
            warn(&format!("get_audio_duration_ms: probe failed {}: {}", file_path, e));
            return 0;
        }
    };

    let track = match probed.format.default_track() {
        Some(t) => t,
        None => return 0,
    };

    let params = &track.codec_params;
    let sample_rate = params.sample_rate.unwrap_or(0) as u64;
    let n_frames = params.n_frames.unwrap_or(0);
    if sample_rate == 0 || n_frames == 0 {
        return 0;
    }

    let ms = (n_frames.saturating_mul(1000)) / sample_rate;
    i32::try_from(ms).unwrap_or(i32::MAX)
}

/// Extract waveform peak amplitude data from an audio file.
///
/// Decodes the audio file, divides all samples into `num_bars` time buckets,
/// and returns the peak amplitude (normalized 0.0–1.0) for each bucket.
///
/// Returns an empty vec if the file cannot be decoded.
pub fn get_waveform_peaks(file_path: &str, num_bars: usize) -> Result<Vec<f32>> {
    if num_bars == 0 {
        return Ok(vec![]);
    }

    let path = Path::new(file_path);
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open audio file: {}", file_path))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .with_context(|| format!("Failed to probe audio format: {}", file_path))?;

    let mut format = probed.format;

    let track = format.default_track()
        .context("No audio track found")?;

    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create audio decoder")?;

    // First pass: collect all samples to find total count and global peak
    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                warn(&format!("Waveform decode error, stopping: {}", e));
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(e) => {
                warn(&format!("Waveform packet decode error, skipping: {}", e));
                continue;
            }
        };

        let spec = *decoded.spec();
        let num_frames = decoded.frames();

        if num_frames == 0 {
            continue;
        }

        let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let samples = sample_buf.samples();
        let channels = spec.channels.count().max(1);

        // Mix down to mono by averaging channels, take absolute value
        for frame in 0..num_frames {
            let mut sum: f32 = 0.0;
            for ch in 0..channels {
                sum += samples[frame * channels + ch].abs();
            }
            all_samples.push(sum / channels as f32);
        }
    }

    if all_samples.is_empty() {
        return Ok(vec![0.0; num_bars]);
    }

    // Divide samples into num_bars buckets and compute peak per bucket
    let samples_per_bar = all_samples.len() as f64 / num_bars as f64;
    let mut peaks = Vec::with_capacity(num_bars);

    for i in 0..num_bars {
        let start = (i as f64 * samples_per_bar) as usize;
        let end = ((i + 1) as f64 * samples_per_bar) as usize;
        let end = end.min(all_samples.len());

        let peak = if start < end {
            all_samples[start..end]
                .iter()
                .copied()
                .fold(0.0f32, f32::max)
        } else {
            0.0
        };

        peaks.push(peak);
    }

    // Normalize to 0.0–1.0 range
    let global_peak = peaks.iter().copied().fold(0.0f32, f32::max);
    if global_peak > 0.0 {
        for p in &mut peaks {
            *p /= global_peak;
        }
    }

    Ok(peaks)
}
