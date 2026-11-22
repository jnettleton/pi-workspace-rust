//! Loading JPEG/PNG files and scaling them for the display.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use image::{imageops::FilterType, DynamicImage, ImageDecoder, ImageReader, ImageResult};

use crate::tft_display::{color::Color, enums::Rotate, tft_display::TftDisplay};
use crate::tft_spi::TftSpi;

/// How to scale a picture to the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    /// Whole picture visible, scaled up or down; may leave borders.
    Fit,
    /// Screen completely covered, scaled and center-cropped.
    Fill,
}

/// RGB pixels ready for `TftDisplay::draw_image`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u16,
    pub height: u16,
    /// `width * height` pixels, 3 bytes each (R, G, B)
    pub rgb: Vec<u8>,
}

/// Opens a JPEG or PNG (format detected from the contents) and applies its
/// EXIF orientation, so phone photos come out the right way up.
pub fn open(path: &Path) -> ImageResult<DynamicImage> {
    let mut decoder = ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image)
}

/// Scales `image` to a `width` x `height` screen. Keeps the aspect ratio.
pub fn scale(image: &DynamicImage, width: u16, height: u16, scale: Scale) -> Picture {
    let (w, h) = (u32::from(width), u32::from(height));
    let scaled = match scale {
        Scale::Fit => image.resize(w, h, FilterType::Triangle),
        // Crop the source to the screen's shape first, then resize. (Resizing
        // first, as `resize_to_fill` does, makes a huge intermediate image for
        // very thin pictures: a 5000x1 strip would become 1,600,000x320.)
        Scale::Fill => {
            let (x, y, crop_w, crop_h) = center_crop(image.width(), image.height(), w, h);
            image.crop_imm(x, y, crop_w, crop_h).resize_exact(w, h, FilterType::Triangle)
        }
    };
    let rgb = if scaled.color().has_alpha() {
        over_black(scaled.into_rgba8())
    } else {
        scaled.into_rgb8()
    };
    Picture {
        // Both are at most width/height, so they fit in u16
        width: rgb.width() as u16,
        height: rgb.height() as u16,
        rgb: rgb.into_raw(),
    }
}

/// Blends a picture with transparency over a black background. (Just dropping
/// the alpha channel would show whatever color is stored under transparent
/// pixels.)
fn over_black(rgba: image::RgbaImage) -> image::RgbImage {
    let (w, h) = rgba.dimensions();
    let mut rgb = image::RgbImage::new(w, h);
    for (out, px) in rgb.pixels_mut().zip(rgba.pixels()) {
        let alpha = u16::from(px[3]);
        let blend = |c: u8| ((u16::from(c) * alpha + 127) / 255) as u8;
        *out = image::Rgb([blend(px[0]), blend(px[1]), blend(px[2])]);
    }
    rgb
}

/// The largest centered part of an `image_w` x `image_h` picture with the
/// same shape as a `w` x `h` screen, as (x, y, width, height).
fn center_crop(image_w: u32, image_h: u32, w: u32, h: u32) -> (u32, u32, u32, u32) {
    let (iw, ih, w, h) = (u64::from(image_w), u64::from(image_h), u64::from(w), u64::from(h));
    let (crop_w, crop_h) = if iw * h > ih * w {
        // Wider than the screen: use the full height, cut the sides
        ((ih * w / h).max(1), ih)
    } else {
        // Taller than the screen: use the full width, cut top and bottom
        (iw, (iw * h / w).max(1))
    };
    // All values are at most the image's own u32 size
    (((iw - crop_w) / 2) as u32, ((ih - crop_h) / 2) as u32, crop_w as u32, crop_h as u32)
}

/// Whether a picture is taller than it is wide (after EXIF orientation).
pub fn is_portrait(image: &DynamicImage) -> bool {
    image.height() > image.width()
}

/// Where and how [`show`] put a picture on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shown {
    pub rotation: Rotate,
    /// Top-left corner of the scaled picture (centered on the screen)
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
    pub scale_time: Duration,
    pub draw_time: Duration,
}

/// Shows `image` centered on the screen: portrait rotation for portrait
/// pictures, landscape otherwise. With [`Scale::Fit`] the rest of the screen
/// is cleared to black.
pub fn show<S: TftSpi>(
    display: &mut TftDisplay<S>,
    image: &DynamicImage,
    scale_mode: Scale,
) -> crate::tft_display::tft_display::Result<Shown> {
    let rotation = if is_portrait(image) { Rotate::Degrees0 } else { Rotate::Degrees90 };
    display.set_rotation(rotation)?;

    let start = Instant::now();
    let picture = scale(image, display.width(), display.height(), scale_mode);
    let scale_time = start.elapsed();

    let start = Instant::now();
    if (picture.width, picture.height) != (display.width(), display.height()) {
        display.fill_screen(Color::BLACK)?;
    }
    let x = (i32::from(display.width()) - i32::from(picture.width)) / 2;
    let y = (i32::from(display.height()) - i32::from(picture.height)) / 2;
    display.draw_image(x, y, picture.width, picture.height, &picture.rgb)?;

    Ok(Shown {
        rotation,
        x,
        y,
        width: picture.width,
        height: picture.height,
        scale_time,
        draw_time: start.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn test_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| Rgb([x as u8, y as u8, 128])))
    }

    #[test]
    fn fit_keeps_aspect_ratio() {
        // 4:3 photo on a 480x320 screen: limited by height
        let picture = scale(&test_image(4000, 3000), 480, 320, Scale::Fit);
        assert_eq!((picture.width, picture.height), (427, 320));
        assert_eq!(picture.rgb.len(), 427 * 320 * 3);

        // Small images are scaled up
        let picture = scale(&test_image(48, 32), 480, 320, Scale::Fit);
        assert_eq!((picture.width, picture.height), (480, 320));

        // Portrait image on a landscape screen
        let picture = scale(&test_image(300, 600), 480, 320, Scale::Fit);
        assert_eq!((picture.width, picture.height), (160, 320));
    }

    #[test]
    fn fill_covers_screen() {
        for (w, h) in [(4000, 3000), (300, 600), (48, 32), (1000, 1000)] {
            let picture = scale(&test_image(w, h), 480, 320, Scale::Fill);
            assert_eq!((picture.width, picture.height), (480, 320), "{w}x{h}");
        }
    }

    #[test]
    fn opens_jpeg_and_png() {
        let dir = std::env::temp_dir().join(format!("picture-open-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = test_image(64, 40);
        for name in ["test.jpg", "test.png", "jpeg-without-extension"] {
            let path = dir.join(name);
            let format = if name.contains("png") { image::ImageFormat::Png } else { image::ImageFormat::Jpeg };
            source.save_with_format(&path, format).unwrap();
            let image = open(&path).unwrap();
            assert_eq!((image.width(), image.height()), (64, 40), "{name}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A JPEG of `image` with an EXIF block holding just an Orientation tag.
    fn jpeg_with_orientation(image: &DynamicImage, orientation: u16) -> Vec<u8> {
        let mut jpeg = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
            .unwrap();
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "JPEG starts with SOI");

        // TIFF structure (big-endian): header, one IFD with one entry, no next IFD
        let mut tiff = b"MM\x00\x2A\x00\x00\x00\x08".to_vec();
        tiff.extend_from_slice(&1u16.to_be_bytes()); // entry count
        tiff.extend_from_slice(&0x0112u16.to_be_bytes()); // tag: Orientation
        tiff.extend_from_slice(&3u16.to_be_bytes()); // type: SHORT
        tiff.extend_from_slice(&1u32.to_be_bytes()); // count
        tiff.extend_from_slice(&orientation.to_be_bytes());
        tiff.extend_from_slice(&[0, 0]); // value padding
        tiff.extend_from_slice(&0u32.to_be_bytes()); // no next IFD

        // APP1 segment right after SOI; its length counts itself but not the marker
        let mut app1 = vec![0xFF, 0xE1];
        app1.extend_from_slice(&((2 + 6 + tiff.len()) as u16).to_be_bytes());
        app1.extend_from_slice(b"Exif\x00\x00");
        app1.extend_from_slice(&tiff);

        let mut out = jpeg[..2].to_vec();
        out.extend_from_slice(&app1);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    /// Phone cameras store the sensor's pixels and an EXIF Orientation saying
    /// how to turn them; `open` should return the picture the right way up.
    #[test]
    fn applies_exif_orientation() {
        // 64x32, red in the top-left 32x16 quarter, dark elsewhere
        let stored = DynamicImage::ImageRgb8(RgbImage::from_fn(64, 32, |x, y| {
            if x < 32 && y < 16 {
                Rgb([230, 20, 20])
            } else {
                Rgb([20, 20, 20])
            }
        }));
        let dir = std::env::temp_dir().join(format!("picture-exif-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // (orientation, size after turning, where the red quarter ends up)
        let cases = [
            (1, (64, 32), "top-left"),     // as stored
            (3, (64, 32), "bottom-right"), // turn 180
            (6, (32, 64), "top-right"),    // turn 90 clockwise
            (8, (32, 64), "bottom-left"),  // turn 90 counter-clockwise
        ];
        for (orientation, (w, h), red_corner) in cases {
            let path = dir.join(format!("orientation-{orientation}.jpg"));
            std::fs::write(&path, jpeg_with_orientation(&stored, orientation)).unwrap();
            let image = open(&path).unwrap().into_rgb8();
            assert_eq!(image.dimensions(), (w, h), "orientation {orientation}");

            // Sample the middle of each quarter (JPEG is lossy, so just check which is red)
            let red_at = |x: u32, y: u32| image.get_pixel(x, y)[0] > 150;
            let quarters = [
                ("top-left", w / 4, h / 4),
                ("top-right", 3 * w / 4, h / 4),
                ("bottom-left", w / 4, 3 * h / 4),
                ("bottom-right", 3 * w / 4, 3 * h / 4),
            ];
            for (name, x, y) in quarters {
                assert_eq!(red_at(x, y), name == red_corner, "orientation {orientation}: {name} quarter");
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn open_reports_missing_and_invalid_files() {
        let dir = std::env::temp_dir().join(format!("picture-error-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(open(&dir.join("does-not-exist.jpg")).is_err());

        let not_an_image = dir.join("not-an-image.jpg");
        std::fs::write(&not_an_image, b"this is not a JPEG").unwrap();
        assert!(open(&not_an_image).is_err());

        let truncated = dir.join("truncated.jpg");
        let jpeg = jpeg_with_orientation(&test_image(64, 40), 1);
        std::fs::write(&truncated, &jpeg[..jpeg.len() / 2]).unwrap();
        assert!(open(&truncated).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn center_crop_matches_screen_shape() {
        // Wider than 3:2: sides cut
        assert_eq!(center_crop(960, 320, 480, 320), (240, 0, 480, 320));
        assert_eq!(center_crop(4000, 2000, 480, 320), (500, 0, 3000, 2000));
        // Taller than 3:2: top and bottom cut
        assert_eq!(center_crop(4000, 3000, 480, 320), (0, 167, 4000, 2666));
        assert_eq!(center_crop(300, 600, 320, 480), (0, 75, 300, 450));
        // Same shape: nothing cut
        assert_eq!(center_crop(960, 640, 480, 320), (0, 0, 960, 640));
        // Thin strips keep at least one pixel
        assert_eq!(center_crop(5000, 1, 480, 320), (2499, 0, 1, 1));
        assert_eq!(center_crop(1, 5000, 480, 320), (0, 2499, 1, 1));
    }

    /// Transparent parts of a picture are shown over black, not as whatever
    /// color happens to be stored under the transparency.
    #[test]
    fn transparency_shows_black() {
        use image::{Rgba, RgbaImage};
        // 3 pixels: transparent red, half-transparent white, opaque blue
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(3, 1, |x, _| match x {
            0 => Rgba([255, 0, 0, 0]),
            1 => Rgba([255, 255, 255, 128]),
            _ => Rgba([0, 0, 255, 255]),
        }));
        let picture = scale(&image, 3, 1, Scale::Fit);
        assert_eq!(&picture.rgb[0..3], &[0, 0, 0], "fully transparent");
        assert!(picture.rgb[3..6].iter().all(|&c| (127..=129).contains(&c)), "half transparent: {:?}", &picture.rgb[3..6]);
        assert_eq!(&picture.rgb[6..9], &[0, 0, 255], "opaque");
    }

    /// Very thin pictures still scale to at least 1x1 and stay on screen.
    #[test]
    fn extreme_aspect_ratios() {
        for (w, h) in [(4000, 3), (3, 4000), (1, 1), (5000, 1)] {
            for mode in [Scale::Fit, Scale::Fill] {
                let picture = scale(&test_image(w, h), 480, 320, mode);
                let size = (picture.width, picture.height);
                assert!((1..=480).contains(&size.0) && (1..=320).contains(&size.1), "{w}x{h} {mode:?}: {size:?}");
                assert_eq!(picture.rgb.len(), usize::from(size.0) * usize::from(size.1) * 3);
            }
        }
    }

    #[test]
    fn portrait_detection() {
        assert!(is_portrait(&test_image(30, 40)));
        assert!(!is_portrait(&test_image(40, 30)));
        assert!(!is_portrait(&test_image(40, 40)));
    }
}

/// `show` against the mock panel.
#[cfg(test)]
mod show_tests {
    use super::*;
    use crate::tft_display::mock::MockSpi;
    use image::{Rgb, RgbImage};

    const COLOR: Rgb<u8> = Rgb([200, 100, 52]);

    /// A solid-color image: it stays exactly this color when scaled, so every
    /// on-screen pixel can be checked.
    fn solid(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_pixel(w, h, COLOR))
    }

    /// Shows `image` on a mock panel of the logical size `show` should pick,
    /// and checks the picture covers exactly the expected area with black around it.
    fn check_show(image: DynamicImage, scale: Scale, expected: (Rotate, i32, i32, u16, u16)) {
        let (rotation, x, y, w, h) = expected;
        let (screen_w, screen_h) = match rotation {
            Rotate::Degrees0 | Rotate::Degrees180 => (320, 480),
            Rotate::Degrees90 | Rotate::Degrees270 => (480, 320),
        };
        let spi = MockSpi::new(screen_w, screen_h);
        let mut display = TftDisplay::new(spi.clone());
        // Start from a screen full of something else, like a real panel
        display.set_rotation(rotation).unwrap();
        display.fill_screen(Color::WHITE).unwrap();

        let shown = show(&mut display, &image, scale).unwrap();
        assert_eq!(
            (shown.rotation, shown.x, shown.y, shown.width, shown.height),
            expected,
            "{}x{} {scale:?}",
            image.width(),
            image.height()
        );

        let color = Color::rgb(COLOR[0], COLOR[1], COLOR[2]).to_rgb666_bytes();
        let p = spi.0.borrow();
        for py in 0..screen_h {
            for px in 0..screen_w {
                let inside = (x..x + i32::from(w)).contains(&(px as i32)) && (y..y + i32::from(h)).contains(&(py as i32));
                let want = if inside { color } else { [0, 0, 0] };
                assert_eq!(p.pixels[py * screen_w + px], want, "pixel ({px}, {py})");
            }
        }
    }

    #[test]
    fn landscape_picture_fits_with_side_borders() {
        // 4:3 on 480x320: limited by height, centered horizontally
        check_show(solid(4000, 3000), Scale::Fit, (Rotate::Degrees90, 26, 0, 427, 320));
    }

    #[test]
    fn wide_picture_fits_with_top_and_bottom_borders() {
        check_show(solid(1600, 600), Scale::Fit, (Rotate::Degrees90, 0, 70, 480, 180));
    }

    #[test]
    fn portrait_picture_uses_portrait_rotation() {
        check_show(solid(300, 600), Scale::Fit, (Rotate::Degrees0, 40, 0, 240, 480));
    }

    #[test]
    fn square_picture_is_shown_in_landscape() {
        check_show(solid(1000, 1000), Scale::Fit, (Rotate::Degrees90, 80, 0, 320, 320));
    }

    #[test]
    fn small_picture_is_scaled_up() {
        check_show(solid(48, 32), Scale::Fit, (Rotate::Degrees90, 0, 0, 480, 320));
    }

    #[test]
    fn fill_covers_the_screen() {
        check_show(solid(4000, 3000), Scale::Fill, (Rotate::Degrees90, 0, 0, 480, 320));
        check_show(solid(300, 600), Scale::Fill, (Rotate::Degrees0, 0, 0, 320, 480));
    }

    /// Fill crops to the middle: the left and right edges of a wide picture are cut off.
    #[test]
    fn fill_crops_to_the_center() {
        // 960x320: left third red, middle third green, right third blue
        let image = DynamicImage::ImageRgb8(RgbImage::from_fn(960, 320, |x, _| match x / 320 {
            0 => Rgb([255, 0, 0]),
            1 => Rgb([0, 255, 0]),
            _ => Rgb([0, 0, 255]),
        }));
        let spi = MockSpi::new(480, 320);
        let mut display = TftDisplay::new(spi.clone());
        show(&mut display, &image, Scale::Fill).unwrap();

        // Nothing to scale (already 320 high), so the screen shows picture
        // columns 240..720: the last quarter of red, all of green, and the
        // first quarter of blue; the outer 240 columns on each side are cut.
        let p = spi.0.borrow();
        let at = |x: usize| p.pixels[160 * 480 + x];
        assert_eq!(at(40), Color::RED.to_rgb666_bytes());
        assert_eq!(at(240), Color::GREEN.to_rgb666_bytes());
        assert_eq!(at(440), Color::BLUE.to_rgb666_bytes());
    }
}
