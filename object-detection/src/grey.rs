use anyhow::{Context, Result, bail};
use image::{DynamicImage, ImageFormat};

/// A greyscale frame: one byte per pixel, row-major, top row first.
#[derive(Debug, Clone)]
pub struct Grey {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Grey {
    /// Decodes a JPEG to grey values the way Ultralytics reads this
    /// model's frames: OpenCV's `IMREAD_GRAYSCALE`, which has libjpeg
    /// output the luma (Y) component as stored. `image` decodes a
    /// 3-component JPEG to RGB instead, so Y is recomputed with libjpeg's
    /// own fixed-point JFIF weights, Y = (19595 R + 38470 G + 7471 B +
    /// 32768) >> 16 (0.299, 0.587, 0.114 × 65536); where the decoder
    /// rounded or clamped RGB, this can differ from the stored Y by a
    /// level. A 1-component JPEG is its luma already. Anything that does
    /// not decode as a JPEG is an error (D4).
    pub fn decode(jpeg: &[u8]) -> Result<Grey> {
        let image = image::load_from_memory_with_format(jpeg, ImageFormat::Jpeg).context("decoding the frame JPEG")?;
        let (width, height) = (usize::try_from(image.width())?, usize::try_from(image.height())?);
        let pixels = match image {
            DynamicImage::ImageLuma8(luma) => luma.into_raw(),
            DynamicImage::ImageRgb8(rgb) => rgb
                .pixels()
                .map(|p| {
                    let [r, g, b] = p.0.map(u32::from);
                    // At most 65535·255 + 32768 < 2^24, then >> 16: ≤ 255.
                    ((19595 * r + 38470 * g + 7471 * b + 32768) >> 16) as u8
                })
                .collect(),
            other => bail!("the frame JPEG decodes to {:?}, expected grey or RGB", other.color()),
        };
        Ok(Grey { width, height, pixels })
    }
}
