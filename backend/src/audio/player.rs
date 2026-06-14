//! Audio playback: decode (symphonia) → cpal output stream.
//!
//! [`Player::load`] decodes the whole file into a mono `f32` buffer (chanting
//! clips are short) reusing the `symphonia` probe/decode pattern from
//! [`crate::waveform`], resamples it to the output device's rate, and opens a
//! cpal output stream. Both FLAC (user recordings) and MP3 (shipped reference
//! recordings) decode through the same path — the player never assumes FLAC.
//!
//! The playback logic — frame cursor, position/duration, seek, range + loop
//! boundary detection — lives in [`PlaybackCore`], which is independent of cpal
//! so it can be unit-tested directly. The cpal output callback only locks the
//! shared core and calls [`PlaybackCore::fill_output`]; position is the played
//! frame count, giving deterministic sub-100 ms accuracy and seeks with no async
//! race (so the Qt `MediaPlayer` seek/position workarounds are unnecessary).

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::logger::{error, warn};

/// Playback state, mirroring the three Qt `MediaPlayer` states the QML checks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

impl PlayerState {
    /// Integer value exposed to QML (matches the `AudioManager` state enum).
    pub fn as_i32(self) -> i32 {
        match self {
            PlayerState::Stopped => 0,
            PlayerState::Playing => 1,
            PlayerState::Paused => 2,
        }
    }
}

/// Convert a millisecond position to a frame index at `sample_rate`.
fn ms_to_frame(ms: i64, sample_rate: u32) -> usize {
    if ms <= 0 {
        return 0;
    }
    ((ms as u64).saturating_mul(sample_rate as u64) / 1000) as usize
}

/// Convert a frame index at `sample_rate` to a millisecond position.
fn frame_to_ms(frame: usize, sample_rate: u32) -> i64 {
    if sample_rate == 0 {
        return 0;
    }
    ((frame as u64).saturating_mul(1000) / sample_rate as u64) as i64
}

/// Pure, cpal-independent playback state: a mono sample buffer plus a frame
/// cursor and the range/loop bounds. The cpal callback drives it via
/// [`fill_output`](PlaybackCore::fill_output); tests drive it directly.
pub struct PlaybackCore {
    /// Mono samples at `sample_rate` (the output device's rate after resample).
    samples: Vec<f32>,
    sample_rate: u32,
    /// Next frame to play.
    cursor: usize,
    state: PlayerState,
    volume: f32,
    /// Loop-back target when looping a range.
    range_start: usize,
    /// Exclusive end frame; `None` plays to the end of the buffer.
    range_end: Option<usize>,
    looping: bool,
    /// Set when playback reaches a non-looping end; the owner clears it to emit
    /// a stop event. Avoids the audio thread touching any QObject directly.
    finished: bool,
}

impl PlaybackCore {
    /// Construct from a mono sample buffer at `sample_rate`, stopped at frame 0.
    pub fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        PlaybackCore {
            samples,
            sample_rate,
            cursor: 0,
            state: PlayerState::Stopped,
            volume: 1.0,
            range_start: 0,
            range_end: None,
            looping: false,
            finished: false,
        }
    }

    fn total_frames(&self) -> usize {
        self.samples.len()
    }

    /// Total length of the loaded audio in milliseconds.
    pub fn duration_ms(&self) -> i64 {
        frame_to_ms(self.total_frames(), self.sample_rate)
    }

    /// Current playback position in milliseconds.
    pub fn position_ms(&self) -> i64 {
        frame_to_ms(self.cursor, self.sample_rate)
    }

    pub fn state(&self) -> PlayerState {
        self.state
    }

    /// Return and clear the "playback reached a non-looping end" flag.
    pub fn take_finished(&mut self) -> bool {
        std::mem::replace(&mut self.finished, false)
    }

    pub fn play(&mut self) {
        if self.total_frames() == 0 {
            return;
        }
        // Resume from the start if a previous play ran to the end.
        if self.cursor >= self.range_end.unwrap_or_else(|| self.total_frames()) {
            self.cursor = self.range_start;
        }
        self.state = PlayerState::Playing;
    }

    pub fn pause(&mut self) {
        if self.state == PlayerState::Playing {
            self.state = PlayerState::Paused;
        }
    }

    /// Stop and rewind to the start of the current range (or the file).
    pub fn stop(&mut self) {
        self.state = PlayerState::Stopped;
        self.cursor = self.range_start;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Seek to `position_ms`, clamped to the loaded duration. Deterministic —
    /// the cursor is the single source of truth, so there is no post-seek race.
    pub fn seek_ms(&mut self, position_ms: i64) {
        let frame = ms_to_frame(position_ms, self.sample_rate).min(self.total_frames());
        self.cursor = frame;
    }

    /// Play `[start_ms, end_ms)`, looping back to `start_ms` at the end when
    /// `looping`. Clears the range (plays to end of file) if the bounds are
    /// invalid.
    pub fn play_range(&mut self, start_ms: i64, end_ms: i64, looping: bool) {
        let total = self.total_frames();
        let start = ms_to_frame(start_ms, self.sample_rate).min(total);
        let end = ms_to_frame(end_ms, self.sample_rate).min(total);
        if end <= start {
            warn(&format!(
                "play_range: invalid bounds start_ms={start_ms} end_ms={end_ms}; playing full file"
            ));
            self.clear_range();
            self.cursor = start;
            self.play();
            return;
        }
        self.range_start = start;
        self.range_end = Some(end);
        self.looping = looping;
        self.cursor = start;
        self.state = PlayerState::Playing;
    }

    /// Clear any active range/loop so playback runs to the end of the file.
    pub fn clear_range(&mut self) {
        self.range_start = 0;
        self.range_end = None;
        self.looping = false;
    }

    /// Fill an interleaved output buffer of `out_channels` channels, advancing
    /// the cursor. Writes silence when not playing; replicates the mono sample
    /// across all output channels. Loops at the range end or stops at a
    /// non-looping end (setting [`finished`](Self::take_finished)).
    pub fn fill_output(&mut self, out: &mut [f32], out_channels: usize) {
        if self.state != PlayerState::Playing || out_channels == 0 {
            out.iter_mut().for_each(|s| *s = 0.0);
            return;
        }

        let end = self.range_end.unwrap_or_else(|| self.total_frames());
        let frames = out.len() / out_channels;

        for f in 0..frames {
            if self.cursor >= end {
                if self.looping && self.range_end.is_some() {
                    self.cursor = self.range_start;
                } else {
                    self.state = PlayerState::Stopped;
                    self.finished = true;
                    // Silence the remainder of this output buffer.
                    out[f * out_channels..].iter_mut().for_each(|s| *s = 0.0);
                    return;
                }
            }

            let sample = self.samples[self.cursor] * self.volume;
            let base = f * out_channels;
            for ch in 0..out_channels {
                out[base + ch] = sample;
            }
            self.cursor += 1;
        }
    }
}

/// A loaded, playable audio file. Holds the cpal output stream open for the
/// lifetime of the handle; dropping it tears the stream (and its audio thread)
/// down.
///
/// The stream is **paused while not playing** (created paused, started on
/// [`play`](Player::play), paused again on [`pause`](Player::pause) /
/// [`stop`](Player::stop) / [`halt`](Player::halt)). Keeping it running while
/// idle makes ALSA/PulseAudio emit repeated `snd_pcm_avail_delay` I/O errors and
/// wastes the device, so the device is only active during playback.
pub struct Player {
    stream: cpal::Stream,
    core: Arc<Mutex<PlaybackCore>>,
}

impl Player {
    /// Decode `path` (blocking) and build the output stream. Convenience wrapper
    /// over [`decode_to_mono`] + [`from_samples`](Player::from_samples); the
    /// bridge decodes off the GUI thread and calls `from_samples` directly.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let (mono, src_rate) = decode_to_mono(path.as_ref())?;
        Self::from_samples(mono, src_rate)
    }

    /// Build the output stream from already-decoded mono samples at `src_rate`,
    /// resampling to the output device's rate. The stream is created **paused**
    /// — call [`play`](Player::play) to start. Building the stream is cheap, so
    /// this runs on the Qt thread (where the `!Send` stream must live) after the
    /// expensive decode has happened on a background thread.
    pub fn from_samples(mono: Vec<f32>, src_rate: u32) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("No output audio device available"))?;
        let supported = device
            .default_output_config()
            .context("No default output configuration for the audio device")?;

        let out_rate = supported.sample_rate();
        let out_channels = supported.channels() as usize;
        let sample_format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let resampled = crate::audio::format::resample_mono_to(&mono, src_rate, out_rate)?;

        let core = Arc::new(Mutex::new(PlaybackCore::new(resampled, out_rate)));

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_output::<f32>(&device, &config, &core, out_channels)?,
            cpal::SampleFormat::I16 => build_output::<i16>(&device, &config, &core, out_channels)?,
            cpal::SampleFormat::U16 => build_output::<u16>(&device, &config, &core, out_channels)?,
            cpal::SampleFormat::I32 => build_output::<i32>(&device, &config, &core, out_channels)?,
            cpal::SampleFormat::I8 => build_output::<i8>(&device, &config, &core, out_channels)?,
            cpal::SampleFormat::U8 => build_output::<u8>(&device, &config, &core, out_channels)?,
            other => return Err(anyhow!("Unsupported output sample format: {other:?}")),
        };

        // Leave the stream paused; play() starts it. Some backends auto-start, so
        // pause defensively to guarantee no audio until the user presses play.
        let _ = stream.pause();

        Ok(Player { stream, core })
    }

    /// A clone of the shared playback state. The cpal stream is `!Send` on some
    /// platforms, so the bridge's position-polling thread reads position/state
    /// (and [`PlaybackCore::take_finished`]) through this handle instead of the
    /// `Player` itself.
    pub fn shared_core(&self) -> Arc<Mutex<PlaybackCore>> {
        self.core.clone()
    }

    fn with_core<R>(&self, f: impl FnOnce(&mut PlaybackCore) -> R) -> R {
        let mut core = self
            .core
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut core)
    }

    pub fn play(&self) {
        let started = self.with_core(|c| {
            c.play();
            c.state() == PlayerState::Playing
        });
        if started {
            let _ = self.stream.play();
        }
    }

    pub fn pause(&self) {
        self.with_core(|c| c.pause());
        let _ = self.stream.pause();
    }

    pub fn stop(&self) {
        self.with_core(|c| c.stop());
        let _ = self.stream.pause();
    }

    /// Pause only the cpal stream without changing playback state. Used when the
    /// player reaches a non-looping end (state already moved to `Stopped` on the
    /// audio thread) so the device stops feeding silence.
    pub fn halt(&self) {
        let _ = self.stream.pause();
    }

    pub fn seek_ms(&self, position_ms: i64) {
        self.with_core(|c| c.seek_ms(position_ms));
    }

    pub fn set_volume(&self, volume: f32) {
        self.with_core(|c| c.set_volume(volume));
    }

    pub fn play_range(&self, start_ms: i64, end_ms: i64, looping: bool) {
        self.with_core(|c| c.play_range(start_ms, end_ms, looping));
        let _ = self.stream.play();
    }

    pub fn clear_range(&self) {
        self.with_core(|c| c.clear_range());
    }

    pub fn position_ms(&self) -> i64 {
        self.with_core(|c| c.position_ms())
    }

    pub fn duration_ms(&self) -> i64 {
        self.with_core(|c| c.duration_ms())
    }

    pub fn state(&self) -> PlayerState {
        self.with_core(|c| c.state())
    }

    /// Poll (and clear) the "playback reached a non-looping end" flag so the
    /// owner can emit a stop signal off the audio thread.
    pub fn take_finished(&self) -> bool {
        self.with_core(|c| c.take_finished())
    }
}

/// Build a cpal output stream for sample type `T`, pulling mono `f32` frames
/// from the shared [`PlaybackCore`] and converting them to `T`.
fn build_output<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    core: &Arc<Mutex<PlaybackCore>>,
    out_channels: usize,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let data_core = core.clone();
    let mut scratch: Vec<f32> = Vec::new();

    // Throttle the error callback: some ALSA/Pulse setups emit
    // `snd_pcm_avail_delay` errors on every period, which would flood the log.
    let mut last_error_log: Option<std::time::Instant> = None;

    let stream = device
        .build_output_stream(
            config.clone(),
            move |data: &mut [T], _| {
                scratch.resize(data.len(), 0.0);
                {
                    let mut core = data_core
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    core.fill_output(&mut scratch, out_channels);
                }
                for (dst, &src) in data.iter_mut().zip(scratch.iter()) {
                    *dst = T::from_sample(src);
                }
            },
            move |err| {
                let now = std::time::Instant::now();
                let should_log = last_error_log
                    .map(|t| now.duration_since(t) >= std::time::Duration::from_secs(2))
                    .unwrap_or(true);
                if should_log {
                    last_error_log = Some(now);
                    error(&format!("Player output stream error: {err}"));
                }
            },
            None,
        )
        .context("Failed to build the audio output stream")?;

    Ok(stream)
}

/// Decode a file into a [`PlaybackCore`] at the source sample rate (no device /
/// resample). Used for the player's tests and fixtures so the decode + boundary
/// logic can be exercised without an audio output device.
pub fn decode_file_to_core<P: AsRef<Path>>(path: P) -> Result<PlaybackCore> {
    let (mono, src_rate) = decode_to_mono(path.as_ref())?;
    Ok(PlaybackCore::new(mono, src_rate))
}

/// Decode an audio file fully into a mono `f32` buffer, returning the samples
/// and the source sample rate. Reuses the `symphonia` probe/decode pattern from
/// [`crate::waveform`]; both FLAC and MP3 decode through this path. Public so the
/// bridge can run the (potentially slow) decode off the GUI thread, then build
/// the stream via [`Player::from_samples`] on the Qt thread.
pub fn decode_to_mono(path: &Path) -> Result<(Vec<f32>, u32)> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open audio file: {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .with_context(|| format!("Failed to probe audio format: {}", path.display()))?;

    let mut format = probed.format;
    let track = format.default_track().context("No audio track found")?;
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(0);
    if sample_rate == 0 {
        return Err(anyhow!("Audio track has no sample rate"));
    }

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create audio decoder")?;

    let mut mono: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                warn(&format!("Player decode error, stopping: {e}"));
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(e) => {
                warn(&format!("Player packet decode error, skipping: {e}"));
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

        mono.reserve(num_frames);
        for frame in 0..num_frames {
            let mut sum = 0.0f32;
            for ch in 0..channels {
                sum += samples[frame * channels + ch];
            }
            mono.push(sum / channels as f32);
        }
    }

    if mono.is_empty() {
        return Err(anyhow!("Decoded audio is empty: {}", path.display()));
    }

    Ok((mono, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core_at(sample_rate: u32, frames: usize) -> PlaybackCore {
        PlaybackCore::new(vec![0.5; frames], sample_rate)
    }

    #[test]
    fn duration_and_position_math() {
        let core = core_at(48_000, 48_000); // 1 s
        assert_eq!(core.duration_ms(), 1000);
        assert_eq!(core.position_ms(), 0);
    }

    #[test]
    fn seek_rounds_and_clamps() {
        let mut core = core_at(48_000, 48_000);
        core.seek_ms(500);
        assert_eq!(core.position_ms(), 500);
        // Beyond duration clamps to the end.
        core.seek_ms(5000);
        assert_eq!(core.position_ms(), core.duration_ms());
        // Negative clamps to 0.
        core.seek_ms(-100);
        assert_eq!(core.position_ms(), 0);
    }

    #[test]
    fn fill_output_silent_when_stopped() {
        let mut core = core_at(48_000, 100);
        let mut out = [1.0f32; 8];
        core.fill_output(&mut out, 2);
        assert!(out.iter().all(|&s| s == 0.0));
        assert_eq!(core.position_ms(), 0);
    }

    #[test]
    fn fill_output_advances_cursor_and_replicates_channels() {
        let mut core = PlaybackCore::new(vec![0.5; 10], 48_000);
        core.play();
        let mut out = [0.0f32; 8]; // 4 stereo frames
        core.fill_output(&mut out, 2);
        // Every sample is the mono value across both channels.
        assert!(out.iter().all(|&s| (s - 0.5).abs() < 1e-6));
        // Cursor advanced by 4 frames.
        assert_eq!(core.position_ms(), frame_to_ms(4, 48_000));
    }

    #[test]
    fn volume_scales_output() {
        let mut core = PlaybackCore::new(vec![1.0; 10], 48_000);
        core.set_volume(0.25);
        core.play();
        let mut out = [0.0f32; 4];
        core.fill_output(&mut out, 1);
        assert!(out.iter().all(|&s| (s - 0.25).abs() < 1e-6));
        // Volume clamps to [0, 1].
        core.set_volume(2.0);
        assert_eq!({ core.volume }, 1.0);
    }

    #[test]
    fn natural_end_stops_and_sets_finished() {
        let mut core = PlaybackCore::new(vec![0.5; 4], 48_000);
        core.play();
        let mut out = [0.0f32; 8]; // request 8 mono frames, only 4 available
        core.fill_output(&mut out, 1);
        assert_eq!(core.state(), PlayerState::Stopped);
        assert!(core.take_finished());
        assert!(!core.take_finished(), "finished flag clears after read");
        // The played frames have audio, the tail is silence.
        assert!((out[0] - 0.5).abs() < 1e-6);
        assert_eq!(out[4], 0.0);
    }

    #[test]
    fn play_range_stops_at_end_without_loop() {
        // 1 s buffer at 1 kHz → 1000 frames, 1 ms per frame.
        let mut core = PlaybackCore::new(vec![0.5; 1000], 1000);
        core.play_range(100, 105, false);
        assert_eq!(core.position_ms(), 100);
        assert_eq!(core.state(), PlayerState::Playing);
        let mut out = [0.0f32; 20]; // more than the 5-frame range
        core.fill_output(&mut out, 1);
        assert_eq!(core.state(), PlayerState::Stopped);
        assert!(core.take_finished());
        // Stopped exactly at the range end (frame 105).
        assert_eq!(core.position_ms(), 105);
    }

    #[test]
    fn play_range_loops_back_to_start() {
        let mut core = PlaybackCore::new(vec![0.5; 1000], 1000);
        core.play_range(100, 105, true);
        // Pull more frames than the 5-frame range to force a wrap.
        let mut out = [0.0f32; 8];
        core.fill_output(&mut out, 1);
        // Still playing (looping), wrapped to 100 + (8 - 5) = 103.
        assert_eq!(core.state(), PlayerState::Playing);
        assert!(!core.take_finished());
        assert_eq!(core.position_ms(), 103);
    }

    #[test]
    fn play_range_invalid_bounds_plays_full_file() {
        let mut core = PlaybackCore::new(vec![0.5; 1000], 1000);
        core.play_range(200, 100, false);
        assert!(core.range_end.is_none(), "range cleared on invalid bounds");
        assert_eq!(core.state(), PlayerState::Playing);
        assert_eq!(core.position_ms(), 200);
    }

    #[test]
    fn stop_rewinds_to_range_start() {
        let mut core = PlaybackCore::new(vec![0.5; 1000], 1000);
        core.play_range(100, 200, false);
        core.seek_ms(150);
        core.stop();
        assert_eq!(core.state(), PlayerState::Stopped);
        assert_eq!(core.position_ms(), 100);
    }
}
