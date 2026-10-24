use crate::{
    config::{Credentials, Settings, Translation},
    image_store::{ImageView, StoredImage},
    ocr::{self, BaiduClient, OcrResult},
    platform::{self, ClipboardContent},
    text, translate,
};
use image::RgbaImage;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Condvar, Mutex, Weak,
    },
    time::{Duration, Instant},
};

pub enum Notice {
    Finished(u64, Result<Output, String>),
    Action(String),
    Settings {
        settings: Box<Settings>,
        keys: Box<Credentials>,
        return_to_interfaces: bool,
    },
    ChildClosed(usize),
    Apply(usize, super::win::Range, String, String),
    Record(bool),
    ImageOcr(ImageView),
    PictureInfo(bool),
}
pub enum Output {
    Ocr(OcrResult, String),
    Text(String),
    Image(Option<RgbaImage>),
    Clipboard(ClipboardSnapshot),
}
pub enum ClipboardSnapshot {
    Text(String),
    Image(ImageView),
}
pub enum Work {
    Ocr(RgbaImage, Box<Settings>, Box<Credentials>),
    OcrStored(ImageView, Box<Settings>, Box<Credentials>),
    Translate(String, Box<Translation>, Box<Credentials>),
    Capture(String),
    Load(PathBuf),
    Clipboard,
    Convert(String, bool),
}
pub fn ocr_output(result: OcrResult, settings: &Settings) -> Result<Output, String> {
    // Preserve the original OCR for restore, and leave table/LaTeX intact.
    let processed = if matches!(settings.ocr_mode.as_str(), "公式" | "表格" | "竖排") {
        result.text.clone()
    } else {
        settings.processing.apply(
            &result.text,
            result.layout.as_deref(),
            &settings.format_options,
            settings.first_line_indent,
        )?
    };
    Ok(Output::Ocr(result, processed))
}
pub struct Job {
    pub id: u64,
    pub cancel: Arc<AtomicBool>,
    pub work: Work,
}
#[derive(Clone)]
pub struct Proxy {
    tx: mpsc::Sender<Notice>,
    pub notify: platform::Notify,
    pub partial: Arc<Mutex<Option<(u64, String)>>>,
    pub picture_menu: Arc<Mutex<String>>,
}
impl Proxy {
    pub fn send(&self, event: Notice) {
        if self.tx.send(event).is_ok() {
            self.notify.request_repaint();
        }
    }
    pub fn action(&self, s: &str) {
        self.send(Notice::Action(s.into()));
    }
    pub fn progress(&self, id: u64, s: String) {
        if let Ok(mut p) = self.partial.lock() {
            *p = Some((id, s));
        }
        self.notify.request_repaint();
    }
}
// One waiting task per lane. Replacing a waiting task releases its inputs immediately.
#[derive(Default)]
struct Mailbox {
    state: Mutex<(bool, Option<Job>)>,
    ready: Condvar,
}
impl Mailbox {
    fn recv(&self) -> Result<Job, mpsc::RecvTimeoutError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| mpsc::RecvTimeoutError::Disconnected)?;
        while !state.0 && state.1.is_none() {
            let (next, timeout) = self
                .ready
                .wait_timeout(state, Duration::from_secs(60))
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)?;
            state = next;
            if timeout.timed_out() && state.1.is_none() {
                return Err(mpsc::RecvTimeoutError::Timeout);
            }
        }
        if state.0 {
            Err(mpsc::RecvTimeoutError::Disconnected)
        } else {
            state.1.take().ok_or(mpsc::RecvTimeoutError::Disconnected)
        }
    }
    fn submit(&self, job: Job) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if state.0 {
            return false;
        }
        if let Some(old) = state.1.replace(job) {
            old.cancel.store(true, Ordering::Relaxed);
        }
        self.ready.notify_one();
        true
    }
}
#[derive(Default)]
struct ClipboardReader {
    latest: Option<(u32, Weak<StoredImage>, Option<usize>)>,
}
impl ClipboardReader {
    fn read(&mut self) -> Result<ClipboardSnapshot, String> {
        use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
        self.read_with(
            || unsafe { GetClipboardSequenceNumber() },
            platform::read_clipboard,
        )
    }
    fn read_with(
        &mut self,
        sequence_number: impl Fn() -> u32,
        read: impl FnOnce() -> Result<ClipboardContent, String>,
    ) -> Result<ClipboardSnapshot, String> {
        let sequence = sequence_number();
        if sequence != 0 {
            if let Some((cached_sequence, cached, png_bytes)) = &self.latest {
                if *cached_sequence == sequence {
                    if let Some(image) = cached.upgrade() {
                        let mut view = ImageView::new(image);
                        view.png_bytes = *png_bytes;
                        return Ok(ClipboardSnapshot::Image(view));
                    }
                }
            }
        }
        self.latest = None;
        // Reads only the current clipboard, never Windows clipboard history.
        let content = read()?;
        let after = sequence_number();
        match content {
            ClipboardContent::Text(text) => Ok(ClipboardSnapshot::Text(text)),
            ClipboardContent::Image(image, png_bytes) => {
                let original = StoredImage::intern(image)?;
                if sequence != 0 && after == sequence {
                    self.latest = Some((sequence, Arc::downgrade(&original), png_bytes));
                }
                let mut view = ImageView::new(original);
                view.png_bytes = png_bytes;
                Ok(ClipboardSnapshot::Image(view))
            }
        }
    }
}
pub struct Worker {
    lanes: [Arc<Mailbox>; 3],
    pub rx: mpsc::Receiver<Notice>,
    pub proxy: Proxy,
}
impl Drop for Worker {
    fn drop(&mut self) {
        for lane in &self.lanes {
            if let Ok(mut state) = lane.state.lock() {
                state.0 = true;
                state.1 = None;
                lane.ready.notify_all();
            }
        }
    }
}
impl Worker {
    pub fn submit(&self, job: Job) -> bool {
        let lane = match &job.work {
            Work::Translate(..) => 2,
            Work::Ocr(..) | Work::OcrStored(..) => 1,
            _ => 0,
        };
        self.lanes[lane].submit(job)
    }
    pub fn new(notify: platform::Notify) -> Self {
        let (tx, rx) = mpsc::channel();
        let proxy = Proxy {
            tx,
            notify,
            partial: Arc::new(Mutex::new(None)),
            picture_menu: Arc::new(Mutex::new(String::new())),
        };
        let lanes: [Arc<Mailbox>; 3] = std::array::from_fn(|_| Arc::new(Mailbox::default()));
        // Fixed local/OCR/translation lanes: network timeouts cannot block clipboard reads.
        for lane in &lanes {
            let lane = lane.clone();
            let output = proxy.clone();
            std::thread::spawn(move || {
                let mut baidu = BaiduClient::default();
                let mut last_ocr = Instant::now();
                let mut clipboard = ClipboardReader::default();
                loop {
                    let job = match lane.recv() {
                        Ok(job) => job,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            baidu.release_idle_http();
                            continue;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    if job.cancel.load(Ordering::Relaxed) {
                        continue;
                    }
                    if last_ocr.elapsed() >= Duration::from_secs(60) {
                        baidu.release_idle_http();
                    }
                    let result = (|| match job.work {
                        Work::Ocr(image, settings, keys) => {
                            last_ocr = Instant::now();
                            ocr::recognize(&mut baidu, image, &settings, &keys)
                                .and_then(|result| ocr_output(result, &settings))
                        }
                        Work::OcrStored(image, settings, keys) => {
                            last_ocr = Instant::now();
                            ocr::recognize(&mut baidu, image.decode()?, &settings, &keys)
                                .and_then(|result| ocr_output(result, &settings))
                        }
                        Work::Translate(body, settings, keys) => {
                            translate::translate(&body, &settings, &keys, job.cancel.clone(), |s| {
                                output.progress(job.id, s);
                            })
                            .map(Output::Text)
                        }
                        Work::Capture(kind) => {
                            std::thread::sleep(Duration::from_millis(90));
                            let (image, x, y) = platform::capture_desktop()?;
                            let image = match kind.as_str() {
                                "fullscreen" => Some(image),
                                "window" => platform::select_window(image, x, y)?,
                                _ => platform::select_region(image, x, y)?,
                            };
                            Ok(Output::Image(image))
                        }
                        Work::Clipboard => clipboard.read().map(Output::Clipboard),
                        Work::Load(path) => {
                            let reader = image::ImageReader::open(&path)
                                .map_err(|_| "无法打开图片文件。")?
                                .with_guessed_format()
                                .map_err(|_| "无法判断图片格式。")?;
                            let (w, h) =
                                reader.into_dimensions().map_err(|_| "无法读取图片尺寸。")?;
                            if u64::from(w) * u64::from(h) > 32_000_000 {
                                return Err("图片超过 3200 万像素，请先裁剪。".into());
                            }
                            let image = image::ImageReader::open(path)
                                .map_err(|_| "无法打开图片文件。")?
                                .with_guessed_format()
                                .map_err(|_| "无法判断图片格式。")?
                                .decode()
                                .map_err(|_| "图片解码失败。")?
                                .into_rgba8();
                            Ok(Output::Image(Some(image)))
                        }
                        Work::Convert(body, traditional) => {
                            text::convert_chinese(&body, traditional).map(Output::Text)
                        }
                    })();
                    if !job.cancel.load(Ordering::Relaxed) {
                        output.send(Notice::Finished(job.id, result));
                    }
                }
            });
        }
        Self { lanes, rx, proxy }
    }
}
