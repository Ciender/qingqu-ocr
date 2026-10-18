//! Bounded OCR upload preparation. Encode once; retry only when the bounded writer overflows.
use image::{ImageEncoder, RgbaImage};
use std::io::{self, Write};

#[derive(Clone, Copy)]
pub(crate) struct Policy {
    pub max_edge: u32,
    pub min_edge: u32,
    max_pixels: u64,
    max_png_bytes: usize,
}
impl Policy {
    // Conservative client budgets, not a claim that every provider has identical limits.
    // 2.8 MB PNG -> <3.74 MB Base64, leaving room for JSON/data URL wrappers.
    pub fn standard() -> Self {
        Self {
            max_edge: 4096,
            min_edge: 1,
            max_pixels: 10_000_000,
            max_png_bytes: 2_800_000,
        }
    }
    pub fn baidu() -> Self {
        // Worst-case URL escaping triples Base64: 2 MB -> <8 MB form image field.
        Self {
            min_edge: 15,
            max_png_bytes: 2_000_000,
            ..Self::standard()
        }
    }
}

struct Limited {
    bytes: Vec<u8>,
    limit: usize,
    overflow: bool,
}
impl Write for Limited {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() > self.limit.saturating_sub(self.bytes.len()) {
            self.overflow = true;
            return Err(io::Error::other("OCR image byte budget exceeded"));
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn png(mut image: RgbaImage, policy: Policy) -> Result<Vec<u8>, String> {
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return Err("图片尺寸为空。".into());
    }
    // Apply dimensions before allocating an encoder or compressed output.
    let scale = (policy.max_edge as f64 / w.max(h) as f64)
        .min((policy.max_pixels as f64 / (u64::from(w) * u64::from(h)) as f64).sqrt())
        .min(1.0);
    let (mut nw, mut nh) = (
        ((w as f64 * scale).floor() as u32).max(1),
        ((h as f64 * scale).floor() as u32).max(1),
    );
    if nw.min(nh) < policy.min_edge {
        let up = policy.min_edge as f64 / nw.min(nh) as f64;
        nw = (nw as f64 * up).ceil() as u32;
        nh = (nh as f64 * up).ceil() as u32;
        if nw.max(nh) > policy.max_edge || u64::from(nw) * u64::from(nh) > policy.max_pixels {
            return Err("图片过于狭长，无法同时满足 OCR 长短边限制，请分段框选。".into());
        }
    }
    if (nw, nh) != (w, h) {
        image = resample(&image, nw, nh);
    }
    // Composite transparency on white; OCR inputs do not need a separate RGB copy.
    for p in image.pixels_mut() {
        let a = u32::from(p[3]);
        for i in 0..3 {
            p[i] = ((u32::from(p[i]) * a + 255 * (255 - a) + 127) / 255) as u8;
        }
        p[3] = 255;
    }
    for _ in 0..16 {
        let (w, h) = image.dimensions();
        let mut out = Limited {
            bytes: Vec::with_capacity(policy.max_png_bytes),
            limit: policy.max_png_bytes,
            overflow: false,
        };
        // Lossless compression avoids the much larger Fast PNGs on text captures.
        // Keep RGBA storage to avoid allocating another full-size RGB image.
        let encoded = image::codecs::png::PngEncoder::new_with_quality(
            &mut out,
            image::codecs::png::CompressionType::Level(3),
            image::codecs::png::FilterType::Adaptive,
        )
        .write_image(image.as_raw(), w, h, image::ExtendedColorType::Rgba8);
        if encoded.is_ok() {
            return Ok(out.bytes);
        }
        if !out.overflow {
            return Err("OCR 图片编码失败。".into());
        }
        drop(out);
        let (nw, nh) = ((w * 4 / 5).max(1), (h * 4 / 5).max(1));
        if nw.min(nh) < policy.min_edge || (nw, nh) == (w, h) {
            break;
        }
        image = resample(&image, nw, nh);
    }
    Err("图片仍超过 OCR 上传预算，请缩小选区或分段识别。".into())
}

// Area average for reduction, nearest for minimum-edge enlargement. Only destination storage;
// unlike a separable float filter, no full-width intermediate float image is allocated.
fn resample(src: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    if w >= src.width() && h >= src.height() {
        return RgbaImage::from_fn(w, h, |x, y| {
            *src.get_pixel(
                (u64::from(x) * u64::from(src.width()) / u64::from(w)) as u32,
                (u64::from(y) * u64::from(src.height()) / u64::from(h)) as u32,
            )
        });
    }
    RgbaImage::from_fn(w, h, |x, y| {
        let sx = src.width() as f64 / w as f64;
        let sy = src.height() as f64 / h as f64;
        let (x0, x1, y0, y1) = (
            x as f64 * sx,
            (x + 1) as f64 * sx,
            y as f64 * sy,
            (y + 1) as f64 * sy,
        );
        let mut sum = [0.0; 3];
        for iy in y0.floor() as u32..(y1.ceil() as u32).min(src.height()) {
            for ix in x0.floor() as u32..(x1.ceil() as u32).min(src.width()) {
                let weight = (x1.min((ix + 1) as f64) - x0.max(ix as f64))
                    * (y1.min((iy + 1) as f64) - y0.max(iy as f64));
                let p = src.get_pixel(ix, iy);
                let a = p[3] as f64 / 255.;
                for c in 0..3 {
                    sum[c] += (p[c] as f64 * a + 255. * (1. - a)) * weight;
                }
            }
        }
        image::Rgba([
            (sum[0] / (sx * sy)).round() as u8,
            (sum[1] / (sx * sy)).round() as u8,
            (sum[2] / (sx * sy)).round() as u8,
            255,
        ])
    })
}
