//! Loading JPEG/PNG files and scaling them for the display.

use std::path::Path;

use image::{imageops::FilterType, DynamicImage, ImageDecoder, ImageReader, ImageResult};

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
        Scale::Fill => image.resize_to_fill(w, h, FilterType::Triangle),
    };
    let rgb = scaled.into_rgb8();
    Picture {
        // Both are at most width/height, so they fit in u16
        width: rgb.width() as u16,
        height: rgb.height() as u16,
        rgb: rgb.into_raw(),
    }
}

/// Whether a picture is taller than it is wide (after EXIF orientation).
pub fn is_portrait(image: &DynamicImage) -> bool {
    image.height() > image.width()
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
    fn portrait_detection() {
        assert!(is_portrait(&test_image(30, 40)));
        assert!(!is_portrait(&test_image(40, 30)));
        assert!(!is_portrait(&test_image(40, 40)));
    }
}
