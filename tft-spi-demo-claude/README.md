# tft-spi-demo-claude

Rust driver and demo for a Waveshare 3.5" RPi LCD (A) touch display on a
Raspberry Pi 4: shapes, text, touch input with calibration, and a JPEG/PNG
viewer.

## Display specs

| | |
|---|---|
| Board | Waveshare 3.5" RPi LCD (A), plugs onto the 40-pin header |
| LCD controller | ILI9486 |
| Resolution | 320 × 480 native (portrait), 480 × 320 in landscape |
| Interface | SPI into shift registers that feed the ILI9486's 16-bit parallel bus: every command, parameter and pixel is a 16-bit word. DC low = command, DC high = parameter/pixel |
| Color | 16-bit RGB565, 2 bytes per pixel |
| Color order | Panel is wired BGR; set with the MADCTL BGR bit |
| Touch controller | XPT2046, resistive, 12-bit readings (0–4095) |
| Touch interface | SPI on its own chip select (CE1), pen-down interrupt on TP_IRQ |

`tft-spi-demo` sends single bytes and 18-bit color, as for a plain SPI
ILI9486. On this board that only partly works: the shift registers pair each
byte with its neighbor, so whether a command gets through depends on how many
bytes came before it. Full-screen fills look right, but rotation, address
windows and text don't. This driver sends 16-bit words as Waveshare's own
driver does: commands as `[0x00, cmd]`, each parameter as `[0x00, p]`, and
RGB565 pixels. `TFT_BUS=8` switches back to `tft-spi-demo`'s 8-bit way for
comparison.

## Pinout

| Signal | GPIO | Physical pin | Used for |
|---|---|---|---|
| SPI0 MOSI | 10 | 19 | Data to LCD and touch controller |
| SPI0 MISO | 9 | 21 | Data from touch controller |
| SPI0 SCLK | 11 | 23 | SPI clock for both |
| SPI0 CE0 (LCD_CS) | 8 | 24 | LCD chip select, active low (driven as a GPIO) |
| SPI0 CE1 (TP_CS) | 7 | 26 | Touch chip select, active low |
| LCD_RS (DC) | 24 | 18 | Low = command, high = data |
| RST | 25 | 22 | LCD reset, active low |
| TP_IRQ | 17 | 11 | Low while the panel is touched |

The display (`/dev/spidev0.0`) and touch controller (`/dev/spidev0.1`) are
opened as separate SPI devices, each with its own clock speed. The LCD's chip
select (GPIO 8) is driven as a plain GPIO and held low, as in `tft-spi-demo`:
CS going high resets the board's 16-bit word counter, and with 8-bit
transfers letting the SPI hardware raise CE0 after every transfer showed bars. Touch reads release it while they use CE1.

## Settings used by this driver

| | |
|---|---|
| Display SPI clock | 16 MHz requested, about 15.6 MHz actual on a Pi 4 (the Pi divides its 500 MHz core clock by an even number and rounds down). At `tft-spi-demo`'s 31.2 MHz (about 27.8 MHz) solid colors and text look fine but photo colors are badly off (some pixel bits land one position off). Override with `TFT_SPI_HZ` |
| Touch SPI clock | 2 MHz (XPT2046 maximum is about 2.5 MHz) |
| SPI mode | Mode 0 |
| Pixel format (0x3A) | `0x55`: 16 bits per pixel (`0x66`, 18 bits, with `TFT_BUS=8`) |
| Transfers | Whole 16-bit words only. One row of pixels per SPI transfer (at most 480 × 2 = 960 bytes) |
| Full-screen update | 480 × 320 × 2 = 307,200 bytes, about 0.16 s at 15.6 MHz |
| Touch commands | `0xD0` read X, `0x90` read Y (12-bit, differential) |

Rotation is set with MADCTL (0x36), always with the BGR bit (`0x08`):

| Rotation | MADCTL bits | Value | Size |
|---|---|---|---|
| `Degrees0` | MY | `0x88` | 320 × 480 |
| `Degrees90` | MV | `0x28` | 480 × 320 |
| `Degrees180` | MX | `0x48` | 320 × 480 |
| `Degrees270` | MX, MY, MV | `0xE8` | 480 × 320 |

## Raspberry Pi setup

- Enable SPI: `dtparam=spi=on` in `/boot/firmware/config.txt` (`/boot/config.txt`
  before Bookworm), or with `raspi-config`.
- Don't load the Waveshare `waveshare35a` overlay or LCD-show driver; they
  take over the display.
- Keep the SPI clock steady: add `core_freq_min=500` to
  `/boot/firmware/config.txt` and reboot. See below.
- Run as a user in the `spi` and `gpio` groups, or with `sudo`.

### SPI clock and the core clock

The Pi's SPI clock is divided from the GPU core clock: about 15.6 MHz is
500 MHz / 32. With the default `ondemand` CPU governor the firmware drops the
core clock to 200 MHz while the CPU is idle, and the SPI clock drops with it to
6.25 MHz. Full-screen fills and images send one large transfer per row and
leave the CPU idle, so they hit this, while text (many tiny transfers that
keep the CPU busy) doesn't.

Measured with `--bench` on a Pi 4:

| | Default (`ondemand`) | `core_freq_min=500` (`ondemand`) | `performance` governor |
|---|---|---|---|
| `fill_screen` | 165–413 ms, varies run to run | 166 ms | 166 ms |
| `draw_image` | 413 ms | 167 ms | 167 ms |
| `draw_text` (800 chars) | 242 ms | 242 ms | 238 ms |

`core_freq_min=500` fixes only the core clock; the CPU governor still
saves power when idle. This version still runs at full speed: its per-row
work keeps the CPU busy enough that `ondemand` doesn't slow it down. (The C++
port does less per row, lets the CPU slow down, and takes 175 ms.) To try
the `performance` governor without rebooting:
`echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`
(and `ondemand` to put it back).

## Running

On the Pi:

```
cargo pi4-native                        # build only (alias for `cargo build --release`)
cargo run --release                     # demo: rotations, shapes, text, touch drawing
cargo run --release -- --calibrate      # redo the touch calibration first
cargo run --release -- photo.jpg        # show a picture (fit, with borders)
cargo run --release -- --fill photo.jpg # show a picture filling the screen
cargo run --release -- --bench          # time full-screen drawing (10 runs each) and exit
```

`--bench` prints min/avg/max times for a full-screen fill, a full-screen
image and a screen of size-2 text. The C++ port prints the same lines, so
the two can be compared run for run.

The touch calibration is saved to `touch_calibration.txt` in the current
directory.

## Tests

`cargo test` runs on any platform, including Windows (82 tests, a few
seconds). The Pi-only parts (`rppal`, the SPI and touch drivers, the app's
main loop) are left out of non-Linux builds.

Display tests use a mock ILI9486 (`src/tft_display/mock.rs`) that decodes the
commands the driver sends and paints the pixels into a framebuffer. It also
models the panel's physical memory, applying the MADCTL rotation bits, and
logs every command, so tests can check exactly what would be on screen.

What's covered:

| Area | Tests |
|---|---|
| `fill_rectangle`, `draw_text`, `draw_image` | Every pixel on screen, clipping at all edges, nothing sent when off screen, one transfer per row |
| Lines, circles, rectangles, rounded rectangles | Pixel-for-pixel against plain per-pixel versions of the algorithms; clipped at every edge; no overflow at extreme coordinates |
| Rotations | MADCTL values verified on the hardware are pinned; each rotation covers the whole panel once; the four are consistent quarter turns with no mirroring |
| Initialization | The known-working command sequence, 18-bit pixel format, reset and ILI9486 timing |
| Touch | Filtering a burst of readings (median, dropping stuck values); the calibration math; the calibration screens with scripted touches (target positions, retries, saving/loading the file, per rotation) |
| Pictures | JPEG/PNG loading, EXIF orientation, bad files, Fit/Fill scaling and cropping, the viewer's centering and borders |
| Demo screen | Button layout (rounded, gaps, centered labels), pressed state, which button a touch hits |

Not covered (needs the hardware): the `rppal` SPI/GPIO calls, reading the
XPT2046 touch controller, and the demo's touch-drawing loop.
