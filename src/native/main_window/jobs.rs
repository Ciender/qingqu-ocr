//! Task lifecycle and result delivery, independent from window layout.
use super::*;
impl Main {
    pub fn begin(
        &mut self,
        h: &mut Host,
        work: Work,
        label: &str,
        purpose: Purpose,
        hidden: Vec<usize>,
    ) {
        self.cancel(h);
        let cancel = Arc::new(AtomicBool::new(false));
        let id = self.next_id;
        self.next_id += 1;
        let job = Job {
            id,
            cancel: cancel.clone(),
            work,
        };
        if self.worker.as_ref().is_some_and(|w| w.submit(job)) {
            if matches!(purpose, Purpose::Translate) {
                unsafe {
                    history::begin_group(self.translated);
                }
            }
            self.pending = Some(Pending {
                id,
                cancel,
                started: Instant::now(),
                label: label.into(),
                purpose,
                hidden,
            });
            self.set_status(h, format!("{label}…"));
            unsafe {
                SetTimer(h.hwnd, 1, 200, None);
            }
        } else {
            platform::restore_owned_windows(&hidden);
            self.set_status(h, "上一个任务仍在结束，请稍后重试。");
        }
    }
    pub fn cancel(&mut self, h: &mut Host) {
        if let Some(p) = self.pending.take() {
            if matches!(p.purpose, Purpose::Translate) {
                unsafe {
                    history::end_group(self.translated);
                }
            }
            p.cancel.store(true, Ordering::Relaxed);
            platform::restore_owned_windows(&p.hidden);
        }
        unsafe {
            KillTimer(h.hwnd, 1);
        }
    }
    pub(super) fn cancel_editor_job(&mut self, h: &mut Host, editor: HWND) {
        let edits_active_job = self.pending.as_ref().is_some_and(|p| match p.purpose {
            Purpose::Translate => editor == self.source || editor == self.translated,
            Purpose::Ocr { .. } => editor == self.source,
            Purpose::Convert { target, .. } => editor as usize == target,
            _ => false,
        });
        if edits_active_job {
            self.cancel(h);
            self.set_status(h, "已保留手动编辑，取消本次结果更新。");
        }
    }
    pub fn load(&mut self, h: &mut Host, path: PathBuf) {
        self.begin(
            h,
            Work::Load(path),
            "读取图片",
            Purpose::Capture(self.settings.text_ocr_mode.clone()),
            Vec::new(),
        );
    }
    pub fn action(&mut self, h: &mut Host, action: &str) {
        match action {
            "show" => unsafe {
                show_main(h.hwnd);
            },
            "settings" => self.open_settings(h),
            "clipboard" => self.begin(
                h,
                Work::Clipboard,
                "读取当前剪贴板",
                Purpose::Clipboard,
                Vec::new(),
            ),
            "translate" => self.command(h, TRANSLATE, 0, null_mut()),
            "format" => self.command(h, FORMAT, 0, null_mut()),
            "hide-toolbar" => {
                if self.toolbar != 0 {
                    unsafe {
                        PostMessageW(self.toolbar as HWND, WM_CLOSE, 0, 0);
                    }
                }
                self.settings.floating_button = false;
            }
            "capture" | "window" | "fullscreen" | "table" | "vertical" | "formula" => {
                let mode = match action {
                    "table" => "表格",
                    "formula" => "公式",
                    "vertical" => "竖排",
                    _ => &self.settings.text_ocr_mode,
                }
                .to_owned();
                // The task is fixed at click time, independently of the text
                // selector; cancellation cannot leave a formula mode behind.
                let capture_label = format!("选择截图 · {}", self.settings.ocr_route(&mode));
                let hidden = platform::hide_owned_windows();
                self.begin(
                    h,
                    Work::Capture(
                        if action == "vertical" {
                            "capture"
                        } else {
                            action
                        }
                        .into(),
                    ),
                    &capture_label,
                    Purpose::Capture(mode),
                    hidden,
                );
            }
            _ => {}
        }
    }
    pub fn pump(&mut self, h: &mut Host) {
        let mut events = Vec::new();
        if let Some(rx) = &self.events {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }
        for event in events {
            match event {
                platform::Event::Action(a) => self.action(h, &a),
                platform::Event::Status(s) => self.set_status(h, s),
                platform::Event::Quit => unsafe {
                    PostMessageW(h.hwnd, EXIT, 0, 0);
                },
            }
        }
        let mut notices = Vec::new();
        if let Some(worker) = &self.worker {
            while let Ok(notice) = worker.rx.try_recv() {
                notices.push(notice);
            }
        }
        let partial = self
            .worker
            .as_ref()
            .and_then(|w| w.proxy.partial.lock().ok()?.take());
        if let Some((id, body)) = partial {
            if self.pending.as_ref().is_some_and(|p| p.id == id) {
                unsafe {
                    stream_text(self.translated, &body);
                }
            }
        }
        for notice in notices {
            match notice {
                Notice::Action(s) => self.action(h, &s),
                Notice::ImageOcr(image) => self.recognize_stored(h, image),
                Notice::PictureInfo(visible) => self.set_picture_info(h, visible),
                Notice::ChildClosed(hwnd) => {
                    self.children.retain(|&h| h != hwnd);
                    if self.settings_hwnd == hwnd {
                        self.settings_hwnd = 0;
                    }
                    if self.toolbar == hwnd {
                        self.toolbar = 0;
                        self.settings.floating_button = false;
                    }
                }
                Notice::Record(paused) => {
                    if let Some(d) = &self.desktop {
                        if let Err(e) = d.pause_hotkeys(paused) {
                            self.set_status(h, e);
                        }
                    }
                }
                Notice::Settings {
                    settings,
                    keys,
                    return_to_interfaces,
                } => self.apply_settings(h, *settings, *keys, return_to_interfaces),
                Notice::Apply(target, range, before, after) => unsafe {
                    if IsWindow(target as HWND) != 0 {
                        let current = win::range_text(target as HWND, range);
                        if current == before {
                            self.cancel_editor_job(h, target as HWND);
                            replace_preserving_view(target as HWND, range, &after);
                            self.refresh_document(h);
                            self.set_status(h, "已应用，可撤销。");
                        } else {
                            self.set_status(h, "预览期间原文已改变，请重新处理。");
                        }
                    }
                },
                Notice::Finished(id, result) => {
                    if self.pending.as_ref().is_none_or(|p| p.id != id) {
                        continue;
                    }
                    let pending = self.pending.take().unwrap();
                    unsafe {
                        KillTimer(h.hwnd, 1);
                    }
                    platform::restore_owned_windows(&pending.hidden);
                    let was_translation = matches!(pending.purpose, Purpose::Translate);
                    match result {
                        Err(e) => {
                            unsafe {
                                show_main(h.hwnd);
                            }
                            self.set_status(h, e);
                        }
                        Ok(Output::Ocr(result, processed)) => {
                            let formula =
                                matches!(pending.purpose, Purpose::Ocr { formula: true, .. });
                            let vertical =
                                matches!(pending.purpose, Purpose::Ocr { vertical: true, .. });
                            let elapsed = result.elapsed.as_secs_f32();
                            self.original = result.text.clone();
                            self.processing_base = result.text.clone();
                            self.processing_result = processed.clone();
                            self.layout = result.layout;
                            self.tables = result.tables;
                            self.table_view = TablePane::new();
                            unsafe {
                                set_text(self.source, &processed);
                                scintilla::table_layout(self.source, !self.tables.is_empty());
                                show_main(h.hwnd);
                            }
                            if self.settings.auto_copy && !processed.trim().is_empty() {
                                if let Some(table) = self.tables.first() {
                                    let _ = table_layout::copy_table(table);
                                } else {
                                    let _ = platform::copy_text(&processed);
                                }
                            }
                            self.set_status(
                                h,
                                if formula {
                                    format!("{} · 公式完成 · {elapsed:.2} s", pending.label)
                                } else {
                                    format!(
                                        "{} · {} 行 · {elapsed:.2} s · 公式请用公式截图",
                                        pending.label, result.lines
                                    )
                                },
                            );
                            if (formula || vertical) && !processed.trim().is_empty() {
                                let completed = self.status.clone();
                                self.open_rendered(h, self.source, false);
                                self.set_status(h, format!("{completed} · {}", self.status));
                            }
                            if !self.tables.is_empty() {
                                self.table_active = true;
                                if !self.settings.show_translation {
                                    self.toggle_translation(h, true);
                                }
                                self.layout(h);
                                self.set_status(h, "表格已在右栏显示 · 复制按钮可粘贴到 Word / Excel，拖选后 Ctrl+C 复制选中内容");
                            }
                            if self.settings.auto_translate
                                && self.settings.show_translation
                                && !formula
                                && !vertical
                                && self.tables.is_empty()
                            {
                                self.action(h, "translate");
                            }
                        }
                        Ok(Output::Text(body)) => match pending.purpose {
                            Purpose::Convert {
                                target,
                                range,
                                before,
                            } => unsafe {
                                if win::range_text(target as HWND, range) == before {
                                    let pasted = target == self.source as usize
                                        && range.start == 0
                                        && before == text(target as HWND)
                                        && scintilla::pasted_document(target as HWND);
                                    if pasted {
                                        let base = match history::paragraph_source(target as HWND) {
                                            Ok(base) => base.unwrap_or_else(|| before.clone()),
                                            Err(error) => {
                                                self.set_status(h, error);
                                                continue;
                                            }
                                        };
                                        if let Err(error) =
                                            history::begin_paragraph_change(target as HWND, &base)
                                        {
                                            self.set_status(h, error);
                                            continue;
                                        }
                                    }
                                    if before == self.processing_result {
                                        self.processing_result = body.clone();
                                    }
                                    replace_preserving_view(target as HWND, range, &body);
                                    if pasted {
                                        history::end_group(target as HWND);
                                    }
                                    self.refresh_document(h);
                                    self.set_status(
                                        h,
                                        format!("{}模式已应用；后续 OCR 自动沿用。", pending.label),
                                    );
                                }
                            },
                            _ => {
                                unsafe {
                                    set_text(self.translated, &body);
                                }
                                self.set_status(
                                    h,
                                    format!(
                                        "翻译完成 · {:.2} s",
                                        pending.started.elapsed().as_secs_f32()
                                    ),
                                );
                            }
                        },
                        Ok(Output::Clipboard(content)) => {
                            let result = match content {
                                backend::ClipboardSnapshot::Text(body) => self.note(h, body),
                                backend::ClipboardSnapshot::Image(image) => {
                                    self.picture_shared(h, image)
                                }
                            };
                            if let Err(e) = result {
                                self.set_status(h, e);
                            } else {
                                self.set_status(h, "已打开当前剪贴板");
                            }
                        }
                        Ok(Output::Image(image)) => {
                            if let Some(image) = image {
                                let mode = match pending.purpose {
                                    Purpose::Capture(mode) => mode,
                                    _ => self.settings.text_ocr_mode.clone(),
                                };
                                self.recognize_mode(h, image, &mode);
                            } else {
                                self.set_status(h, "已取消截图");
                            }
                        }
                    }
                    if was_translation {
                        unsafe {
                            history::end_group(self.translated);
                        }
                    }
                }
            }
        }
    }
}
