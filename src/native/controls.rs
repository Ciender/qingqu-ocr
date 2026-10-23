//! Draw native controls with the app palette; Windows retains keyboard, focus, and dropdown behavior.
use super::*;

fn theme(hwnd: HWND) -> Theme {
    unsafe {
        let parent = GetParent(hwnd);
        let index =
            (GetPropW(parent, platform::wide("QingquTheme").as_ptr()) as usize).saturating_sub(1);
        Theme::new(
            if index >= 2 { "catppuccin" } else { "vscode" },
            index % 2 == 1,
        )
    }
}
fn scale(hwnd: HWND, n: f32) -> i32 {
    unsafe {
        let dpi =
            (GetPropW(GetParent(hwnd), platform::wide("QingquDpi").as_ptr()) as usize).max(96);
        (n * dpi as f32 / 96.).round() as i32
    }
}
pub unsafe fn install(hwnd: HWND, class: &str, id: u16) {
    if class == "COMBOBOX" {
        SetWindowSubclass(hwnd, Some(combo_proc), 0x4342, 0);
        let height = scale(hwnd, 22.);
        SendMessageW(hwnd, CB_SETITEMHEIGHT, usize::MAX, (height - 2) as isize);
        SendMessageW(hwnd, CB_SETITEMHEIGHT, 0, scale(hwnd, 26.) as isize);
        SendMessageW(hwnd, CB_SETMINVISIBLE, 12, 0);
        SendMessageW(hwnd, CB_SETDROPPEDWIDTH, scale(hwnd, 180.) as usize, 0);
        let mut info = COMBOBOXINFO {
            cbSize: size_of::<COMBOBOXINFO>() as u32,
            ..Default::default()
        };
        if GetComboBoxInfo(hwnd, &mut info) != 0 {
            SetWindowTheme(
                info.hwndList,
                platform::wide(if theme(hwnd).bg == color(0xffffff) {
                    "Explorer"
                } else {
                    "DarkMode_Explorer"
                })
                .as_ptr(),
                null(),
            );
        }
    } else if class == "BUTTON" {
        SetWindowSubclass(hwnd, Some(check_proc), 0x4348, 0);
    } else if class == "LISTBOX" && id == 500 {
        SendMessageW(hwnd, LB_SETITEMHEIGHT, 0, scale(hwnd, 35.) as isize);
    }
}
unsafe fn control_font(hwnd: HWND) -> HFONT {
    SendMessageW(hwnd, WM_GETFONT, 0, 0) as HFONT
}
unsafe fn draw_combo(hwnd: HWND, dc: HDC) {
    let t = theme(hwnd);
    let (w, height) = dimensions(hwnd);
    let r = rect(0, 0, w, height);
    fill(dc, &r, t.bg);
    let b = Brush::new(t.border);
    FrameRect(dc, &r, b.0);
    let index = SendMessageW(hwnd, CB_GETCURSEL, 0, 0);
    let value = if index >= 0 {
        let n = SendMessageW(hwnd, CB_GETLBTEXTLEN, index as usize, 0).max(0) as usize;
        let mut s = vec![0u16; n + 1];
        SendMessageW(hwnd, CB_GETLBTEXT, index as usize, s.as_mut_ptr() as isize);
        String::from_utf16_lossy(&s[..n])
    } else {
        String::new()
    };
    label(
        dc,
        &rect(scale(hwnd, 7.), 0, (w - scale(hwnd, 28.)).max(0), height),
        &value,
        if IsWindowEnabled(hwnd) != 0 {
            t.text
        } else {
            t.muted
        },
        control_font(hwnd),
        DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );
    let x = w - scale(hwnd, 12.);
    let y = height / 2;
    let pen = CreatePen(PS_SOLID, scale(hwnd, 1.).max(1), t.muted);
    let old = SelectObject(dc, pen);
    MoveToEx(dc, x - scale(hwnd, 3.), y - scale(hwnd, 1.), null_mut());
    LineTo(dc, x, y + scale(hwnd, 2.));
    LineTo(dc, x + scale(hwnd, 4.), y - scale(hwnd, 2.));
    SelectObject(dc, old);
    DeleteObject(pen);
    if GetFocus() == hwnd {
        let mut focus = r;
        InflateRect(&mut focus, -2, -2);
        let b = Brush::new(t.accent);
        FrameRect(dc, &focus, b.0);
    }
}
unsafe extern "system" fn combo_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            draw_combo(hwnd, dc);
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_PRINTCLIENT => {
            draw_combo(hwnd, w as HDC);
            return 0;
        }
        WM_MOUSEMOVE => {
            let mut track = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            TrackMouseEvent(&mut track);
        }
        _ => {}
    }
    let result = DefSubclassProc(hwnd, msg, w, l);
    if matches!(
        msg,
        CB_SETCURSEL | WM_SETFOCUS | WM_KILLFOCUS | WM_ENABLE | WM_MOUSELEAVE
    ) {
        InvalidateRect(hwnd, null(), 0);
    }
    result
}
unsafe fn draw_check(hwnd: HWND, dc: HDC) {
    let t = theme(hwnd);
    let (w, height) = dimensions(hwnd);
    fill(dc, &rect(0, 0, w, height), t.bg);
    let size = scale(hwnd, 13.);
    let r = rect(0, (height - size) / 2, size, size);
    let checked = SendMessageW(hwnd, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
    fill(dc, &r, if checked { t.accent } else { t.bg });
    let b = Brush::new(if checked { t.accent } else { t.muted });
    FrameRect(dc, &r, b.0);
    if checked {
        let ink = if t.bg == color(0xffffff) || t.bg == color(0xeff1f5) {
            color(0xffffff)
        } else {
            t.bg
        };
        let pen = CreatePen(PS_SOLID, scale(hwnd, 2.).max(1), ink);
        let old = SelectObject(dc, pen);
        MoveToEx(dc, r.left + scale(hwnd, 3.), r.top + size / 2, null_mut());
        LineTo(dc, r.left + size / 2, r.bottom - scale(hwnd, 3.));
        LineTo(dc, r.right - scale(hwnd, 2.), r.top + scale(hwnd, 3.));
        SelectObject(dc, old);
        DeleteObject(pen);
    }
    label(
        dc,
        &rect(
            size + scale(hwnd, 7.),
            0,
            w - size - scale(hwnd, 7.),
            height,
        ),
        &text(hwnd),
        t.text,
        control_font(hwnd),
        DT_SINGLELINE | DT_VCENTER,
    );
}
unsafe extern "system" fn check_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            draw_check(hwnd, dc);
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_PRINTCLIENT => {
            draw_check(hwnd, w as HDC);
            return 0;
        }
        _ => {}
    }
    let result = DefSubclassProc(hwnd, msg, w, l);
    if matches!(
        msg,
        BM_SETCHECK | WM_LBUTTONUP | WM_SETFOCUS | WM_KILLFOCUS | WM_ENABLE
    ) {
        InvalidateRect(hwnd, null(), 0);
    }
    result
}
pub unsafe fn draw_item(h: &Host, item: &DRAWITEMSTRUCT) -> bool {
    let combo = item.CtlType == ODT_COMBOBOX;
    if !combo && item.CtlType != ODT_LISTBOX {
        return false;
    }
    let selected = item.itemState & ODS_SELECTED != 0;
    let t = h.theme;
    fill(
        item.hDC,
        &item.rcItem,
        if selected {
            t.hover
        } else if combo {
            t.bg
        } else {
            t.surface
        },
    );
    if item.itemID == u32::MAX {
        return true;
    }
    let n = SendMessageW(
        item.hwndItem,
        if combo {
            CB_GETLBTEXTLEN
        } else {
            LB_GETTEXTLEN
        },
        item.itemID as usize,
        0,
    )
    .max(0) as usize;
    let mut value = vec![0u16; n + 1];
    SendMessageW(
        item.hwndItem,
        if combo { CB_GETLBTEXT } else { LB_GETTEXT },
        item.itemID as usize,
        value.as_mut_ptr() as isize,
    );
    let padding = h.px(if combo { 8. } else { 12. });
    let r = rect(
        item.rcItem.left + padding,
        item.rcItem.top,
        item.rcItem.right - item.rcItem.left - 2 * padding,
        item.rcItem.bottom - item.rcItem.top,
    );
    label(
        item.hDC,
        &r,
        &String::from_utf16_lossy(&value[..n]),
        t.text,
        h.font.0,
        DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );
    if selected && !combo {
        fill(
            item.hDC,
            &rect(
                item.rcItem.left,
                item.rcItem.top,
                h.px(3.),
                item.rcItem.bottom - item.rcItem.top,
            ),
            t.accent,
        );
    }
    true
}
