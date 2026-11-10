//! Current-user Windows integration; no elevation or shell subprocesses.
use super::wide;
use std::{
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};
use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Foundation::RPC_E_CHANGED_MODE,
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IPersistFile,
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
        },
        UI::Shell::{
            FOLDERID_Desktop, FOLDERID_Programs, IShellLinkW, SHGetKnownFolderPath, ShellLink,
            KF_FLAG_DEFAULT,
        },
    },
};
use windows_sys::Win32::{Foundation::*, System::Registry::*};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "QingquOCR";

pub fn startup_enabled() -> bool {
    startup_enabled_at(RUN_KEY)
}

fn startup_enabled_at(key: &str) -> bool {
    unsafe {
        let mut h = null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(key).as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut h,
        ) != 0
        {
            return false;
        }
        let result = RegQueryValueExW(
            h,
            wide(RUN_VALUE).as_ptr(),
            null(),
            null_mut(),
            null_mut(),
            null_mut(),
        ) == 0;
        RegCloseKey(h);
        result
    }
}

pub fn set_startup(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|_| "无法定位程序文件。")?;
    set_startup_at(RUN_KEY, enabled.then_some(exe.as_path()))
}

fn set_startup_at(key: &str, exe: Option<&Path>) -> Result<(), String> {
    // Build the command before opening the key so failures cannot leak its handle.
    let command = exe.map(|exe| {
        let mut data = vec![b'"' as u16];
        data.extend(exe.as_os_str().encode_wide());
        data.extend(wide("\" --tray"));
        data
    });
    unsafe {
        let mut h = null_mut();
        let status = if command.is_some() {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(key).as_ptr(),
                0,
                null(),
                0,
                KEY_SET_VALUE,
                null(),
                &mut h,
                null_mut(),
            )
        } else {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                wide(key).as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut h,
            )
        };
        if status == ERROR_FILE_NOT_FOUND && command.is_none() {
            return Ok(());
        }
        if status != 0 {
            return Err("无法修改当前用户开机自启设置。".into());
        }
        let name = wide(RUN_VALUE);
        let code = if let Some(data) = command {
            RegSetValueExW(
                h,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        } else {
            RegDeleteValueW(h, name.as_ptr())
        };
        RegCloseKey(h);
        if code == 0 || (exe.is_none() && code == ERROR_FILE_NOT_FOUND) {
            Ok(())
        } else {
            Err("更新开机自启失败。".into())
        }
    }
}

#[derive(Clone, Copy)]
pub enum Shortcut {
    Desktop,
    StartMenu,
}

struct ComApartment(bool);
impl ComApartment {
    fn enter() -> windows::core::Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result == RPC_E_CHANGED_MODE {
            // The UI thread may already be initialized by Rich Edit/OLE.
            Ok(Self(false))
        } else {
            result.ok()?;
            Ok(Self(true))
        }
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

fn path_wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn shortcut_folder(target: Shortcut) -> windows::core::Result<PathBuf> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};
    unsafe {
        let id = match target {
            Shortcut::Desktop => &FOLDERID_Desktop,
            Shortcut::StartMenu => &FOLDERID_Programs,
        };
        // Known folders also cover OneDrive/redirected desktops and localized Windows.
        let folder = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None)?;
        let path = PathBuf::from(OsString::from_wide(folder.as_wide()));
        CoTaskMemFree(Some(folder.0.cast()));
        Ok(path)
    }
}

pub fn create_shortcut(target: Shortcut) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|_| "无法定位程序文件。")?;
    let _apartment = ComApartment::enter().map_err(|e| format!("无法初始化快捷方式服务：{e}"))?;
    let folder = shortcut_folder(target).map_err(|e| format!("无法定位快捷方式目录：{e}"))?;
    std::fs::create_dir_all(&folder).map_err(|e| format!("无法创建快捷方式目录：{e}"))?;
    let destination = folder.join("轻取 OCR.lnk");
    save_shortcut(&exe, &destination).map_err(|e| format!("无法创建快捷方式：{e}"))?;
    Ok(destination)
}

fn save_shortcut(exe: &Path, destination: &Path) -> windows::core::Result<()> {
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        let target = path_wide(exe);
        link.SetPath(PCWSTR(target.as_ptr()))?;
        link.SetIconLocation(PCWSTR(target.as_ptr()), 0)?;
        if let Some(parent) = exe.parent() {
            link.SetWorkingDirectory(PCWSTR(path_wide(parent).as_ptr()))?;
        }
        link.SetDescription(windows::core::w!("轻取 OCR · 截图文字识别"))?;
        link.SetArguments(windows::core::w!(""))?;
        let file: IPersistFile = link.cast()?;
        file.Save(PCWSTR(path_wide(destination).as_ptr()), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_registration_quotes_unicode_paths_and_removal_is_repeatable() {
        let key = format!("Software\\QingquOCR.Tests\\Startup-{}", std::process::id());
        let first = Path::new("C:\\中文 路径\\轻取.exe");
        let second = Path::new("D:\\新版 路径\\轻取.exe");
        assert!(!startup_enabled_at(&key));
        for exe in [first, second] {
            set_startup_at(&key, Some(exe)).unwrap();
            assert!(startup_enabled_at(&key));
            unsafe {
                let mut data = [0u16; 1024];
                let mut size = std::mem::size_of_val(&data) as u32;
                let mut kind = 0;
                assert_eq!(
                    RegGetValueW(
                        HKEY_CURRENT_USER,
                        wide(&key).as_ptr(),
                        wide(RUN_VALUE).as_ptr(),
                        RRF_RT_REG_SZ,
                        &mut kind,
                        data.as_mut_ptr().cast(),
                        &mut size
                    ),
                    0
                );
                assert_eq!(kind, REG_SZ);
                assert_eq!(
                    String::from_utf16_lossy(&data[..size as usize / 2 - 1]),
                    format!("\"{}\" --tray", exe.display())
                );
            }
        }
        set_startup_at(&key, None).unwrap();
        set_startup_at(&key, None).unwrap();
        assert!(!startup_enabled_at(&key));
        unsafe {
            assert_eq!(RegDeleteKeyW(HKEY_CURRENT_USER, wide(&key).as_ptr()), 0);
        }
    }

    #[test]
    fn shell_shortcuts_roundtrip_unicode_target_icon_directory_and_update_in_place() {
        let _apartment = ComApartment::enter().unwrap();
        for target in [Shortcut::Desktop, Shortcut::StartMenu] {
            assert!(shortcut_folder(target).unwrap().is_absolute());
        }
        let folder =
            std::env::temp_dir().join(format!("qingqu-shortcut-{} 中文", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let destination = folder.join("轻取 OCR.lnk");
        let exe = std::env::current_exe().unwrap();
        let copy = folder.join("带 空格 𠮷.exe");
        std::fs::copy(&exe, &copy).unwrap();
        for target in [&exe, &copy] {
            save_shortcut(target, &destination).unwrap();
            unsafe {
                let link: IShellLinkW =
                    CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
                let file: IPersistFile = link.cast().unwrap();
                file.Load(
                    PCWSTR(path_wide(&destination).as_ptr()),
                    windows::Win32::System::Com::STGM_READ,
                )
                .unwrap();
                let mut buffer = [0u16; 32768];
                let decode = |b: &[u16]| {
                    String::from_utf16_lossy(&b[..b.iter().position(|&v| v == 0).unwrap()])
                };
                link.GetPath(&mut buffer, null_mut(), 0).unwrap();
                assert_eq!(Path::new(&decode(&buffer)), target);
                link.GetWorkingDirectory(&mut buffer).unwrap();
                assert_eq!(Path::new(&decode(&buffer)), target.parent().unwrap());
                let mut index = -1;
                link.GetIconLocation(&mut buffer, &mut index).unwrap();
                assert_eq!(Path::new(&decode(&buffer)), target);
                assert_eq!(index, 0);
                link.GetArguments(&mut buffer).unwrap();
                assert_eq!(decode(&buffer), "");
            }
        }
        std::fs::remove_file(destination).unwrap();
        std::fs::remove_file(copy).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}
