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
        let dir = std::env::temp_dir().join(format!("picture-test-{}", std::process::id()));
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

    #[test]
    fn portrait_detection() {
        assert!(is_portrait(&test_image(30, 40)));
        assert!(!is_portrait(&test_image(40, 30)));
        assert!(!is_portrait(&test_image(40, 40)));
    }
}
