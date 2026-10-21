//! Bounded PNG exports for clipboard consumers. Never touches original/session files.
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime},
};

const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_FILES: usize = 128;
const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const TMP_AGE: Duration = Duration::from_secs(24 * 60 * 60);
// One file handle, no retained PNG buffer. Protect the copied path while this
// process lives, even when its owning pin has closed or another copy prunes.
static COPIED_FILE: Mutex<Option<File>> = Mutex::new(None);

pub struct Export {
    pub path: PathBuf,
    file: Option<File>,
    unpublished_new_file: bool,
}
impl Export {
    pub fn referenced(&mut self) {
        self.unpublished_new_file = false;
    }
}
impl Drop for Export {
    fn drop(&mut self) {
        if self.unpublished_new_file {
            // A failed copy must not accumulate exports that nobody received.
            drop(self.file.take());
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn same_contents(path: &Path, bytes: &[u8]) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    if file.metadata().map(|m| m.len()).ok() != Some(bytes.len() as u64) {
        return false;
    }
    // Re-copying a large PNG must not allocate a second full PNG buffer.
    let mut buffer = [0u8; 16 * 1024];
    for expected in bytes.chunks(buffer.len()) {
        let actual = &mut buffer[..expected.len()];
        if file.read_exact(actual).is_err() || actual != expected {
            return false;
        }
    }
    true
}

pub fn prepare(png: &[u8]) -> Result<Export, String> {
    let folder = std::env::temp_dir().join("QingquOCR-clipboard");
    fs::create_dir_all(&folder).map_err(|_| "无法创建图片剪贴板临时目录。")?;
    let digest = format!("{:x}", md5::compute(png));
    let path = folder.join(format!("image-{digest}.png"));
    let created = !same_contents(&path, png);
    if created {
        let pending = folder.join(format!("image-{digest}-{}.tmp", std::process::id()));
        let result = fs::write(&pending, png).and_then(|_| fs::rename(&pending, &path));
        if result.is_err() {
            let _ = fs::remove_file(&pending);
            return Err("无法保存图片剪贴板 PNG 文件，请检查临时目录。".into());
        }
    }
    // Share reads/writes so consumers can read it; do not share deletion.
    let file = OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&path)
        .map_err(|_| "无法保留图片剪贴板 PNG 文件。")?;
    Ok(Export {
        path,
        file: Some(file),
        unpublished_new_file: created,
    })
}

pub fn published(mut export: Export) {
    export.referenced();
    if let Ok(mut current) = COPIED_FILE.lock() {
        *current = export.file.take();
    }
    if let Some(folder) = export.path.parent() {
        prune(
            folder,
            &export.path,
            SystemTime::now(),
            MAX_FILES,
            MAX_BYTES,
        );
    }
}

fn owned_name(name: &str) -> Option<bool> {
    let name = name.strip_prefix("image-")?;
    let (digest, temporary) = if let Some(digest) = name.strip_suffix(".png") {
        (digest, false)
    } else {
        let (digest, pid) = name.strip_suffix(".tmp")?.split_once('-')?;
        if pid.is_empty() || !pid.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        (digest, true)
    };
    (digest.len() == 32 && digest.bytes().all(|b| b.is_ascii_hexdigit())).then_some(temporary)
}

fn prune(folder: &Path, current: &Path, now: SystemTime, max_files: usize, max_bytes: u64) {
    let Ok(metadata) = fs::symlink_metadata(folder) else {
        return;
    };
    // Never follow a cache directory or file that has been replaced by a link.
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path == current {
            continue;
        }
        let Some(temporary) = entry.file_name().to_str().and_then(owned_name) else {
            continue;
        };
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let age = now.duration_since(modified).unwrap_or_default();
        if age >= if temporary { TMP_AGE } else { MAX_AGE } {
            // Locked files and exports held by another running process survive.
            if fs::remove_file(&path).is_ok() {
                continue;
            }
        }
        if !temporary {
            files.push((modified, path, metadata.len()));
        }
    }
    let current_bytes = fs::metadata(current).map(|m| m.len()).unwrap_or(0);
    let mut bytes = current_bytes.saturating_add(files.iter().map(|f| f.2).sum::<u64>());
    let mut count = files.len() + usize::from(current_bytes != 0);
    files.sort_unstable_by_key(|a| a.0);
    for (_, path, size) in files {
        if count <= max_files && bytes <= max_bytes {
            break;
        }
        if fs::remove_file(path).is_ok() {
            count -= 1;
            bytes = bytes.saturating_sub(size);
        }
    }
}
