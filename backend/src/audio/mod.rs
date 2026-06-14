//! Pure-Rust audio stack for chanting practice: capture + FLAC encode
//! (recorder) and decode + cpal output (player), replacing Qt Multimedia's
//! FFmpeg backend. See `docs/pure-rust-audio-backend.md`.

pub mod format;
pub mod recorder;
