//! Private, process-lifetime storage. Windows deletes the file when its last handle closes.
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::OpenOptionsExt,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn create(kind: &str) -> io::Result<File> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..100 {
        let path = std::env::temp_dir().join(format!(
            "qingqu-{}-{}.{kind}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(7)
            .custom_flags(0x1400_0000)
            .open(path)
        {
            // DELETE_ON_CLOSE | RANDOM_ACCESS
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            result => return result,
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "无法创建会话临时文件",
    ))
}
