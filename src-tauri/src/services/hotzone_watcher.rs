//! 感应区悬停探测。
//!
//! hotzone 窗口常驻屏幕边缘且置顶，若让它自己接收鼠标事件，就会遮住下层窗口
//! （浏览器标签栏、其他应用的标题栏按钮等）导致无法点击。因此该窗口对鼠标
//! **完全穿透**（`set_ignore_cursor_events(true)`），WebView 收不到
//! mouseenter / mouseleave；改由本线程按全局光标坐标判断是否落在感应区矩形内，
//! 仅在「进入 / 离开」状态翻转时通知对应 hotzone 窗口播放或停止感应动画，
//! 3 秒稳定悬停的计时与呼出面板仍由前端负责。
//!
//! 多显示器：每块屏幕一个感应区窗口（label 形如 hotzone-0），各自独立探测与记账，
//! 因此在任意一块屏幕顶部悬停都能唤出面板。
//!
//! 判定用的矩形来自 AppState 里建窗 / 重定位时记下的**物理像素**矩形，而不是回读窗口的
//! outer_position：混合 DPI 下 tao 回报的位置会在两种缩放间跳变，用它判定会闪烁。

use std::collections::HashMap;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::app::state::AppState;
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
    // 每个感应区窗口 label → 上一次是否在区内。
    let mut inside_map: HashMap<String, bool> = HashMap::new();
    // 显示器热插拔对账的节拍：每 25 轮（约 2 秒）检查一次拓扑是否变化。
    let mut tick: u32 = 0;
    loop {
        std::thread::sleep(POLL_INTERVAL);
        tick = tick.wrapping_add(1);
        if tick.is_multiple_of(25) {
            crate::app::windows::reconcile_hotzones(&app);
        }

        // 面板已展开时一律视为「都不在区内」：避免面板打开期间光标停在边缘反复触发呼出，
        // 也保证面板收起后若光标仍停在感应区，会从「离开→进入」重新开始计时。
        let panel_visible = app
            .get_webview_window("panel")
            .and_then(|panel| panel.is_visible().ok())
            .unwrap_or(false);
        let cursor = app.cursor_position().ok();

        for (label, rect) in app.state::<AppState>().hotzone_rects() {
            let now_inside = !panel_visible
                && cursor
                    .map(|c| point_in_rect(c.x, c.y, rect))
                    .unwrap_or(false);
            let prev = inside_map.get(&label).copied().unwrap_or(false);
            if now_inside == prev {
                continue;
            }
            inside_map.insert(label.clone(), now_inside);
            eprintln!(
                "[hotzone] {label} 状态翻转 inside={now_inside} cursor={:?}",
                cursor.map(|c| (c.x as i64, c.y as i64))
            );
            if let Err(error) = app.emit_to(label.clone(), events::HOTZONE_HOVER, now_inside) {
                eprintln!("[hotzone] 通知 {label} 悬停状态失败: {error}");
            }
        }
    }
}

/// 点是否落在矩形 (left, top, right, bottom) 内（物理像素，与 cursor_position 同坐标系）。
fn point_in_rect(x: f64, y: f64, rect: (f64, f64, f64, f64)) -> bool {
    let (left, top, right, bottom) = rect;
    x >= left && x < right && y >= top && y < bottom
}

#[cfg(test)]
mod tests {
    use super::point_in_rect;

    #[test]
    fn point_in_rect_is_half_open() {
        let rect = (10.0, 20.0, 30.0, 40.0);
        assert!(point_in_rect(10.0, 20.0, rect));
        assert!(point_in_rect(29.9, 39.9, rect));
        assert!(!point_in_rect(30.0, 20.0, rect));
        assert!(!point_in_rect(10.0, 40.0, rect));
        assert!(!point_in_rect(9.9, 25.0, rect));
    }
}
