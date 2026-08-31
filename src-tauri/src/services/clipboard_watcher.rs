//! 剪贴板监听：arboard 每 500ms 轮询、SHA-256 去重、回声抑制、图片附件落盘。
//! 每 24h 按保留天数清理过期条目。

use arboard::{Clipboard, ImageData};
use std::time::Duration;

use crate::app::state::AppState;
use crate::domain::clipboard as logic;
use crate::domain::models::ClipboardEntry;
use crate::events;
use tauri::{AppHandle, Emitter, Manager};

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_INLINE_TEXT: usize = 512 * 1024;
const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

/// 启动轮询线程（std 线程 + 交叉 sleep，避免引入 tokio runtime 复杂度）。
pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("clipboard-watcher".into())
        .spawn(move || run(app))
        .expect("启动剪贴板轮询线程失败");
}

fn run(app: AppHandle) {
    let mut board = match Clipboard::new() {
        Ok(board) => board,
        Err(error) => {
            eprintln!("[clipboard] 初始化失败: {error}");
            return;
        }
    };
    let mut last_hash: Option<String> = None;
    let mut since_cleanup = 0u64;
    loop {
        std::thread::sleep(POLL_INTERVAL);
        since_cleanup += POLL_INTERVAL.as_secs();
        if let Some(entry) = poll_once(&app, &mut board, &mut last_hash) {
            let _ = app.emit(events::CLIPBOARD_CHANGED, entry);
        }
        if since_cleanup >= CLEANUP_INTERVAL.as_secs() {
            since_cleanup = 0;
            let _ = cleanup(&app);
        }
    }
}

/// 单次轮询：文本优先，其次图片。返回新入库的条目（无新增返回 None）。
fn poll_once(
    app: &AppHandle,
    board: &mut Clipboard,
    last_hash: &mut Option<String>,
) -> Option<ClipboardEntry> {
    // 回声抑制：跳过应用自身写回的内容。
    if let Some(echo) = app.state::<AppState>().take_echo() {
        if last_hash.as_deref() == Some(echo.as_str()) {
            return None;
        }
    }
    let text = board.get_text().ok();
    if let Some(text) = text {
        if text.is_empty() {
            return None;
        }
        let hash = logic::hash_content(text.as_bytes());
        if last_hash.as_deref() == Some(hash.as_str()) {
            return None;
        }
        *last_hash = Some(hash.clone());
        return capture_text(app, text, hash, false);
    }
    if let Ok(image) = board.get_image() {
        let hash = image_hash(&image);
        if last_hash.as_deref() == Some(hash.as_str()) {
            return None;
        }
        *last_hash = Some(hash.clone());
        return capture_image(app, image, hash);
    }
    None
}

/// 文本捕获（手动「捕获当前剪贴板」复用；echo=true 表示应用自身写回，跳过入库）。
pub fn capture_text(
    app: &AppHandle,
    text: String,
    hash: String,
    echo: bool,
) -> Option<ClipboardEntry> {
    if echo {
        app.state::<AppState>().set_echo(Some(hash));
        return None;
    }
    let kind = logic::classify_text(&text, false);
    let state = app.state::<AppState>();
    let store = state.lock_store().ok()?;
    let capture = logic::build_preview(&text, 240);
    let input = crate::data::clipboard::Capture {
        content: text.chars().take(MAX_INLINE_TEXT).collect(),
        content_type: kind.as_str(),
        preview: capture,
        file_path: None,
        hash,
    };
    store.insert_capture(&input).ok().flatten()
}

/// 图片捕获：PNG 落盘 `clipboard/`，库内仅存路径与预览。
fn capture_image(app: &AppHandle, image: ImageData<'_>, hash: String) -> Option<ClipboardEntry> {
    if image.bytes.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let width = image.width as u32;
    let height = image.height as u32;
    let state = app.state::<AppState>();
    let store = state.lock_store().ok()?;
    let data_dir = store.data_dir.clone();
    let id = uuid::Uuid::new_v4().to_string();
    let relative = format!("clipboard/{id}.png");
    let png = encode_png(&image.bytes, width, height)?;
    std::fs::write(data_dir.join(&relative), png).ok()?;
    let preview = format!("[图片 {width}×{height}]");
    let input = crate::data::clipboard::Capture {
        content: String::new(),
        content_type: "image",
        preview,
        file_path: Some(relative),
        hash,
    };
    store.insert_capture(&input).ok().flatten()
}

fn encode_png(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let buffer = image::RgbaImage::from_raw(width, height, rgba.to_vec())?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(buffer)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(png)
}

fn image_hash(image: &ImageData<'_>) -> String {
    let mut seed = format!("image:{}x{}:", image.width, image.height).into_bytes();
    seed.extend_from_slice(&image.bytes);
    logic::hash_content(&seed)
}

/// 按保留天数清理（读取设置；默认 30 天）。
pub fn cleanup(app: &AppHandle) -> Result<usize, String> {
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    let days = store
        .setting_value("clipboard_retention_days")?
        .and_then(|x| x.parse::<i64>().ok())
        .unwrap_or(30);
    let removed = store.cleanup_clipboard(days)?;
    let _ = app.emit(events::CLIPBOARD_CHANGED, ());
    Ok(removed)
}
