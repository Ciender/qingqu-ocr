//! Three-row paragraph chooser anchored to its toolbar button.
use super::*;

const MENU_STYLE: &str = "QingquParagraphMenu";
const ITEM_WIDTH: f32 = 132.;
const ITEM_HEIGHT: f32 = 30.;
pub const CHOICES: [(u16, &str, Icon); 3] = [
    (AUTO_PARAGRAPH, "自动分段", Icon::AutoParagraph),
    (SPLIT, "段落拆分", Icon::Split),
    (MERGE, "段落合并", Icon::Merge),
];

struct MenuStyle {
    dpi: u32,
    theme: Theme,
    font: Font,
    atlas: Atlas,
    current: u16,
}
impl MenuStyle {
    fn px(&self, value: f32) -> i32 {
        (value * self.dpi as f32 / 96.).round() as i32
    }
}

pub fn choose(h: &Host, current: &str) -> Result<Option<u16>, String> {
    let style = MenuStyle {
        dpi: h.dpi,
        theme: h.theme,
        font: Font::new(12., h.dpi),
        atlas: Atlas::new(h.px(19.), h.theme.text)?,
        current: match current {
            "split" => SPLIT,
            "merge" => MERGE,
            _ => AUTO_PARAGRAPH,
        },
    };
    unsafe {
        let popup = CreatePopupMenu();
        if popup.is_null() {
            return Err("无法打开分段选择菜单。".into());
        }
        let info = MENUINFO {
            cbSize: size_of::<MENUINFO>() as u32,
            fMask: MIM_STYLE,
            dwStyle: MNS_NOCHECK,
            ..Default::default()
        };
        SetMenuInfo(popup, &info);
        for (index, (id, name, _)) in CHOICES.iter().enumerate() {
            let mut caption = platform::wide(name);
            let item = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_FTYPE | MIIM_ID | MIIM_DATA | MIIM_STRING,
                fType: MFT_OWNERDRAW,
                wID: *id as u32,
                dwItemData: index,
                dwTypeData: caption.as_mut_ptr(),
                ..Default::default()
            };
            InsertMenuItemW(popup, index as u32, 1, &item);
        }
        let mut bounds = RECT::default();
        GetWindowRect(GetDlgItem(h.hwnd, PARAGRAPH as i32), &mut bounds);
        let property = platform::wide(MENU_STYLE);
        SetPropW(
            h.hwnd,
            property.as_ptr(),
            (&style as *const MenuStyle) as HANDLE,
        );
        // TPM_RETURNCMD keeps the selection explicit and cancellation inert.
        let result = TrackPopupMenu(
            popup,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_LEFTALIGN | TPM_TOPALIGN,
            bounds.left,
            bounds.bottom,
            0,
            h.hwnd,
            null(),
        );
        RemovePropW(h.hwnd, property.as_ptr());
        DestroyMenu(popup);
        Ok((result != 0).then_some(result as u16))
    }
}

/// Handle only this menu before borrowing the main window's state.
pub unsafe fn message(hwnd: HWND, msg: u32, l: LPARAM) -> Option<LRESULT> {
    if !matches!(msg, WM_MEASUREITEM | WM_DRAWITEM) || l == 0 {
        return None;
    }
    let style = GetPropW(hwnd, platform::wide(MENU_STYLE).as_ptr()) as *const MenuStyle;
    if style.is_null() {
        return None;
    }
    let style = &*style;
    if msg == WM_MEASUREITEM {
        let item = &mut *(l as *mut MEASUREITEMSTRUCT);
        if item.CtlType != ODT_MENU {
            return None;
        }
        item.itemWidth = style.px(ITEM_WIDTH) as u32;
        item.itemHeight = style.px(ITEM_HEIGHT) as u32;
        return Some(1);
    }
    let item = &*(l as *const DRAWITEMSTRUCT);
    if item.CtlType != ODT_MENU {
        return None;
    }
    let &(id, name, icon) = CHOICES.get(item.itemData)?;
    let hot = item.itemState & ODS_SELECTED != 0;
    fill(
        item.hDC,
        &item.rcItem,
        if hot {
            style.theme.hover
        } else {
            style.theme.surface
        },
    );
    let r = item.rcItem;
    style.atlas.paint(
        item.hDC,
        rect(
            r.left + style.px(4.),
            r.top,
            style.px(26.),
            r.bottom - r.top,
        ),
        icon,
    );
    label(
        item.hDC,
        &rect(
            r.left + style.px(35.),
            r.top,
            style.px(78.),
            r.bottom - r.top,
        ),
        name,
        style.theme.text,
        style.font.0,
        DT_SINGLELINE | DT_VCENTER,
    );
    if id == style.current {
        label(
            item.hDC,
            &rect(
                r.right - style.px(20.),
                r.top,
                style.px(16.),
                r.bottom - r.top,
            ),
            "✓",
            style.theme.text,
            style.font.0,
            DT_SINGLELINE | DT_VCENTER | DT_CENTER,
        );
    }
    Some(1)
}
