//! Bounded tiles around fast_image_resize's runtime-dispatched SIMD convolution.
//! One continuous filter and pixel-center transform are used at every scale.

use fast_image_resize::{
    images::{CroppedImage, Image, ImageRef},
    FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer,
};

/// GDI text/frame operations do not define alpha. Mark only the visible
/// rectangle opaque while leaving the surrounding animation canvas untouched.
pub fn opaque_rectangle(
    pixels: &mut [u8],
    stride: usize,
    origin: (usize, usize),
    size: (usize, usize),
) {
    for y in origin.1..origin.1 + size.1 {
        let start = y * stride + origin.0 * 4;
        for pixel in pixels[start..start + size.0 * 4].chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }
}

#[derive(Default)]
pub struct Renderer {
    resizer: Resizer,
    input: Vec<u8>,
    output: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct Transform {
    pub viewport: (i32, i32),
    pub zoom: f32,
    pub scroll: (f32, f32),
    pub background: [u8; 4],
}

impl Renderer {
    /// Diagnostic scalar path; never enables unsupported CPU instructions.
    pub fn without_simd() -> Self {
        let mut renderer = Self::default();
        unsafe {
            renderer
                .resizer
                .set_cpu_extensions(fast_image_resize::CpuExtensions::None);
        }
        renderer
    }
    pub fn workspace_bytes(&self) -> usize {
        self.resizer.size_of_internal_buffers() + self.input.capacity() + self.output.capacity()
    }

    pub fn render(
        &mut self,
        image: &ImageView,
        source: &[u8],
        transform: Transform,
        dest: &mut [u8],
    ) {
        self.render_strided(
            image,
            source,
            transform,
            dest,
            transform.viewport.0 as usize * 4,
        );
    }

    /// Draw into rows of a larger surface without allocating another image.
    pub fn render_strided(
        &mut self,
        image: &ImageView,
        source: &[u8],
        transform: Transform,
        dest: &mut [u8],
        stride: usize,
    ) {
        let Transform {
            viewport,
            zoom,
            scroll,
            background,
        } = transform;
        let (width, height) = viewport;
        let zoom = f64::from(zoom);
        let origin = |size: i32, pixels: u32, scroll: f32| {
            ((size - (f64::from(pixels) * zoom).round() as i32) / 2).max(0) as f64
                - f64::from(scroll)
        };
        let (ox, oy) = (
            origin(width, image.width(), scroll.0),
            origin(height, image.height(), scroll.1),
        );
        for row in dest.chunks_mut(stride).take(height as usize) {
            for pixel in row[..width as usize * 4].chunks_exact_mut(4) {
                pixel.copy_from_slice(&background);
            }
        }
        // Pixel centers outside the image remain background. Source padding
        // preserves the transform at fractional outer edges instead of snapping
        // or stretching a clamped crop rectangle to an integer destination.
        let visible = |origin: f64, pixels: u32, size: i32| {
            let start = (origin - 0.5).ceil().clamp(0., f64::from(size)) as i32;
            let end = (origin + f64::from(pixels) * zoom - 0.5)
                .ceil()
                .clamp(f64::from(start), f64::from(size)) as i32;
            start..end
        };
        let xs = visible(ox, image.width(), width);
        let ys = visible(oy, image.height(), height);
        // Cap both source footprint and output workspace, including very small
        // zooms. There is no second full original or full resampled image.
        let columns = ((512. * zoom) as i32).clamp(1, 1024);
        let rows = ((512. * zoom) as i32).clamp(1, 64);
        let radius = 1. / zoom.min(1.);
        let opaque = image.opaque;
        let options = ResizeOptions::new()
            .resize_alg(ResizeAlg::Convolution(FilterType::Bilinear))
            .use_alpha(!opaque);
        let original = ImageRef::new(
            image.source_width,
            image.source_height,
            source,
            PixelType::U8x4,
        )
        .unwrap();
        for y in (ys.start..ys.end).step_by(rows as usize) {
            let th = rows.min(ys.end - y);
            let top = (f64::from(y) - oy) / zoom;
            for x in (xs.start..xs.end).step_by(columns as usize) {
                let tw = columns.min(xs.end - x);
                let left = (f64::from(x) - ox) / zoom;
                let right = left + f64::from(tw) / zoom;
                let bottom = top + f64::from(th) / zoom;
                let sx = (left - radius).floor() as i32;
                let sy = (top - radius).floor() as i32;
                let sw = (right + radius).ceil() as i32 - sx;
                let sh = (bottom + radius).ceil() as i32 - sy;
                self.output.resize((tw * th * 4) as usize, 0);
                let mut output =
                    Image::from_slice_u8(tw as u32, th as u32, &mut self.output, PixelType::U8x4)
                        .unwrap();
                let options = options.crop(
                    left - f64::from(sx),
                    top - f64::from(sy),
                    f64::from(tw) / zoom,
                    f64::from(th) / zoom,
                );
                if image.turns.is_multiple_of(4)
                    && sx >= 0
                    && sy >= 0
                    && sx + sw <= image.width() as i32
                    && sy + sh <= image.height() as i32
                {
                    // Borrow original rows directly for ordinary unrotated
                    // interior tiles. Alpha workspace is bounded to this crop.
                    let tile =
                        CroppedImage::new(&original, sx as u32, sy as u32, sw as u32, sh as u32)
                            .unwrap();
                    self.resizer.resize(&tile, &mut output, &options).unwrap();
                } else {
                    self.input.resize((sw * sh * 4) as usize, 0);
                    for ty in 0..sh {
                        let py = (sy + ty).clamp(0, image.height() as i32 - 1) as u32;
                        for tx in 0..sw {
                            let px = (sx + tx).clamp(0, image.width() as i32 - 1) as u32;
                            let pixel = image.pixel(source, px, py);
                            let offset = ((ty * sw + tx) * 4) as usize;
                            self.input[offset..offset + 4].copy_from_slice(&pixel);
                        }
                    }
                    let tile =
                        ImageRef::new(sw as u32, sh as u32, &self.input, PixelType::U8x4).unwrap();
                    self.resizer.resize(&tile, &mut output, &options).unwrap();
                }
                for ty in 0..th {
                    let start = (y + ty) as usize * stride + x as usize * 4;
                    let target = &mut dest[start..start + (tw * 4) as usize];
                    let source = &self.output[(ty * tw * 4) as usize..][..(tw * 4) as usize];
                    if opaque {
                        for (out, p) in target.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
                            out.copy_from_slice(&[p[2], p[1], p[0], 255]);
                        }
                        continue;
                    }
                    for (tx, (out, p)) in target
                        .chunks_exact_mut(4)
                        .zip(source.chunks_exact(4))
                        .enumerate()
                    {
                        let a = u32::from(p[3]);
                        let checker = if (((x + tx as i32) / 8) + ((y + ty) / 8)) % 2 == 0 {
                            220
                        } else {
                            245
                        };
                        for c in 0..3 {
                            out[2 - c] =
                                ((u32::from(p[c]) * a + checker * (255 - a) + 127) / 255) as u8;
                        }
                        out[3] = 255;
                    }
                }
            }
        }
    }
}

/// Metadata for immutable RGBA pixels supplied by the caller.
pub struct ImageView {
    pub source_width: u32,
    pub source_height: u32,
    pub turns: u8,
    pub opaque: bool,
}
impl ImageView {
    fn width(&self) -> u32 {
        if self.turns.is_multiple_of(2) {
            self.source_width
        } else {
            self.source_height
        }
    }
    fn height(&self) -> u32 {
        if self.turns.is_multiple_of(2) {
            self.source_height
        } else {
            self.source_width
        }
    }
    fn pixel(&self, source: &[u8], x: u32, y: u32) -> [u8; 4] {
        let (w, h) = (self.source_width, self.source_height);
        let (x, y) = match self.turns % 4 {
            1 => (y, h - 1 - x),
            2 => (w - 1 - x, h - 1 - y),
            3 => (w - 1 - y, x),
            _ => (x, y),
        };
        source[((y * w + x) * 4) as usize..][..4]
            .try_into()
            .unwrap()
    }
}
