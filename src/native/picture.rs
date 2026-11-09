//! Image pin UI; originals and rotations are immutable shared views.
use super::*;

mod resample;
use crate::image_store::{ImageView, StoredImage};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Weak},
    time::Instant,
};
const DISPLAY_BUDGET: usize = 4 * 1024 * 1024;
const INFO_TOGGLE: u16 = 136;
const PREPARED_FRAME: &str = "QingquPicturePreparedFrame";
const CANVAS: &str = "QingquPictureCanvas";
pub const ZOOM_TIMER: usize = 21;

#[derive(Clone, Copy)]
struct ZoomAnchor {
    screen: POINT,
    image: (f32, f32),
    local: POINT,
    viewport: (i32, i32),
}
struct WheelMotion {
    target: f32,
    anchor: ZoomAnchor,
    last_frame: Instant,
    last_input: Instant,
    pixels: Option<crate::image_store::MappedPixels>,
    frame: RefCell<Option<Frame>>,
}

struct Canvas {
    bounds: RECT,
    visible: RECT,
}

pub unsafe fn visible_bounds(hwnd: HWND) -> RECT {
    let canvas = GetPropW(hwnd, platform::wide(CANVAS).as_ptr()) as *const Canvas;
    if let Some(canvas) = canvas.as_ref() {
        canvas.visible
    } else {
        let mut bounds = RECT::default();
        GetWindowRect(hwnd, &mut bounds);
        bounds
    }
}

fn visible_dimensions(hwnd: HWND) -> (i32, i32) {
    unsafe {
        let r = visible_bounds(hwnd);
        (
            (r.right - r.left - PICTURE_FRAME * 2).max(1),
            (r.bottom - r.top - PICTURE_FRAME * 2).max(1),
        )
    }
}

struct Frame {
    bitmap: Bitmap,
    width: i32,
    height: i32,
    zoom: f32,
    surface: RECT,
    offset: (i32, i32),
    canvas: bool,
}
impl Frame {
    unsafe fn print_full(&self, dc: HDC) {
        let (w, h) = if self.canvas {
            (
                self.surface.right - self.surface.left,
                self.surface.bottom - self.surface.top,
            )
        } else {
            (
                self.width + PICTURE_FRAME * 2,
                self.height + PICTURE_FRAME * 2,
            )
        };
        BitBlt(dc, 0, 0, w, h, self.bitmap.dc, 0, 0, SRCCOPY);
        GdiFlush();
    }
    unsafe fn present(&self, dc: HDC) {
        let (w, h) = if self.canvas {
            (
                self.surface.right - self.surface.left - PICTURE_FRAME * 2,
                self.surface.bottom - self.surface.top - PICTURE_FRAME * 2,
            )
        } else {
            (self.width, self.height)
        };
        BitBlt(
            dc,
            0,
            0,
            w,
            h,
            self.bitmap.dc,
            PICTURE_FRAME,
            PICTURE_FRAME,
            SRCCOPY,
        );
    }
    unsafe fn publish(&self, hwnd: HWND, left: i32, top: i32) -> bool {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if style & WS_EX_LAYERED == 0 {
            return false;
        }
        GdiFlush();
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let (left, top, width, height) = if self.canvas {
            (
                self.surface.left,
                self.surface.top,
                self.surface.right - self.surface.left,
                self.surface.bottom - self.surface.top,
            )
        } else {
            (
                left,
                top,
                self.width + PICTURE_FRAME * 2,
                self.height + PICTURE_FRAME * 2,
            )
        };
        let published = UpdateLayeredWindow(
            hwnd,
            null_mut(),
            &POINT { x: left, y: top },
            &SIZE {
                cx: width,
                cy: height,
            },
            self.bitmap.dc,
            &POINT::default(),
            0,
            if self.canvas { &blend } else { null() },
            if self.canvas { ULW_ALPHA } else { ULW_OPAQUE },
        ) != 0;
        if !published {
            // Allocation/driver failure retains the complete GDI fallback.
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (style & !WS_EX_LAYERED) as isize);
        }
        published
    }
}

// SetWindowPos/RedrawWindow can paint synchronously while Picture is borrowed.
// Publish only this immutable, fully rendered frame for the duration of resize.
struct PreparedFrame<'a> {
    hwnd: HWND,
    _frame: &'a Frame,
}
impl<'a> PreparedFrame<'a> {
    unsafe fn install(hwnd: HWND, frame: &'a Frame) -> Option<Self> {
        (SetPropW(
            hwnd,
            platform::wide(PREPARED_FRAME).as_ptr(),
            (frame as *const Frame).cast_mut().cast(),
        ) != 0)
            .then_some(Self {
                hwnd,
                _frame: frame,
            })
    }
}
impl Drop for PreparedFrame<'_> {
    fn drop(&mut self) {
        unsafe {
            RemovePropW(self.hwnd, platform::wide(PREPARED_FRAME).as_ptr());
        }
    }
}

pub unsafe fn is_prepared(hwnd: HWND) -> bool {
    !GetPropW(hwnd, platform::wide(PREPARED_FRAME).as_ptr()).is_null()
}

pub unsafe fn paint_prepared(hwnd: HWND, msg: u32, w: WPARAM) -> Option<LRESULT> {
    if !matches!(msg, WM_PAINT | WM_PRINT | WM_PRINTCLIENT) {
        return None;
    }
    let frame = GetPropW(hwnd, platform::wide(PREPARED_FRAME).as_ptr()) as *const Frame;
    let frame = frame.as_ref()?;
    if msg == WM_PAINT {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut ps);
        frame.present(dc);
        EndPaint(hwnd, &ps);
    } else if msg == WM_PRINT {
        frame.print_full(w as HDC);
    } else {
        frame.present(w as HDC);
    }
    Some(0)
}

pub unsafe fn print_full(hwnd: HWND, dc: HDC) -> bool {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Window>;
    let Some(cell) = ptr.as_ref() else {
        return false;
    };
    let Ok(window) = cell.try_borrow() else {
        return false;
    };
    let Content::Picture(picture) = &window.content else {
        return false;
    };
    let (w, h) = visible_dimensions(hwnd);
    if let Ok(frame) = picture.render(&window.host, w, h) {
        frame.print_full(dc);
    }
    true
}
#[derive(PartialEq, Eq)]
struct DisplayKey {
    original: usize,
    turns: u8,
    width: i32,
    height: i32,
    zoom: u32,
    x: i32,
    y: i32,
    phase: (u32, u32),
    background: u32,
}
struct Display {
    key: DisplayKey,
    original: Weak<StoredImage>,
    bitmap: Bitmap,
    bytes: usize,
}
thread_local! { static DISPLAYS: RefCell<VecDeque<Display>> = const { RefCell::new(VecDeque::new()) }; }
thread_local! { static PINS: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new()); }

pub struct Picture {
    pub pixels: ImageView,
    zoom: f32,
    fit: bool,
    x: i32,
    y: i32,
    registered: bool,
    info: bool,
    drag: Option<(POINT, POINT)>,
    motion: Option<Box<WheelMotion>>,
    phase: (f32, f32),
    resampler: RefCell<Option<resample::Renderer>>,
    canvas: Option<Box<Canvas>>,
}
impl Picture {
    pub fn new(pixels: ImageView) -> Self {
        Self {
            pixels,
            zoom: 1.,
            fit: true,
            x: 0,
            y: 0,
            registered: false,
            info: false,
            drag: None,
            motion: None,
            phase: (0., 0.),
            resampler: RefCell::new(None),
            canvas: None,
        }
    }
    pub fn with_info(mut self, visible: bool) -> Self {
        self.info = visible;
        self
    }
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        h.top = true;
        unsafe {
            super::picture_shadow::create(h.hwnd)?;
        }
        let id = Arc::as_ptr(&self.pixels.original) as usize;
        PINS.with(|pins| *pins.borrow_mut().entry(id).or_default() += 1);
        self.registered = true;
        Ok(())
    }
    fn info_height(&self, h: &Host) -> i32 {
        if self.info {
            h.px(24.)
        } else {
            0
        }
    }
    fn viewport(&self, h: &Host) -> (i32, i32) {
        let (w, height) = visible_dimensions(h.hwnd);
        (w.max(1), (height - self.info_height(h)).max(1))
    }
    pub fn show_info(&mut self, h: &mut Host, visible: bool) {
        if self.info == visible {
            return;
        }
        self.stop_motion(h);
        let old_height = self.info_height(h);
        self.info = visible;
        unsafe {
            let mut bounds = RECT::default();
            GetWindowRect(h.hwnd, &mut bounds);
            self.resize(
                h,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top + self.info_height(h) - old_height,
            );
        }
        self.layout(h);
    }
    pub fn layout(&mut self, h: &mut Host) {
        self.stop_motion(h);
        let (w, height) = self.viewport(h);
        if self.fit {
            self.zoom = (w as f32 / self.pixels.width() as f32)
                .min(height as f32 / self.pixels.height() as f32)
                .clamp(0.01, 1.);
            self.x = 0;
            self.y = 0;
            self.phase = (0., 0.);
        }
        self.update_scroll(h);
        h.invalidate();
        unsafe {
            if self.registered && GetWindowLongPtrW(h.hwnd, GWL_EXSTYLE) as u32 & WS_EX_LAYERED != 0
            {
                let (w, height) = dimensions(h.hwnd);
                if let Ok(frame) = self.render(h, w, height) {
                    let mut bounds = RECT::default();
                    GetWindowRect(h.hwnd, &mut bounds);
                    let guard = PreparedFrame::install(h.hwnd, &frame);
                    if guard.is_some() {
                        frame.publish(h.hwnd, bounds.left, bounds.top);
                    }
                }
            }
        }
    }
    fn update_scroll(&mut self, h: &Host) {
        let (w, height) = self.viewport(h);
        let (sw, sh) = (
            (self.pixels.width() as f32 * self.zoom).round() as i32,
            (self.pixels.height() as f32 * self.zoom).round() as i32,
        );
        self.x = self.x.clamp(0, (sw - w).max(0));
        self.y = self.y.clamp(0, (sh - height).max(0));
        // No native scrollbars: changing their visibility from WM_SIZE caused
        // repeated WM_SIZE/LAYOUT messages and visible flicker.
    }
    pub fn popup(&mut self, h: &mut Host) {
        self.stop_motion(h);
        let recognition = h
            .proxy
            .as_ref()
            .and_then(|proxy| proxy.picture_menu.lock().ok().map(|label| label.clone()))
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| "识别图片".into());
        unsafe {
            let id = menu_checked(
                h.hwnd,
                &[
                    (COPY, "复制 · Ctrl+C"),
                    (0, ""),
                    (132, "向左旋转 90°"),
                    (133, "向右旋转 90°"),
                    (ZOOM_IN, "放大"),
                    (ZOOM_OUT, "缩小"),
                    (130, "实际大小"),
                    (131, "适应窗口"),
                    (135, "输入缩放比例…"),
                    (INFO_TOGGLE, "显示图片信息"),
                    (PIN, if h.top { "取消置顶" } else { "置顶" }),
                    (134, &recognition),
                    (0, ""),
                    (CLOSE, "关闭 · Esc"),
                ],
                if self.info { &[INFO_TOGGLE] } else { &[] },
            );
            if id != 0 {
                PostMessageW(h.hwnd, COMMAND, id as usize, 0);
            }
        }
    }
    pub fn command(&mut self, h: &mut Host, id: u16) {
        match id {
            COPY => {
                let result = self.pixels.copy(h.hwnd);
                h.title = result
                    .map(|_| "图片剪贴板 · 已复制原始像素".into())
                    .unwrap_or_else(|e| e);
            }
            ZOOM_IN => {
                self.set_zoom(h, self.zoom * 1.1);
                return;
            }
            ZOOM_OUT => {
                self.set_zoom(h, self.zoom / 1.1);
                return;
            }
            130 | ZOOM_RESET => {
                self.set_zoom(h, 1.);
                return;
            }
            131 => {
                let (w, height) = picture_work_area(h.hwnd);
                self.set_zoom(
                    h,
                    (w as f32 / self.pixels.width() as f32)
                        .min(
                            (height - self.info_height(h)).max(1) as f32
                                / self.pixels.height() as f32,
                        )
                        .min(1.),
                );
                return;
            }
            INFO_TOGGLE => {
                self.show_info(h, !self.info);
                if let Some(proxy) = &h.proxy {
                    proxy.send(Notice::PictureInfo(self.info));
                }
                return;
            }
            135 => {
                if let Ok(window) = create(
                    "图片缩放比例",
                    (300., 145.),
                    Content::PictureZoom(picture_zoom::Zoom {
                        target: h.hwnd,
                        percent: (self.zoom * 100.).round() as u32,
                    }),
                    h.proxy.clone(),
                    &h.palette,
                    h.dark,
                    h.font_size,
                ) {
                    unsafe {
                        place_beside(window, h.hwnd);
                    }
                }
            }
            132 | 133 => {
                self.pixels.turns = (self.pixels.turns + if id == 132 { 3 } else { 1 }) % 4;
                self.x = 0;
                self.y = 0;
                self.phase = (0., 0.);
                self.set_zoom(h, self.zoom);
                return;
            }
            134 => {
                // Only share the handle here; materialize OCR pixels on its worker.
                if let Some(p) = &h.proxy {
                    p.send(Notice::ImageOcr(self.pixels.clone()));
                }
            }
            _ => {}
        }
        self.layout(h);
    }
    pub fn wheel(&mut self, h: &mut Host, w: usize, l: isize) {
        let delta = (w >> 16) as u16 as i16 as f32;
        if delta == 0. {
            return;
        }
        let screen = POINT {
            x: l as u16 as i16 as i32,
            y: (l as u32 >> 16) as u16 as i16 as i32,
        };
        let now = Instant::now();
        let target = (self
            .motion
            .as_ref()
            .map_or(self.zoom, |motion| motion.target)
            * 1.1_f32.powf(delta / 120.))
        .clamp(0.01, 8.);
        let anchor = self
            .motion
            .as_ref()
            .filter(|motion| {
                motion.anchor.screen.x == screen.x && motion.anchor.screen.y == screen.y
            })
            .map(|motion| motion.anchor)
            .unwrap_or_else(|| self.zoom_anchor(h, screen));
        if let Some(motion) = &mut self.motion {
            motion.target = target;
            motion.anchor = anchor;
            motion.last_input = now;
        } else if target != self.zoom {
            self.motion = Some(Box::new(WheelMotion {
                target,
                anchor,
                last_frame: now,
                last_input: now,
                pixels: self.pixels.original.map_owned().ok(),
                frame: RefCell::new(None),
            }));
            unsafe {
                // USER_TIMER_MINIMUM: Windows normally wakes this at ~16 ms.
                // More wheel messages update the target without queuing frames.
                if SetTimer(h.hwnd, ZOOM_TIMER, 10, None) == 0 {
                    self.stop_motion(h);
                    self.apply_zoom(h, target, anchor);
                }
            }
        }
    }
    pub fn timer(&mut self, h: &mut Host) {
        let Some(motion) = &mut self.motion else {
            return;
        };
        let now = Instant::now();
        let (zoom, finished) = advance_zoom(
            self.zoom,
            motion.target,
            now.duration_since(motion.last_frame).as_secs_f32(),
            now.duration_since(motion.last_input).as_secs_f32(),
        );
        motion.last_frame = now;
        let anchor = motion.anchor;
        self.apply_zoom(h, zoom, anchor);
        if finished {
            self.stop_motion(h);
        }
        if self.motion.is_some() && now.elapsed().as_secs_f32() >= 1. / 60. {
            // Keep the periodic timer when frames fit the refresh budget.
            // Re-arming every cheap frame adds a timer tick after rendering.
            // Expensive frames get a real input/CPU break before the next one.
            unsafe {
                KillTimer(h.hwnd, ZOOM_TIMER);
                if SetTimer(h.hwnd, ZOOM_TIMER, 10, None) == 0 {
                    let motion = self.motion.take().unwrap();
                    self.stop_motion(h);
                    self.apply_zoom(h, motion.target, motion.anchor);
                }
            }
        }
    }

    pub fn stop_motion(&mut self, h: &Host) {
        self.resampler.get_mut().take();
        if self.motion.take().is_some() {
            unsafe {
                KillTimer(h.hwnd, ZOOM_TIMER);
            }
        }
        if let Some(canvas) = self.canvas.take() {
            unsafe {
                RemovePropW(h.hwnd, platform::wide(CANVAS).as_ptr());
            }
            let r = canvas.visible;
            self.resize_at(h, r.left, r.top, r.right - r.left, r.bottom - r.top, true);
        }
    }
    pub fn set_zoom(&mut self, h: &mut Host, zoom: f32) {
        self.stop_motion(h);
        let (w, height) = self.viewport(h);
        let mut anchor = POINT {
            x: w / 2,
            y: height / 2,
        };
        unsafe {
            ClientToScreen(h.hwnd, &mut anchor);
        }
        self.zoom_at(h, zoom, anchor);
    }
    fn zoom_at(&mut self, h: &mut Host, zoom: f32, anchor: POINT) {
        let anchor = self.zoom_anchor(h, anchor);
        self.apply_zoom(h, zoom, anchor);
    }
    fn zoom_anchor(&self, h: &Host, anchor: POINT) -> ZoomAnchor {
        let (old_w, old_h) = self.viewport(h);
        let mut local = anchor;
        unsafe {
            let bounds = visible_bounds(h.hwnd);
            local.x -= bounds.left + PICTURE_FRAME;
            local.y -= bounds.top + PICTURE_FRAME;
        }
        // Footer and background anchors use the nearest point of the image.
        let image_x = (image_coordinate(local.x, old_w, self.pixels.width(), self.zoom, self.x)
            - self.phase.0 / self.zoom)
            .clamp(0., self.pixels.width() as f32);
        let image_y = (image_coordinate(local.y, old_h, self.pixels.height(), self.zoom, self.y)
            - self.phase.1 / self.zoom)
            .clamp(0., self.pixels.height() as f32);
        ZoomAnchor {
            screen: anchor,
            image: (image_x, image_y),
            local,
            viewport: (old_w, old_h),
        }
    }
    fn apply_zoom(&mut self, h: &mut Host, zoom: f32, anchor: ZoomAnchor) {
        let zoom = zoom.clamp(0.01, 8.);
        let (image_x, image_y) = anchor.image;
        let local = anchor.local;
        let (old_w, old_h) = anchor.viewport;
        self.fit = false;
        self.zoom = zoom;
        let (max_w, max_h) = picture_work_area(h.hwnd);
        let scaled_w = (self.pixels.width() as f32 * zoom).round().max(1.) as i32;
        let scaled_h = (self.pixels.height() as f32 * zoom).round().max(1.) as i32;
        let w = scaled_w.min(max_w);
        let height = scaled_h.min((max_h - self.info_height(h)).max(1));
        // Once the viewport reaches the monitor size, pan the cropped image to
        // keep the same source pixel under the cursor instead of resetting it.
        let (x, offset_x) = zoom_axis(image_x, zoom, scaled_w, w, local.x, old_w);
        let (y, offset_y) = zoom_axis(image_y, zoom, scaled_h, height, local.y, old_h);
        self.x = x;
        self.y = y;
        let left = anchor.screen.x as f32 - image_x * zoom - offset_x - PICTURE_FRAME as f32;
        let top = anchor.screen.y as f32 - image_y * zoom - offset_y - PICTURE_FRAME as f32;
        // Keep the whole-pixel window rounding remainder in the sampling transform.
        self.phase = (left - left.round(), top - top.round());
        if self.motion.is_some()
            && (self.canvas.is_some() || self.viewport(h) != (w, height))
            && unsafe { GetWindowLongPtrW(h.hwnd, GWL_EXSTYLE) as u32 & WS_EX_LAYERED != 0 }
        {
            self.prepare_canvas(
                h,
                rect(
                    left.round() as i32,
                    top.round() as i32,
                    w + PICTURE_FRAME * 2,
                    height + PICTURE_FRAME * 2 + self.info_height(h),
                ),
            );
        }
        let presented = self.resize_at(
            h,
            left.round() as i32,
            top.round() as i32,
            w + PICTURE_FRAME * 2,
            height + PICTURE_FRAME * 2 + self.info_height(h),
            true,
        );
        self.update_scroll(h);
        if !presented {
            h.invalidate();
        }
    }
    fn resize(&mut self, h: &Host, width: i32, height: i32) {
        unsafe {
            let mut bounds = RECT::default();
            GetWindowRect(h.hwnd, &mut bounds);
            self.resize_at(h, bounds.left, bounds.top, width, height, true);
        }
    }
    fn prepare_canvas(&mut self, h: &Host, mut visible: RECT) {
        let (left, top) = pin_position::safe_position(&visible, &pin_position::monitor_areas());
        visible = rect(
            left,
            top,
            visible.right - visible.left,
            visible.bottom - visible.top,
        );
        // Reserve a screen-aligned grid of pixels, independent of the image's
        // moving origin. Grow only when the image leaves that reservation.
        let mut padded = visible;
        if let Some(motion) = &self.motion {
            // Reserve through the next scale bucket, instead of repeatedly
            // growing for every wheel packet's small increase.
            let zoom = 1.25_f32
                .powf(motion.target.log(1.25).ceil())
                .clamp(0.01, 8.);
            let (max_w, max_h) = picture_work_area(h.hwnd);
            let sw = (self.pixels.width() as f32 * zoom).round().max(1.) as i32;
            let sh = (self.pixels.height() as f32 * zoom).round().max(1.) as i32;
            let w = sw.min(max_w);
            let height = sh.min((max_h - self.info_height(h)).max(1));
            let a = motion.anchor;
            let (_, ox) = zoom_axis(a.image.0, zoom, sw, w, a.local.x, a.viewport.0);
            let (_, oy) = zoom_axis(a.image.1, zoom, sh, height, a.local.y, a.viewport.1);
            let left = (a.screen.x as f32 - a.image.0 * zoom - ox).round() as i32 - PICTURE_FRAME;
            let top = (a.screen.y as f32 - a.image.1 * zoom - oy).round() as i32 - PICTURE_FRAME;
            padded.left = padded.left.min(left);
            padded.top = padded.top.min(top);
            padded.right = padded.right.max(left + w + PICTURE_FRAME * 2);
            padded.bottom = padded
                .bottom
                .max(top + height + self.info_height(h) + PICTURE_FRAME * 2);
        }
        unsafe {
            InflateRect(
                &mut padded,
                super::picture_shadow::PAD,
                super::picture_shadow::PAD,
            );
        }
        let bounds = canvas_bounds(&padded, self.canvas.as_ref().map(|c| c.bounds));
        let canvas = self
            .canvas
            .get_or_insert_with(|| Box::new(Canvas { bounds, visible }));
        canvas.bounds = bounds;
        canvas.visible = visible;
        unsafe {
            if SetPropW(
                h.hwnd,
                platform::wide(CANVAS).as_ptr(),
                (&**canvas as *const Canvas).cast_mut().cast(),
            ) == 0
            {
                self.canvas.take();
            }
        }
    }
    fn resize_at(
        &mut self,
        h: &Host,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        repaint: bool,
    ) -> bool {
        unsafe {
            let old_viewport = visible_dimensions(h.hwnd);
            let proposed = rect(left, top, width, height);
            let (left, top) =
                pin_position::safe_position(&proposed, &pin_position::monitor_areas());
            let frame = repaint
                .then(|| {
                    self.render(h, width - PICTURE_FRAME * 2, height - PICTURE_FRAME * 2)
                        .ok()
                })
                .flatten();
            let prepared = frame
                .as_ref()
                .and_then(|frame| PreparedFrame::install(h.hwnd, frame));
            // The compositor receives geometry and pixels in one operation.
            // Moving a GDI window first exposed the previous surface at the
            // new position/size, producing a backwards twitch between frames.
            let atomic = prepared.is_some()
                && frame
                    .as_ref()
                    .is_some_and(|frame| frame.publish(h.hwnd, left, top));
            if !atomic && self.canvas.is_some() {
                // Retry a compact surface if reservation allocation/upload
                // fails. Its DC must use compact geometry on the GDI fallback.
                drop(prepared);
                drop(frame);
                RemovePropW(h.hwnd, platform::wide(CANVAS).as_ptr());
                self.canvas.take();
                return self.resize_at(h, left, top, width, height, repaint);
            }
            if !atomic {
                SetWindowPos(
                    h.hwnd,
                    null_mut(),
                    left,
                    top,
                    width,
                    height,
                    SWP_NOZORDER
                        | SWP_NOACTIVATE
                        | if prepared.is_some() {
                            SWP_NOREDRAW | SWP_NOCOPYBITS
                        } else {
                            0
                        },
                );
            }
            let presented = prepared.is_some();
            if presented && !atomic {
                // Publish the whole new image before moving/resizing its shadow.
                RedrawWindow(
                    h.hwnd,
                    null(),
                    null_mut(),
                    RDW_INVALIDATE | RDW_FRAME | RDW_UPDATENOW | RDW_NOERASE,
                );
                GdiFlush();
            }
            // Image and shadow surfaces can each be viewport-sized. Release
            // the staging image before allocating the shadow to avoid peaks
            // from two simultaneous temporary full-window bitmaps.
            drop(prepared);
            if let Some(frame) = frame {
                if frame.canvas || old_viewport == (frame.width, frame.height) {
                    // A cropped large image keeps the same viewport while
                    // zooming. Reuse its DIB for this interaction instead of
                    // allocating/committing several MiB on every frame.
                    self.recycle_frame(frame);
                }
            }
            if presented {
                super::picture_shadow::sync(h.hwnd);
            }
            presented
        }
    }
    pub fn reset_position(&mut self, h: &Host, area: &RECT, index: usize) {
        self.stop_motion(h);
        self.drag = None;
        unsafe {
            if GetCapture() == h.hwnd {
                ReleaseCapture();
            }
            let bounds = visible_bounds(h.hwnd);
            let r = pin_position::reset_bounds(&bounds, area, index);
            self.resize_at(h, r.left, r.top, r.right - r.left, r.bottom - r.top, true);
        }
        self.update_scroll(h);
    }
    pub fn dpi_changed(&mut self, h: &mut Host, dpi: u32) {
        self.stop_motion(h);
        let old_footer = self.info_height(h);
        h.change_dpi(dpi);
        unsafe {
            let bounds = visible_bounds(h.hwnd);
            self.resize_at(
                h,
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top + self.info_height(h) - old_footer,
                true,
            );
        }
        self.layout(h);
    }
    pub fn mouse(&mut self, h: &Host, msg: u32, l: isize) {
        unsafe {
            match msg {
                WM_LBUTTONDOWN => {
                    let mut point = POINT {
                        x: l as u16 as i16 as i32,
                        y: (l as u32 >> 16) as u16 as i16 as i32,
                    };
                    ClientToScreen(h.hwnd, &mut point);
                    self.stop_motion(h);
                    let mut bounds = RECT::default();
                    GetWindowRect(h.hwnd, &mut bounds);
                    self.drag = Some((
                        point,
                        POINT {
                            x: bounds.left,
                            y: bounds.top,
                        },
                    ));
                    SetCapture(h.hwnd);
                }
                WM_MOUSEMOVE => {
                    if let Some((start, origin)) = self.drag {
                        let mut point = POINT {
                            x: l as u16 as i16 as i32,
                            y: (l as u32 >> 16) as u16 as i16 as i32,
                        };
                        ClientToScreen(h.hwnd, &mut point);
                        let mut bounds = RECT::default();
                        GetWindowRect(h.hwnd, &mut bounds);
                        self.resize_at(
                            h,
                            origin.x + point.x - start.x,
                            origin.y + point.y - start.y,
                            bounds.right - bounds.left,
                            bounds.bottom - bounds.top,
                            false,
                        );
                    }
                }
                WM_LBUTTONUP | WM_CANCELMODE => {
                    self.drag = None;
                    if GetCapture() == h.hwnd {
                        ReleaseCapture();
                    }
                }
                WM_CAPTURECHANGED => self.drag = None,
                _ => {}
            }
        }
    }
    pub fn scroll(&mut self, h: &mut Host, msg: u32, w: usize) {
        self.stop_motion(h);
        let mut info = SCROLLINFO {
            cbSize: size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            ..Default::default()
        };
        let horizontal = msg == WM_HSCROLL;
        unsafe {
            GetScrollInfo(
                h.hwnd,
                if horizontal { SB_HORZ } else { SB_VERT },
                &mut info,
            );
        }
        let position = if horizontal { &mut self.x } else { &mut self.y };
        match w as u16 as i32 {
            SB_LINEUP => *position -= 30,
            SB_LINEDOWN => *position += 30,
            SB_PAGEUP => *position -= info.nPage as i32,
            SB_PAGEDOWN => *position += info.nPage as i32,
            SB_THUMBTRACK | SB_THUMBPOSITION => *position = info.nTrackPos,
            SB_TOP => *position = 0,
            SB_BOTTOM => *position = info.nMax,
            _ => {}
        }
        self.fit = false;
        self.update_scroll(h);
        h.invalidate();
    }
    pub unsafe fn paint(&self, h: &Host, dc: HDC) {
        let (w, total) = visible_dimensions(h.hwnd);
        if let Ok(frame) = self.render(h, w, total) {
            frame.present(dc);
            GdiFlush();
            let mut bounds = RECT::default();
            GetWindowRect(h.hwnd, &mut bounds);
            let prepared = PreparedFrame::install(h.hwnd, &frame);
            if prepared.is_some() {
                frame.publish(h.hwnd, bounds.left, bounds.top);
            }
            drop(prepared);
            self.recycle_frame(frame);
        }
    }
    fn recycle_frame(&self, frame: Frame) {
        if let Some(motion) = &self.motion {
            *motion.frame.borrow_mut() = Some(frame);
        }
    }
    fn render(&self, h: &Host, w: i32, total: i32) -> Result<Frame, String> {
        let (surface, offset) = if let Some(canvas) = &self.canvas {
            (
                canvas.bounds,
                (
                    canvas.visible.left - canvas.bounds.left,
                    canvas.visible.top - canvas.bounds.top,
                ),
            )
        } else {
            (
                rect(0, 0, w + PICTURE_FRAME * 2, total + PICTURE_FRAME * 2),
                (0, 0),
            )
        };
        let surface_size = (surface.right - surface.left, surface.bottom - surface.top);
        let reusable = self
            .motion
            .as_ref()
            .and_then(|motion| motion.frame.borrow_mut().take())
            .filter(|frame| {
                (
                    frame.surface.right - frame.surface.left,
                    frame.surface.bottom - frame.surface.top,
                ) == surface_size
            });
        let mut frame = match reusable {
            Some(frame) => frame,
            None => Frame {
                bitmap: Bitmap::new(surface_size.0, surface_size.1)?,
                width: w,
                height: total,
                zoom: self.zoom,
                surface,
                offset,
                canvas: self.canvas.is_some(),
            },
        };
        frame.zoom = self.zoom;
        frame.width = w;
        frame.height = total;
        frame.surface = surface;
        frame.offset = offset;
        frame.canvas = self.canvas.is_some();
        unsafe {
            if frame.canvas {
                GdiFlush();
                std::slice::from_raw_parts_mut(
                    frame.bitmap.pixels,
                    (surface_size.0 * surface_size.1 * 4) as usize,
                )
                .fill(0);
                super::picture_shadow::paint_canvas(
                    &frame.bitmap,
                    surface_size.0,
                    surface_size.1,
                    rect(
                        offset.0,
                        offset.1,
                        w + PICTURE_FRAME * 2,
                        total + PICTURE_FRAME * 2,
                    ),
                );
            }
            self.paint_frame(h, &frame);
            if frame.canvas {
                GdiFlush();
                qingqu_picture_renderer::opaque_rectangle(
                    std::slice::from_raw_parts_mut(
                        frame.bitmap.pixels,
                        (surface_size.0 * surface_size.1 * 4) as usize,
                    ),
                    surface_size.0 as usize * 4,
                    (offset.0 as usize, offset.1 as usize),
                    (
                        (w + PICTURE_FRAME * 2) as usize,
                        (total + PICTURE_FRAME * 2) as usize,
                    ),
                );
            }
        }
        Ok(frame)
    }
    unsafe fn paint_frame(&self, h: &Host, frame: &Frame) {
        let (w, total) = (frame.width, frame.height);
        let height = (total - self.info_height(h)).max(1);
        let dc = frame.bitmap.dc;
        let previous = SaveDC(dc);
        SetViewportOrgEx(dc, frame.offset.0, frame.offset.1, null_mut());
        paint_window_frame_at(h.hwnd, dc, w + PICTURE_FRAME * 2, total + PICTURE_FRAME * 2);
        SetViewportOrgEx(
            dc,
            frame.offset.0 + PICTURE_FRAME,
            frame.offset.1 + PICTURE_FRAME,
            null_mut(),
        );
        let key = DisplayKey {
            original: Arc::as_ptr(&self.pixels.original) as usize,
            turns: self.pixels.turns,
            width: w,
            height,
            zoom: self.zoom.to_bits(),
            x: self.x,
            y: self.y,
            phase: (self.phase.0.to_bits(), self.phase.1.to_bits()),
            background: h.theme.bg,
        };
        let bytes = w as usize * height as usize * 4;
        let cached = DISPLAYS.with(|cache| {
            // Intermediate animation frames must not evict settled pin caches.
            if self.motion.is_some() {
                return false;
            }
            let Ok(mut cache) = cache.try_borrow_mut() else {
                return false;
            };
            cache.retain(|entry| entry.original.strong_count() > 0);
            if let Some(index) = cache.iter().position(|entry| entry.key == key) {
                let entry = cache.remove(index).unwrap();
                BitBlt(dc, 0, 0, w, height, entry.bitmap.dc, 0, 0, SRCCOPY);
                cache.push_back(entry);
                return true;
            }
            if bytes > DISPLAY_BUDGET {
                return false;
            }
            while cache.iter().map(|entry| entry.bytes).sum::<usize>() + bytes > DISPLAY_BUDGET {
                cache.pop_front();
            }
            let Ok(bitmap) = Bitmap::new(w, height) else {
                return false;
            };
            self.paint_uncached(h, &bitmap, (w, height), w as usize * 4, 0);
            BitBlt(dc, 0, 0, w, height, bitmap.dc, 0, 0, SRCCOPY);
            cache.push_back(Display {
                key,
                original: Arc::downgrade(&self.pixels.original),
                bitmap,
                bytes,
            });
            true
        });
        if !cached {
            let stride = (frame.surface.right - frame.surface.left) as usize * 4;
            let offset = (frame.offset.1 + PICTURE_FRAME) as usize * stride
                + (frame.offset.0 + PICTURE_FRAME) as usize * 4;
            self.paint_uncached(h, &frame.bitmap, (w, height), stride, offset);
        }
        if self.info {
            if total <= height {
                RestoreDC(dc, previous);
                return;
            }
            fill(dc, &rect(0, height, w, total - height), h.theme.surface);
            fill(dc, &rect(0, height, w, 1), h.theme.border);
            label(
                dc,
                &rect(
                    h.px(8.),
                    height + 1,
                    (w - h.px(16.)).max(0),
                    (total - height - 1).max(0),
                ),
                &format!(
                    "{} × {} px · {} · {:.0}%",
                    self.pixels.width(),
                    self.pixels.height(),
                    image_size_label(&self.pixels),
                    self.zoom * 100.
                ),
                h.theme.muted,
                h.font.0,
                DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
        }
        RestoreDC(dc, previous);
    }
    unsafe fn paint_uncached(
        &self,
        h: &Host,
        bitmap: &Bitmap,
        viewport: (i32, i32),
        stride: usize,
        offset: usize,
    ) {
        let (w, height) = viewport;
        let transient;
        let pixels = if let Some(pixels) = self
            .motion
            .as_ref()
            .and_then(|motion| motion.pixels.as_ref())
        {
            &**pixels
        } else {
            let Ok(mapped) = self.pixels.original.map() else {
                fill(bitmap.dc, &rect(0, 0, w, height), h.theme.bg);
                return;
            };
            transient = mapped;
            &*transient
        };
        let bg = [
            (h.theme.bg >> 16 & 255) as u8,
            (h.theme.bg >> 8 & 255) as u8,
            (h.theme.bg & 255) as u8,
            255,
        ];
        // A transient full frame avoids visible strips and DIB/GDI batching
        // races from overwriting a strip before its previous blit has finished.
        // It is released after presentation; retained cache stays at 4 MiB.
        let dest = std::slice::from_raw_parts_mut(
            bitmap.pixels.add(offset),
            (height as usize - 1) * stride + w as usize * 4,
        );
        let mut workspace = self.resampler.borrow_mut();
        let mut transient = resample::Renderer::default();
        let renderer = if self.motion.is_some() {
            workspace.get_or_insert_with(resample::Renderer::default)
        } else {
            &mut transient
        };
        renderer.render_strided(
            &self.pixels,
            pixels,
            resample::Transform {
                viewport: (w, height),
                zoom: self.zoom,
                scroll: (self.x as f32 - self.phase.0, self.y as f32 - self.phase.1),
                background: bg,
            },
            dest,
            stride,
        );
    }
}

fn advance_zoom(current: f32, target: f32, elapsed: f32, since_input: f32) -> (f32, bool) {
    let distance = (target / current).ln();
    // Time-based logarithmic interpolation gives the same response at varying
    // frame rates. It never overshoots or accumulates a queue of old wheel steps.
    if distance.abs() < 0.0001 || since_input >= 0.24 {
        (target, true)
    } else {
        (
            current * (distance * (1. - (-elapsed / 0.060).exp())).exp(),
            false,
        )
    }
}

fn image_size_label(image: &ImageView) -> String {
    let (kind, bytes) = match image.png_bytes {
        Some(bytes) => ("PNG", bytes as u64),
        None => (
            "位图",
            u64::from(image.width()) * u64::from(image.height()) * 4,
        ),
    };
    if bytes >= 1024 * 1024 {
        format!("{kind} {:.2} MB", bytes as f64 / (1024. * 1024.))
    } else {
        format!("{kind} {:.1} KB", bytes as f64 / 1024.)
    }
}

fn image_coordinate(local: i32, viewport: i32, pixels: u32, zoom: f32, scroll: i32) -> f32 {
    let scaled = (pixels as f32 * zoom).round() as i32;
    ((local - ((viewport - scaled) / 2).max(0) + scroll) as f32 / zoom).clamp(0., pixels as f32)
}

fn zoom_axis(
    image: f32,
    zoom: f32,
    scaled: i32,
    viewport: i32,
    local: i32,
    old_viewport: i32,
) -> (i32, f32) {
    let target = local.clamp(0, old_viewport) as f32 * viewport as f32 / old_viewport as f32;
    let scroll = (image * zoom - target)
        .round()
        .clamp(0., (scaled - viewport).max(0) as f32) as i32;
    (
        scroll,
        ((viewport - scaled) / 2).max(0) as f32 - scroll as f32,
    )
}

fn canvas_bounds(visible: &RECT, previous: Option<RECT>) -> RECT {
    const GRID: i32 = 256;
    let reserve = rect(
        visible.left.div_euclid(GRID) * GRID,
        visible.top.div_euclid(GRID) * GRID,
        (visible.right + GRID - 1).div_euclid(GRID) * GRID - visible.left.div_euclid(GRID) * GRID,
        (visible.bottom + GRID - 1).div_euclid(GRID) * GRID - visible.top.div_euclid(GRID) * GRID,
    );
    if let Some(old) = previous {
        let union = RECT {
            left: old.left.min(reserve.left),
            top: old.top.min(reserve.top),
            right: old.right.max(reserve.right),
            bottom: old.bottom.max(reserve.bottom),
        };
        // Moving an anchor across monitors must not retain an unbounded
        // transparent envelope. Only the current reservation is then needed.
        let bytes = i64::from(union.right - union.left) * i64::from(union.bottom - union.top) * 4;
        let reserved =
            i64::from(reserve.right - reserve.left) * i64::from(reserve.bottom - reserve.top) * 4;
        if bytes <= 32 * 1024 * 1024 && bytes <= reserved + 4 * 1024 * 1024 {
            union
        } else {
            reserve
        }
    } else {
        reserve
    }
}

impl Drop for Picture {
    fn drop(&mut self) {
        if !self.registered {
            return;
        }
        let id = Arc::as_ptr(&self.pixels.original) as usize;
        let last_pin = PINS.with(|pins| {
            let mut pins = pins.borrow_mut();
            let Some(count) = pins.get_mut(&id) else {
                return true;
            };
            *count -= 1;
            if *count != 0 {
                return false;
            }
            pins.remove(&id);
            true
        });
        // OCR may still own the original; display resources belong to pins.
        if last_pin {
            DISPLAYS.with(|cache| {
                if let Ok(mut cache) = cache.try_borrow_mut() {
                    cache.retain(|entry| entry.key.original != id);
                }
            });
        }
    }
}

fn picture_work_area(hwnd: HWND) -> (i32, i32) {
    unsafe {
        let mut monitor = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(
            MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
            &mut monitor,
        );
        (
            (monitor.rcWork.right - monitor.rcWork.left - 24).max(1),
            (monitor.rcWork.bottom - monitor.rcWork.top - 24).max(1),
        )
    }
}
