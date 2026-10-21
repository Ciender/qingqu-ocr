//! Standard Office formats, published together after all representations exist.
use super::*;
use windows_sys::Win32::System::{DataExchange::*, Memory::*};
struct Data(HGLOBAL);
impl Data {
    unsafe fn new(bytes: &[u8]) -> Result<Self, String> {
        let allocation = Self(GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len() + 2));
        if allocation.0.is_null() {
            return Err("无法分配表格剪贴板内存。".into());
        }
        let ptr = GlobalLock(allocation.0);
        if ptr.is_null() {
            return Err("无法锁定表格剪贴板内存。".into());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast(), bytes.len());
        GlobalUnlock(allocation.0);
        Ok(allocation)
    }
    unsafe fn publish(&mut self, format: u32) -> Result<(), String> {
        if SetClipboardData(format, self.0).is_null() {
            return Err("无法写入表格剪贴板格式。".into());
        }
        self.0 = null_mut();
        Ok(())
    }
}
impl Drop for Data {
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
pub(super) fn copy(xml: &[u8], tsv: &str, rtf: &[u8]) -> Result<(), String> {
    unsafe {
        let unicode = wide(tsv);
        let unicode = std::slice::from_raw_parts(unicode.as_ptr().cast::<u8>(), unicode.len() * 2);
        let mut plain = Data::new(unicode)?;
        let mut rich = Data::new(rtf)?;
        let mut spreadsheet = Data::new(xml)?;
        let xml_format = RegisterClipboardFormatW(wide("XML Spreadsheet").as_ptr());
        let rtf_format = RegisterClipboardFormatW(wide("Rich Text Format").as_ptr());
        if xml_format == 0 || rtf_format == 0 {
            return Err("无法注册 Office 表格剪贴板格式。".into());
        }
        let mut opened = false;
        for _ in 0..20 {
            if OpenClipboard(null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if !opened {
            return Err("剪贴板正被占用，请重试。".into());
        }
        let _open = Open;
        if EmptyClipboard() == 0 {
            return Err("无法清空剪贴板。".into());
        }
        spreadsheet.publish(xml_format)?;
        rich.publish(rtf_format)?;
        plain.publish(13)?;
        Ok(())
    }
}
