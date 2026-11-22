pub mod calibration;
pub mod filter;
#[cfg(target_os = "linux")]
pub mod tft_touch;

use std::error::Error;

/// Blocking touch input: what the calibration screens need. `TftTouch`
/// implements it on the Pi; tests use scripted touches.
pub trait TouchInput {
    /// Waits for a touch and returns a filtered raw reading (0..=4095 per axis).
    fn wait_for_touch(&mut self) -> Result<(u16, u16), Box<dyn Error>>;
    /// Waits until the panel has been released.
    fn wait_for_release(&mut self);
}
