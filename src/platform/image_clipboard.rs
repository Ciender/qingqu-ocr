//! One copy, several representations of the same original image.
use super::*;
use image::{codecs::png::PngEncoder, ImageEncoder};
use std::{mem::size_of, time::Duration};
use windows_sys::Win32::System::{DataExchange::*, Memory::*};

struct GlobalData(HGLOBAL);
impl GlobalData {
    unsafe fn new(size: usize, fill: impl FnOnce(&mut [u8])) -> Result<Self, String> {
        let data = Self(GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size));
        if data.0.is_null() {
            return Err("无法分配图片剪贴板内存。".into());
        }
        let ptr = GlobalLock(data.0);
        if ptr.is_null() {
            return Err("无法写入图片剪贴板内存。".into());
        }
        fill(std::slice::from_raw_parts_mut(ptr.cast(), size));
        GlobalUnlock(data.0);
        Ok(data)
    }
    unsafe fn bytes(bytes: &[u8]) -> Result<Self, String> {
        Self::new(bytes.len(), |out| out.copy_from_slice(bytes))
    }
    unsafe fn publish(&mut self, format: u32) -> Result<(), String> {
        if SetClipboardData(format, self.0).is_null() {
            return Err("复制图片失败，无法写入全部兼容格式。".into());
        }
        self.0 = null_mut(); // Windows owns successfully published memory.
        Ok(())
    }
}
impl Drop for GlobalData {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                GlobalFree(self.0);
            }
        }
    }
}
struct Open;
impl Drop for Open {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

// Read existing source metadata only: no encoding, copied buffer or retained lock.
pub fn png_size(sequence: u32) -> Option<usize> {
    unsafe {
        if sequence == 0 || OpenClipboard(null_mut()) == 0 {
            return None;
        }
        let _open = Open;
        if GetClipboardSequenceNumber() != sequence {
            return None;
        }
        let format = RegisterClipboardFormatW(wide("PNG").as_ptr());
        if format == 0 || IsClipboardFormatAvailable(format) == 0 {
            return None;
        }
        let handle = GetClipboardData(format);
        if handle.is_null() {
            return None;
        }
        let bytes = GlobalSize(handle);
        (bytes != 0).then_some(bytes)
    }
}

pub fn copy(owner: HWND, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > 64_000_000
        || rgba.len() != width as usize * height as usize * 4
    {
        return Err("图片尺寸或像素数据无效。".into());
    }
    let mut png = Vec::new();
    PngEncoder::new_with_quality(
        &mut png,
        image::codecs::png::CompressionType::Level(3),
        image::codecs::png::FilterType::Adaptive,
    )
    .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
    .map_err(|_| "图片剪贴板 PNG 编码失败。")?;
    let mut export = super::clipboard_cache::prepare(&png)?;
    let path = export
        .path
        .to_str()
        .ok_or("图片剪贴板路径不是有效 Unicode。")?;
    // Quoting allows spaces in the temp path without splitting an image argument.
    let text: Vec<u8> = wide(&format!("\"{path}\""))
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let files: Vec<u8> = wide(path)
        .into_iter()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect();
    unsafe {
        let format = RegisterClipboardFormatW(wide("PNG").as_ptr());
        if format == 0 {
            return Err("无法注册 PNG 剪贴板格式。".into());
        }
        let header = BITMAPV5HEADER {
            bV5Size: size_of::<BITMAPV5HEADER>() as u32,
            bV5Width: width as i32,
            bV5Height: height as i32,
            bV5Planes: 1,
            bV5BitCount: 32,
            bV5Compression: BI_BITFIELDS,
            bV5SizeImage: rgba.len() as u32,
            bV5RedMask: 0x00ff0000,
            bV5GreenMask: 0x0000ff00,
            bV5BlueMask: 0x000000ff,
            bV5AlphaMask: 0xff000000,
            bV5CSType: 0x73524742, // LCS_sRGB
            bV5Intent: LCS_GM_IMAGES as u32,
            ..Default::default()
        };
        let dib = GlobalData::new(size_of::<BITMAPV5HEADER>() + rgba.len(), |out| {
            std::ptr::write_unaligned(out.as_mut_ptr().cast::<BITMAPV5HEADER>(), header);
            let row_bytes = width as usize * 4;
            // Positive height and bottom-up BGRA also work in Word's DIB reader.
            for (source, dest) in rgba
                .chunks_exact(row_bytes)
                .rev()
                .zip(out[size_of::<BITMAPV5HEADER>()..].chunks_exact_mut(row_bytes))
            {
                for (p, q) in source.chunks_exact(4).zip(dest.chunks_exact_mut(4)) {
                    q.copy_from_slice(&[p[2], p[1], p[0], p[3]]);
                }
            }
        })?;
        let dropfiles = DROPFILES {
            pFiles: size_of::<DROPFILES>() as u32,
            fWide: 1,
            ..Default::default()
        };
        let file_list = GlobalData::new(size_of::<DROPFILES>() + files.len(), |out| {
            std::ptr::write_unaligned(out.as_mut_ptr().cast::<DROPFILES>(), dropfiles);
            out[size_of::<DROPFILES>()..].copy_from_slice(&files);
        })?;
        // Prepare everything before opening/emptying, then publish in one lock.
        let mut formats = [
            (format, GlobalData::bytes(&png)?),
            (17, dib),       // CF_DIBV5; Windows synthesizes CF_DIB and CF_BITMAP.
            (15, file_list), // CF_HDROP for apps accepting image files.
            (13, GlobalData::bytes(&text)?), // CF_UNICODETEXT for terminal right-click.
        ];
        let mut opened = false;
        for attempt in 0..6 {
            if OpenClipboard(owner) != 0 {
                opened = true;
                break;
            }
            if attempt < 5 {
                thread::sleep(Duration::from_millis(10));
            }
        }
        if !opened {
            return Err("复制图片失败，剪贴板正被占用，请稍后重试。".into());
        }
        let _open = Open;
        if EmptyClipboard() == 0 {
            return Err("复制图片失败，无法清空旧剪贴板内容。".into());
        }
        for (format, data) in &mut formats {
            data.publish(*format)?;
            if *format == 15 {
                // Even a partial publish can expose the PNG through CF_HDROP.
                export.referenced();
            }
        }
        drop(_open);
        // No filesystem scanning while holding the Windows clipboard lock.
        super::clipboard_cache::published(export);
        Ok(())
    }
}
