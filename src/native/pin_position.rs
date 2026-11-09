//! Recovery limits across physical monitors; work areas are for explicit reset.
use super::win::*;
use std::{mem::size_of, ptr::null};

pub fn monitor_areas() -> Vec<RECT> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            (*(data as *mut Vec<RECT>)).push(info.rcMonitor);
        }
        1
    }
    let mut areas = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            null(),
            Some(collect),
            (&mut areas as *mut Vec<RECT>) as LPARAM,
        );
    }
    areas
}

pub fn safe_position(bounds: &RECT, areas: &[RECT]) -> (i32, i32) {
    let width = (bounds.right - bounds.left).max(1);
    let height = (bounds.bottom - bounds.top).max(1);
    // A monitor seam is never a wall. Leave a valid proposal unchanged on
    // any monitor, otherwise apply the smallest correction to a real screen.
    // The physical screen edge defines 95% overflow. A taskbar's inner edge
    // must not stop a pin early; only explicit reset uses the work area.
    areas
        .iter()
        .filter(|area| area.right > area.left && area.bottom > area.top)
        .map(|area| {
            let visible_w = ((width + 19) / 20).min(area.right - area.left);
            let visible_h = ((height + 19) / 20).min(area.bottom - area.top);
            let x = bounds
                .left
                .clamp(area.left + visible_w - width, area.right - visible_w);
            let y = bounds
                .top
                .clamp(area.top + visible_h - height, area.bottom - visible_h);
            let dx = i64::from(x) - i64::from(bounds.left);
            let dy = i64::from(y) - i64::from(bounds.top);
            ((x, y), dx * dx + dy * dy)
        })
        .min_by_key(|(_, distance)| *distance)
        .map_or((bounds.left, bounds.top), |(position, _)| position)
}

pub fn reset_bounds(bounds: &RECT, area: &RECT, index: usize) -> RECT {
    let aw = (area.right - area.left).max(1);
    let ah = (area.bottom - area.top).max(1);
    let width = (bounds.right - bounds.left).clamp(1, aw);
    let height = (bounds.bottom - bounds.top).clamp(1, ah);
    let offset = (index % 8) as i32 * 24;
    let left = area.left + ((aw - width) / 2 + offset).min(aw - width);
    let top = area.top + ((ah - height) / 2 + offset).min(ah - height);
    rect(left, top, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_edges_retain_exactly_five_percent_even_for_small_pins() {
        for area in [rect(0, 0, 1920, 1040), rect(-1920, -1080, 1920, 1080)] {
            for (w, h) in [(640, 360), (20, 10), (1, 1), (4000, 3000)] {
                for (x, y) in [
                    (-20000, -20000),
                    (20000, 20000),
                    (-20000, 20000),
                    (20000, -20000),
                ] {
                    let (x, y) = safe_position(&rect(x, y, w, h), &[area]);
                    assert_eq!((x + w).min(area.right) - x.max(area.left), (w + 19) / 20);
                    assert_eq!((y + h).min(area.bottom) - y.max(area.top), (h + 19) / 20);
                }
            }
        }
    }

    #[test]
    fn taskbars_do_not_stop_dragging_at_the_inner_work_area_edge() {
        let screen = rect(0, 0, 1920, 1080);
        for work in [
            rect(0, 0, 1920, 1040),
            rect(0, 40, 1920, 1040),
            rect(40, 0, 1880, 1080),
            rect(0, 0, 1880, 1080),
        ] {
            let bounds = rect(20000, 20000, 20, 10);
            assert_eq!(
                safe_position(&bounds, &[screen]),
                (screen.right - 1, screen.bottom - 1)
            );
            assert_eq!(
                safe_position(&rect(-20000, -20000, 20, 10), &[screen]),
                (screen.left - 19, screen.top - 9)
            );
            let reset = reset_bounds(&bounds, &work, 0);
            assert!(reset.left >= work.left && reset.top >= work.top);
            assert!(reset.right <= work.right && reset.bottom <= work.bottom);
        }
    }

    #[test]
    fn seams_allow_continuous_transfer_in_both_directions() {
        for areas in [
            [rect(0, 0, 1920, 1040), rect(1920, -300, 2560, 1400)],
            [rect(-1920, 0, 1920, 1080), rect(0, 0, 1920, 1040)],
            [rect(0, 0, 1920, 1080), rect(0, 1080, 1920, 1080)],
        ] {
            for pair in [(areas[0], areas[1]), (areas[1], areas[0])] {
                for step in 0..=100 {
                    let x = pair.0.left + 100 + (pair.1.left - pair.0.left) * step / 100;
                    let y = pair.0.top + 100 + (pair.1.top - pair.0.top) * step / 100;
                    assert_eq!(safe_position(&rect(x, y, 640, 360), &areas), (x, y));
                }
            }
        }
    }

    #[test]
    fn gaps_and_missing_corners_are_not_part_of_the_desktop() {
        for areas in [
            [rect(0, 0, 1920, 1040), rect(2560, 0, 1920, 1040)],
            [rect(0, 0, 1920, 1040), rect(1920, -1080, 1920, 1080)],
        ] {
            let proposed = rect(2050, 500, 200, 100);
            let (x, y) = safe_position(&proposed, &areas);
            assert_ne!((x, y), (proposed.left, proposed.top));
            assert!(areas
                .iter()
                .any(|a| (x + 200).min(a.right) - x.max(a.left) >= 10
                    && (y + 100).min(a.bottom) - y.max(a.top) >= 5));
            // Once clamped, a stationary pointer cannot make the pin oscillate.
            assert_eq!(safe_position(&rect(x, y, 200, 100), &areas), (x, y));
        }
    }

    #[test]
    fn reset_contains_every_pin_on_the_target_monitor() {
        let area = rect(-1280, -200, 1280, 984);
        for bounds in [
            rect(20000, -20000, 640, 360),
            rect(20000, 20000, 4000, 3000),
        ] {
            for index in 0..20 {
                let reset = reset_bounds(&bounds, &area, index);
                assert!(reset.left >= area.left && reset.top >= area.top);
                assert!(reset.right <= area.right && reset.bottom <= area.bottom);
            }
        }
    }
}
