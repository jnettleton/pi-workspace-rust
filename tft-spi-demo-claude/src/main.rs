// The display, shape, font, color and picture code builds on any platform so
// `cargo test` works on a PC; only the Pi hardware parts need Linux. Elsewhere
// most of the crate is unused by `main`, so don't warn about it there.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

#[cfg(target_os = "linux")]
mod app;
#[cfg(target_os = "linux")]
mod calibrate;
mod picture;
pub mod tft_display;
pub mod tft_spi;
pub mod tft_touch;
#[allow(dead_code)]
mod util;

/// Usage:
///   tft-spi-demo-claude [--calibrate]      demo: shapes, text, touch drawing
///   tft-spi-demo-claude [--fill] <image>   show a JPEG or PNG and exit
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("tft-spi-demo-claude needs a Raspberry Pi (Linux). On this platform use `cargo test`.");
    std::process::exit(1);
}
