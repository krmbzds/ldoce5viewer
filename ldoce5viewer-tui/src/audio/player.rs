//! Audio playback via `rodio`.
//!
//! Supports playing MP3 data from in-memory byte slices (pronunciations and
//! sound effects read from the IDM archives).

use std::io::Cursor;
use std::sync::{Arc, Mutex};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("No audio output device available: {0}")]
    NoDevice(String),
    #[error("Failed to decode audio data: {0}")]
    Decode(String),
    #[error("Playback error: {0}")]
    Playback(String),
}

/// Non-blocking audio player.  Keeps the output stream alive and reuses it
/// across calls to avoid repeated device initialisation overhead.
pub struct AudioPlayer {
    /// The output stream (must stay alive as long as the player is used).
    _stream:  OutputStream,
    handle:   OutputStreamHandle,
    /// Currently active sink (if any).
    sink:     Mutex<Option<Sink>>,
}

impl AudioPlayer {
    /// Create a new player, opening the default audio output device.
    pub fn new() -> Result<Self, AudioError> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|e| AudioError::NoDevice(e.to_string()))?;
        Ok(AudioPlayer {
            _stream: stream,
            handle,
            sink: Mutex::new(None),
        })
    }

    /// Play MP3 `data` asynchronously.  Any currently playing audio is stopped.
    pub fn play(&self, data: Vec<u8>) -> Result<(), AudioError> {
        // Stop existing playback
        self.stop();

        let cursor = Cursor::new(data);
        let decoder = Decoder::new(cursor)
            .map_err(|e| AudioError::Decode(e.to_string()))?;

        let sink = Sink::try_new(&self.handle)
            .map_err(|e| AudioError::Playback(e.to_string()))?;
        sink.append(decoder);
        // Detach: playback continues in the background thread managed by rodio.
        *self.sink.lock().unwrap() = Some(sink);
        Ok(())
    }

    /// Stop any currently playing audio.
    pub fn stop(&self) {
        if let Ok(mut guard) = self.sink.lock() {
            if let Some(s) = guard.take() {
                s.stop();
            }
        }
    }

    /// Returns `true` if audio is currently playing.
    pub fn is_playing(&self) -> bool {
        self.sink
            .lock()
            .map(|g| g.as_ref().map(|s| !s.empty()).unwrap_or(false))
            .unwrap_or(false)
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_error_display() {
        let e = AudioError::NoDevice("test".to_owned());
        assert!(e.to_string().contains("test"));
        let e = AudioError::Decode("bad format".to_owned());
        assert!(e.to_string().contains("bad format"));
    }

    /// We can't test actual playback in CI (no audio device), so we just
    /// verify that `new()` either succeeds or returns a graceful error.
    #[test]
    fn audio_player_new_does_not_panic() {
        let _result = AudioPlayer::new();
        // Either Ok or Err(NoDevice) is acceptable
    }
}
