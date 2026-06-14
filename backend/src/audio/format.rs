//! Canonical recording format and conversion helpers.
//!
//! All recordings are normalized to a single canonical PCM format regardless of
//! the capture device's native configuration: **mono, 16-bit, 48 kHz**. The
//! `to_canonical` helper downmixes interleaved input to mono and resamples to
//! 48 kHz (via `rubato`) before the PCM is handed to the FLAC encoder.

use anyhow::Result;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    calculate_cutoff, Async, FixedAsync, Resampler, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

/// Canonical sample rate for all recordings (Hz).
pub const CANONICAL_SAMPLE_RATE: u32 = 48_000;
/// Canonical channel count for all recordings (mono).
pub const CANONICAL_CHANNELS: u16 = 1;
/// Canonical bit depth for all recordings.
pub const CANONICAL_BITS_PER_SAMPLE: u16 = 16;

/// Downmix interleaved `f32` samples to a mono `f32` buffer by averaging the
/// channels of each frame. A `channels` count of 0 is treated as 1.
fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let channels = channels.max(1) as usize;
    if channels == 1 {
        return samples.to_vec();
    }
    let frames = samples.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let mut sum = 0.0f32;
        for ch in 0..channels {
            sum += samples[frame * channels + ch];
        }
        mono.push(sum / channels as f32);
    }
    mono
}

/// Resample a mono `f32` buffer from `in_rate` to [`CANONICAL_SAMPLE_RATE`]
/// using a sinc interpolator. Returns the input unchanged when the rate already
/// matches or the buffer is empty.
fn resample_mono(mono: &[f32], in_rate: u32) -> Result<Vec<f32>> {
    if mono.is_empty() || in_rate == CANONICAL_SAMPLE_RATE {
        return Ok(mono.to_vec());
    }

    let f_ratio = CANONICAL_SAMPLE_RATE as f64 / in_rate as f64;

    let sinc_len = 128;
    let oversampling_factor = 256;
    let window = WindowFunction::Blackman2;
    let f_cutoff = calculate_cutoff(sinc_len, window);
    let params = SincInterpolationParameters {
        sinc_len,
        f_cutoff,
        interpolation: SincInterpolationType::Quadratic,
        oversampling_factor,
        window,
    };

    let mut resampler =
        Async::<f32>::new_sinc(f_ratio, 1.1, &params, 1024, 1, FixedAsync::Input)?;

    let nbr_input_frames = mono.len();
    let out_capacity = resampler.process_all_needed_output_len(nbr_input_frames);
    let mut outdata = vec![0.0f32; out_capacity];

    let input_adapter = InterleavedSlice::new(mono, 1, nbr_input_frames)
        .map_err(|e| anyhow::anyhow!("resample input adapter: {e}"))?;
    let mut output_adapter = InterleavedSlice::new_mut(&mut outdata, 1, out_capacity)
        .map_err(|e| anyhow::anyhow!("resample output adapter: {e}"))?;

    let (_nbr_in, nbr_out) =
        resampler.process_all_into_buffer(&input_adapter, &mut output_adapter, nbr_input_frames, None)?;

    drop(output_adapter);
    outdata.truncate(nbr_out);
    Ok(outdata)
}

/// Convert a slice of `f32` samples in the range `[-1.0, 1.0]` to `i16` PCM,
/// clamping out-of-range values.
fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            (clamped * i16::MAX as f32).round() as i16
        })
        .collect()
}

/// Convert an interleaved `f32` capture buffer of arbitrary channel count and
/// sample rate into canonical mono / 16-bit / 48 kHz PCM, ready for FLAC
/// encoding.
pub fn to_canonical(samples: &[f32], in_rate: u32, in_channels: u16) -> Result<Vec<i16>> {
    let mono = downmix_to_mono(samples, in_channels);
    let resampled = resample_mono(&mono, in_rate)?;
    Ok(f32_to_i16(&resampled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_stereo_to_mono() {
        // Two stereo frames: (0.0, 1.0) -> 0.5, (-1.0, 1.0) -> 0.0
        let stereo = [0.0, 1.0, -1.0, 1.0];
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono, vec![0.5, 0.0]);
    }

    #[test]
    fn passthrough_when_rate_matches() {
        let mono = vec![0.0, 0.5, -0.5, 1.0];
        let out = resample_mono(&mono, CANONICAL_SAMPLE_RATE).unwrap();
        assert_eq!(out, mono);
    }

    #[test]
    fn resample_changes_length_by_ratio() {
        // 24 kHz -> 48 kHz should roughly double the frame count.
        let mono: Vec<f32> = (0..2400)
            .map(|i| (i as f32 * 0.05).sin())
            .collect();
        let out = resample_mono(&mono, 24_000).unwrap();
        let expected = mono.len() * 2;
        let diff = (out.len() as i64 - expected as i64).unsigned_abs();
        // Allow a small margin for the resampler's edge handling.
        assert!(diff < 64, "got {} expected ~{}", out.len(), expected);
    }

    #[test]
    fn f32_to_i16_clamps_and_scales() {
        let out = f32_to_i16(&[0.0, 1.0, -1.0, 2.0, -2.0]);
        assert_eq!(out, vec![0, i16::MAX, -i16::MAX, i16::MAX, -i16::MAX]);
    }

    #[test]
    fn to_canonical_stereo_passthrough_rate() {
        let stereo = [0.0, 0.0, 0.5, 0.5];
        let out = to_canonical(&stereo, CANONICAL_SAMPLE_RATE, 2).unwrap();
        assert_eq!(out.len(), 2);
    }
}
