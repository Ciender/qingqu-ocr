//! Immutable originals shared by all pins. File-backed pixels are mapped only while in use.
use image::RgbaImage;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    os::windows::io::AsRawHandle,
    ptr::null,
    sync::{Arc, Mutex, OnceLock, Weak},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND},
    System::Memory::{
        CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ,
        MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READONLY,
    },
};

pub struct StoredImage {
    width: u32,
    height: u32,
    opaque: bool,
    mapping: HANDLE,
    _file: File,
}
// The mapping and file are immutable; each reader owns a separate view.
unsafe impl Send for StoredImage {}
unsafe impl Sync for StoredImage {}
impl Drop for StoredImage {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.mapping);
        }
    }
}
pub struct Pixels<'a> {
    bytes: &'a [u8],
}
/// An immutable view whose lifetime is owned by one active interaction.
/// Retaining the file-backed mapping avoids refaulting source pages per frame.
pub struct MappedPixels {
    original: Arc<StoredImage>,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
}
impl std::ops::Deref for MappedPixels {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        // The Arc keeps the immutable file/mapping alive until this view drops.
        unsafe {
            std::slice::from_raw_parts(
                self.view.Value.cast(),
                self.original.width as usize * self.original.height as usize * 4,
            )
        }
    }
}
impl Drop for MappedPixels {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.view);
        }
    }
}
impl std::ops::Deref for Pixels<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.bytes
    }
}
impl Drop for Pixels<'_> {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.bytes.as_ptr().cast_mut().cast(),
            });
        }
    }
}
type ImageIndex = HashMap<(u32, u32, [u8; 16]), Vec<Weak<StoredImage>>>;
impl StoredImage {
    pub fn intern(image: RgbaImage) -> Result<Arc<Self>, String> {
        static INDEX: OnceLock<Mutex<ImageIndex>> = OnceLock::new();
        let (width, height) = image.dimensions();
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 64_000_000 {
            return Err("图片尺寸无效或超过 6400 万像素。".into());
        }
        let key = (width, height, md5::compute(image.as_raw()).0);
        let mut index = INDEX
            .get_or_init(Mutex::default)
            .lock()
            .map_err(|_| "图片存储不可用。")?;
        index.retain(|_, images| {
            images.retain(|image| image.strong_count() != 0);
            !images.is_empty()
        });
        if let Some(images) = index.get(&key) {
            for existing in images.iter().filter_map(Weak::upgrade) {
                // Hashes only select candidates; compare actual pixels as well.
                if &*existing.map()? == image.as_raw().as_slice() {
                    return Ok(existing);
                }
            }
        }
        let mut file = crate::session_file::create("rgba").map_err(|_| "无法创建图片临时文件。")?;
        file.write_all(image.as_raw())
            .map_err(|_| "图片临时文件写入失败，请检查磁盘空间。")?;
        let mapping = unsafe {
            CreateFileMappingW(file.as_raw_handle(), null(), PAGE_READONLY, 0, 0, null())
        };
        if mapping.is_null() {
            return Err("无法建立图片文件映射。".into());
        }
        let opaque = image.as_raw().chunks_exact(4).all(|pixel| pixel[3] == 255);
        let stored = Arc::new(Self {
            width,
            height,
            opaque,
            mapping,
            _file: file,
        });
        index.entry(key).or_default().push(Arc::downgrade(&stored));
        Ok(stored)
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn opaque(&self) -> bool {
        self.opaque
    }
    pub fn map_owned(self: &Arc<Self>) -> Result<MappedPixels, String> {
        let view = unsafe { MapViewOfFile(self.mapping, FILE_MAP_READ, 0, 0, 0) };
        if view.Value.is_null() {
            return Err("无法读取图片临时文件。".into());
        }
        Ok(MappedPixels {
            original: self.clone(),
            view,
        })
    }
    pub fn map(&self) -> Result<Pixels<'_>, String> {
        let view = unsafe { MapViewOfFile(self.mapping, FILE_MAP_READ, 0, 0, 0) };
        if view.Value.is_null() {
            return Err("无法读取图片临时文件。".into());
        }
        Ok(Pixels {
            bytes: unsafe {
                std::slice::from_raw_parts(
                    view.Value.cast(),
                    self.width as usize * self.height as usize * 4,
                )
            },
        })
    }
}

#[derive(Clone)]
pub struct ImageView {
    pub original: Arc<StoredImage>,
    pub turns: u8,
    // Source metadata belongs to this clipboard snapshot, not deduplicated pixels.
    pub png_bytes: Option<usize>,
}
impl ImageView {
    pub fn new(original: Arc<StoredImage>) -> Self {
        Self {
            original,
            turns: 0,
            png_bytes: None,
        }
    }
    pub fn width(&self) -> u32 {
        if self.turns.is_multiple_of(2) {
            self.original.width()
        } else {
            self.original.height()
        }
    }
    pub fn height(&self) -> u32 {
        if self.turns.is_multiple_of(2) {
            self.original.height()
        } else {
            self.original.width()
        }
    }
    pub fn pixel(&self, pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
        let (w, h) = (self.original.width(), self.original.height());
        let (x, y) = match self.turns % 4 {
            1 => (y, h - 1 - x),
            2 => (w - 1 - x, h - 1 - y),
            3 => (w - 1 - y, x),
            _ => (x, y),
        };
        pixels[((y * w + x) * 4) as usize..][..4]
            .try_into()
            .unwrap()
    }
    pub fn decode(&self) -> Result<RgbaImage, String> {
        let pixels = self.original.map()?;
        Ok(RgbaImage::from_fn(self.width(), self.height(), |x, y| {
            image::Rgba(self.pixel(&pixels, x, y))
        }))
    }
    pub fn copy(&self, owner: HWND) -> Result<(), String> {
        if !self.turns.is_multiple_of(4) {
            return crate::platform::copy_image(owner, &self.decode()?);
        }
        let pixels = self.original.map()?;
        crate::platform::copy_image_pixels(owner, self.width(), self.height(), &pixels)
    }
}
