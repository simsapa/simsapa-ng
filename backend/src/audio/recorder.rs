//! Microphone capture to FLAC.
//!
//! Opens the default input device via `cpal`, accumulates the captured frames
//! (converted to `f32`), and on `stop` normalizes them to the canonical mono /
//! 16-bit / 48 kHz PCM format ([`crate::audio::format`]) and encodes a `.flac`
//! file with `flacenc`.
//!
//! Capture runs on cpal's own audio thread; the caller only holds the
//! [`Recorder`] handle. Dropping the handle (or calling [`Recorder::stop`])
//! tears down the stream. Per the PRD, PCM is buffered in memory and encoded on
//! stop — chanting clips are short, so this keeps the capture callback cheap and
//! avoids blocking the UI thread during recording.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

use crate::audio::format::{
    self, CANONICAL_BITS_PER_SAMPLE, CANONICAL_CHANNELS, CANONICAL_SAMPLE_RATE,
};
use crate::logger::error;

/// Capture state shared between the cpal callback thread and the owner.
struct Shared {
    /// Interleaved `f32` samples in the device's native rate/channel layout.
    samples: Mutex<Vec<f32>>,
    /// First stream error reported by cpal, if any.
    error: Mutex<Option<String>>,
}

/// A live microphone capture. Holds the cpal input stream open until dropped or
/// [`stop`](Recorder::stop)ped.
pub struct Recorder {
    stream: cpal::Stream,
    shared: Arc<Shared>,
    output_path: PathBuf,
    in_sample_rate: u32,
    in_channels: u16,
}

impl Recorder {
    /// Start capturing from the default input device to `output_flac_path`
    /// (written on [`stop`](Recorder::stop)). Returns a clear error if no input
    /// device is available.
    pub fn start<P: AsRef<Path>>(output_flac_path: P) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No input audio device available"))?;

        let supported = device
            .default_input_config()
            .context("No default input configuration for the audio device")?;

        let in_sample_rate = supported.sample_rate();
        let in_channels = supported.channels();
        let sample_format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let shared = Arc::new(Shared {
            samples: Mutex::new(Vec::new()),
            error: Mutex::new(None),
        });

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, &shared)?,
            cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, &shared)?,
            cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config, &shared)?,
            cpal::SampleFormat::I32 => build_stream::<i32>(&device, &config, &shared)?,
            cpal::SampleFormat::I8 => build_stream::<i8>(&device, &config, &shared)?,
            cpal::SampleFormat::U8 => build_stream::<u8>(&device, &config, &shared)?,
            other => {
                return Err(anyhow!("Unsupported input sample format: {other:?}"));
            }
        };

        stream
            .play()
            .context("Failed to start the audio input stream")?;

        Ok(Recorder {
            stream,
            shared,
            output_path: output_flac_path.as_ref().to_path_buf(),
            in_sample_rate,
            in_channels,
        })
    }

    /// Return (and clear) the first capture error reported by cpal, if any.
    /// Lets the bridge poll for asynchronous stream failures during recording.
    pub fn take_error(&self) -> Option<String> {
        self.shared
            .error
            .lock()
            .ok()
            .and_then(|mut e| e.take())
    }

    /// Stop capturing, normalize to canonical PCM, and write the FLAC file.
    pub fn stop(self) -> Result<()> {
        // Dropping the stream stops the cpal callback thread.
        drop(self.stream);

        if let Some(err) = self.shared.error.lock().ok().and_then(|e| e.clone()) {
            // Surface the stream error but still write whatever was captured.
            error(&format!("Recorder stream error: {err}"));
        }

        let samples = self
            .shared
            .samples
            .lock()
            .map_err(|_| anyhow!("Capture buffer lock poisoned"))?
            .clone();

        let pcm = format::to_canonical(&samples, self.in_sample_rate, self.in_channels)?;
        encode_flac_to_path(&pcm, &self.output_path)?;
        Ok(())
    }
}

/// Build a cpal input stream for sample type `T`, converting each sample to
/// `f32` and appending it to the shared capture buffer.
fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: &Arc<Shared>,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let data_shared = shared.clone();
    let err_shared = shared.clone();

    let stream = device
        .build_input_stream(
            config.clone(),
            move |data: &[T], _| {
                if let Ok(mut buf) = data_shared.samples.lock() {
                    buf.extend(data.iter().map(|&s| s.to_sample::<f32>()));
                }
            },
            move |err| {
                if let Ok(mut slot) = err_shared.error.lock() {
                    if slot.is_none() {
                        *slot = Some(format!("{err}"));
                    }
                }
            },
            None,
        )
        .context("Failed to build the audio input stream")?;

    Ok(stream)
}

/// Encode canonical mono 16-bit PCM to a FLAC file at `path`.
///
/// The PCM is zero-padded up to a whole FLAC block so every frame is full
/// length and STREAMINFO reports `min_blocksize == max_blocksize`. `flacenc`
/// otherwise records the short final frame as the stream's minimum block size,
/// which signals a variable-blocksize stream that `symphonia` (our decoder /
/// waveform path) refuses to parse. The padding adds at most one block of
/// trailing silence (~85 ms at 48 kHz) — harmless for chanting clips.
fn encode_flac_to_path(pcm: &[i16], path: &Path) -> Result<()> {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| anyhow!("FLAC encoder config error: {e:?}"))?;

    let block_size = config.block_size;
    let mut samples_i32: Vec<i32> = pcm.iter().map(|&s| s as i32).collect();
    let remainder = samples_i32.len() % block_size;
    if remainder != 0 {
        samples_i32.resize(samples_i32.len() + (block_size - remainder), 0);
    }

    let source = flacenc::source::MemSource::from_samples(
        &samples_i32,
        CANONICAL_CHANNELS as usize,
        CANONICAL_BITS_PER_SAMPLE as usize,
        CANONICAL_SAMPLE_RATE as usize,
    );

    let flac_stream =
        flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
            .map_err(|e| anyhow!("FLAC encode error: {e:?}"))?;

    let mut sink = flacenc::bitsink::ByteSink::new();
    flac_stream
        .write(&mut sink)
        .map_err(|e| anyhow!("FLAC serialization error: {e:?}"))?;

    std::fs::write(path, sink.as_slice())
        .with_context(|| format!("Failed to write FLAC file: {}", path.display()))?;

    Ok(())
}

/// Encode canonical PCM to a FLAC file — exposed for tests and the player's
/// fixture generation (shares the exact encode path used by the recorder).
pub fn encode_canonical_pcm_to_flac(pcm: &[i16], path: &Path) -> Result<()> {
    encode_flac_to_path(pcm, path)
}
