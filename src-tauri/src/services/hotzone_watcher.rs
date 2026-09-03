//! 感应区悬停探测。
//!
//! hotzone 窗口常驻屏幕顶部且置顶，若让它自己接收鼠标事件，就会遮住下层窗口
//! （浏览器标签栏、其他应用的标题栏按钮等）导致无法点击。因此该窗口对鼠标
//! **完全穿透**（`set_ignore_cursor_events(true)`），WebView 收不到
//! mouseenter / mouseleave；改由本线程按全局光标坐标判断是否落在感应区矩形内，
//! 仅在「进入 / 离开」状态翻转时通知 hotzone 窗口播放或停止感应动画，
//! 3 秒稳定悬停的计时与呼出面板仍由前端负责。

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::events;

/// 光标轮询间隔：80ms 足以让进入/离开的反馈看起来即时，又不至于占用可观 CPU。
const POLL_INTERVAL: Duration = Duration::from_millis(80);

/// 启动轮询线程（与剪贴板监听同样使用 std 线程 + sleep）。
pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("hotzone-watcher".into())
        .spawn(move || run(app))
        .expect("启动感应区轮询线程失败");
}

fn run(app: AppHandle) {
    let mut inside = false;
    loop {
        std::thread::sleep(POLL_INTERVAL);
        let now_inside = cursor_inside_hotzone(&app);
        if now_inside == inside {
            continue;
        }
        inside = now_inside;
        // 只发给 hotzone 窗口，其他窗口无需关心。
        if let Err(error) = app.emit_to("hotzone", events::HOTZONE_HOVER, inside) {
            eprintln!("[hotzone] 通知感应区悬停状态失败: {error}");
        }
    }
}

/// 光标是否位于感应区窗口矩形内。
///
/// 面板已展开时一律视为「不在区内」：避免面板打开期间光标停在顶部反复触发呼出，
/// 也保证面板收起后若光标仍停在感应区，会从「离开→进入」重新开始计时。
fn cursor_inside_hotzone(app: &AppHandle) -> bool {
    let panel_visible = app
        .get_webview_window("panel")
        .and_then(|panel| panel.is_visible().ok())
        .unwrap_or(false);
    if panel_visible {
        return false;
    }

    let Some(hotzone) = app.get_webview_window("hotzone") else {
        return false;
    };
    let Ok(cursor) = app.cursor_position() else {
        return false;
    };
    let (Ok(position), Ok(size)) = (hotzone.outer_position(), hotzone.outer_size()) else {
        return false;
    };

    // 全部按物理像素比较，与 cursor_position 的坐标系一致。
    let left = position.x as f64;
    let top = position.y as f64;
    let right = left + size.width as f64;
    let bottom = top + size.height as f64;
    cursor.x >= left && cursor.x < right && cursor.y >= top && cursor.y < bottom
}
