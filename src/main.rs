#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cloud;
mod config;
mod image_store;
mod native;
mod ocr;
mod ocr_cloud;
mod platform;
mod qwen;
mod service_info;
mod session_file;
mod text;
mod translate;

fn main() {
    platform::enable_dpi_awareness();
    let preview = std::env::args().any(|s| s == "--preview-document");
    let _instance = if preview {
        None
    } else {
        match platform::Instance::acquire() {
            Some(instance) => Some(instance),
            None => return,
        }
    };
    if let Err(error) = native::run() {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                platform::wide(&error).as_ptr(),
                platform::wide("轻取 OCR").as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
        std::process::exit(1);
    }
}
