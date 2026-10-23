use super::win::*;
use std::ptr::null_mut;
include!(concat!(env!("OUT_DIR"), "/icon_atlases.rs"));
#[derive(Clone, Copy)]
#[repr(usize)]
#[allow(dead_code)]
pub enum Icon {
    Text,
    Table,
    Vertical,
    Formula,
    Clipboard,
    Image,
    Layout,
    Split,
    Merge,
    Quote,
    Chinese,
    Translate,
    Copy,
    Clear,
    Undo,
    Search,
    Pin,
    Menu,
    Min,
    Max,
    Close,
    Api,
    Keyboard,
    Appearance,
    Settings,
    Eye,
    ZoomIn,
    ZoomOut,
    Fit,
    RotateLeft,
    RotateRight,
    Window,
    ActualSize,
    Grip,
    Unpin,
    ToTraditional,
    Paste,
    Cut,
    Redo,
    Lines,
    Chevron,
    AutoParagraph,
    ChinesePunctuation,
    EnglishPunctuation,
    Help,
    ToSimplified,
}
pub struct Atlas {
    bitmap: Bitmap,
    cell: i32,
}
impl Atlas {
    pub fn new(cell: i32, color: COLORREF) -> Result<Self, String> {
        let cell = cell.clamp(12, 96);
        let image = image::load_from_memory(icon_atlas_png(cell as u32))
            .map_err(|e| e.to_string())?
            .into_rgba8();
        let bitmap = Bitmap::new(image.width() as i32, image.height() as i32)?;
        unsafe {
            let dest = std::slice::from_raw_parts_mut(bitmap.pixels, image.as_raw().len());
            for (out, pixel) in dest.chunks_exact_mut(4).zip(image.as_raw().chunks_exact(4)) {
                let a = pixel[3] as u32;
                out.copy_from_slice(&[
                    ((color >> 16 & 255) * a / 255) as u8,
                    ((color >> 8 & 255) * a / 255) as u8,
                    ((color & 255) * a / 255) as u8,
                    a as u8,
                ]);
            }
        }
        Ok(Self { bitmap, cell })
    }
    pub unsafe fn paint(&self, dc: HDC, r: RECT, icon: Icon) {
        AlphaBlend(
            dc,
            r.left + (r.right - r.left - self.cell) / 2,
            r.top + (r.bottom - r.top - self.cell) / 2,
            self.cell,
            self.cell,
            self.bitmap.dc,
            icon as i32 * self.cell,
            0,
            self.cell,
            self.cell,
            BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            },
        );
    }
}
pub fn application_icon() -> HICON {
    unsafe {
        LoadIconW(
            GetModuleHandleW(std::ptr::null()),
            std::ptr::without_provenance(1),
        )
    }
}
pub unsafe fn icon_to_window(hwnd: HWND) {
    let icon = application_icon();
    SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
    SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
    // This is a shared resource icon, owned by the module.
    let _ = null_mut::<u8>();
}
