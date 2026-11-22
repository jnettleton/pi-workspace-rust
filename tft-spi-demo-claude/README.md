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
| Interface | 4-wire SPI, 8-bit: DC low = command byte, DC high = parameter/pixel bytes |
| Color | 18-bit RGB666, 3 bytes per pixel (the ILI9486 only supports 18-bit color over SPI) |
| Color order | Panel is wired BGR; set with the MADCTL BGR bit |
| Touch controller | XPT2046, resistive, 12-bit readings (0–4095) |
| Touch interface | SPI on its own chip select (CE1), pen-down interrupt on TP_IRQ |

Some documentation for this board describes a 16-bit shift-register
interface (commands sent as `[0x00, cmd]`, 16-bit color). This unit works
with plain 8-bit SPI and 18-bit color, as found while getting `tft-spi-demo`
working.

## Pinout

| Signal | GPIO | Physical pin | Used for |
|---|---|---|---|
| SPI0 MOSI | 10 | 19 | Data to LCD and touch controller |
| SPI0 MISO | 9 | 21 | Data from touch controller |
| SPI0 SCLK | 11 | 23 | SPI clock for both |
| SPI0 CE0 (LCD_CS) | 8 | 24 | LCD chip select, active low |
| SPI0 CE1 (TP_CS) | 7 | 26 | Touch chip select, active low |
| LCD_RS (DC) | 24 | 18 | Low = command, high = data |
| RST | 25 | 22 | LCD reset, active low |
| TP_IRQ | 17 | 11 | Low while the panel is touched |

The kernel's SPI driver handles CE0 and CE1, so the display (`/dev/spidev0.0`)
and touch controller (`/dev/spidev0.1`) are opened as separate SPI devices,
each with its own clock speed.

## Settings used by this driver

| | |
|---|---|
| Display SPI clock | 31.2 MHz (above the ILI9486's rated SPI speed but works on this board; drop to 16 MHz if pixels get corrupted) |
| Touch SPI clock | 2 MHz (XPT2046 maximum is about 2.5 MHz) |
| SPI mode | Mode 0 |
| Pixel format (0x3A) | `0x66`: 18 bits per pixel |
| Max transfer | 4096 bytes (spidev default `bufsiz`); pixels are sent in 4095-byte chunks so no pixel is split |
| Full-screen update | 480 × 320 × 3 = 460,800 bytes, about 0.12 s at 31.2 MHz |
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
- Run as a user in the `spi` and `gpio` groups, or with `sudo`.

## Running

On the Pi:

```
cargo run --release                     # demo: rotations, shapes, text, touch drawing
cargo run --release -- --calibrate      # redo the touch calibration first
cargo run --release -- photo.jpg        # show a picture (fit, with borders)
cargo run --release -- --fill photo.jpg # show a picture filling the screen
```

The touch calibration is saved to `touch_calibration.txt` in the current
directory.

## Tests

`cargo test` runs on any platform, including Windows. The Pi-only parts
(`rppal`, the SPI and touch drivers, the app) are left out of non-Linux
builds, and the display is tested against a mock ILI9486 that paints what it
receives into a framebuffer.
