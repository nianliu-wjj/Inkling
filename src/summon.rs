//! 呼出面板管理：开关窗口、全局快捷键（Ctrl+Shift+Space）、屏幕顶部中央触顶感应。
//!
//! 全局热键使用 `global-hotkey`；触顶感应使用 `device_query` 轮询鼠标位置。
//! 两者都在 GPUI 后台执行器的轮询循环中驱动。

use gpui::{
    px, App, AppContext, BorrowAppContext, Bounds, Point, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions,
};

use device_query::DeviceQuery;

use crate::panel::PanelApp;
use crate::settings::Settings;

/// 面板尺寸
const PANEL_W: f32 = 480.0;
const PANEL_H: f32 = 380.0;
/// 触顶感应：顶部感应带高度与水平半宽
const HOT_Y: f64 = 80.0;
const HOT_HALF_W: f64 = 120.0;
/// 触顶需要停留的时长
const HOVER_MS: u64 = 100;
/// 收起后多久内不再因触顶呼出
const REOPEN_COOLDOWN_MS: u64 = 600;

#[derive(Clone)]
struct PanelWindowGlobal {
    handle: Option<gpui::AnyWindowHandle>,
}

impl gpui::Global for PanelWindowGlobal {}

fn global(cx: &mut App) -> Option<PanelWindowGlobal> {
    cx.try_global::<PanelWindowGlobal>().cloned()
}

pub fn is_open(cx: &mut App) -> bool {
    global(cx).map(|g| g.handle.is_some()).unwrap_or(false)
}

/// 打开面板（顶部中央滑入）
pub fn show_panel(cx: &mut App) {
    if is_open(cx) {
        return;
    }
    let settings = Settings::load();
    let display_bounds = cx.primary_display().map(|d| d.bounds()).unwrap_or_else(|| {
        Bounds::new(
            Point::new(px(0.0), px(0.0)),
            Size::new(px(1920.), px(1080.)),
        )
    });
    let x = display_bounds.left() + (display_bounds.size.width - px(PANEL_W)) / 2.0;
    let y = display_bounds.top() + px(6.0);

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            Point::new(x, y),
            Size::new(px(PANEL_W), px(PANEL_H)),
        ))),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: false,
        window_background: WindowBackgroundAppearance::Blurred,
        app_id: Some("InklingPanel".into()),
        ..Default::default()
    };

    let handle = cx
        .open_window(options, |_, cx| cx.new(|cx| PanelApp::new(settings, cx)))
        .expect("打开呼出面板失败");

    handle
        .update(cx, |_, window, _| {
            window.activate_window();
        })
        .ok();

    cx.set_global(PanelWindowGlobal {
        handle: Some(handle.into()),
    });
}

/// 收起面板
pub fn close_panel(window: &mut Window, cx: &mut App) {
    if let Some(handle) = global(cx).and_then(|g| g.handle) {
        window.remove_window();
        let _ = handle;
    }
    if cx.has_global::<PanelWindowGlobal>() {
        cx.update_global::<PanelWindowGlobal, _>(|g, _cx| g.handle = None);
    }
}

pub fn toggle_panel(cx: &mut App) {
    if is_open(cx) {
        // 面板窗口通过 remove_window 关闭；此处直接按句柄移除
        if let Some(handle) = global(cx).and_then(|g| g.handle) {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
        if cx.has_global::<PanelWindowGlobal>() {
            cx.update_global::<PanelWindowGlobal, _>(|g, _cx| g.handle = None);
        }
    } else {
        show_panel(cx);
    }
}

/// 启动全局唤起监听（全局快捷键 + 触顶感应），在后台执行器中轮询。
pub fn init(cx: &mut App) {
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

    let manager = GlobalHotKeyManager::new().expect("初始化全局热键失败");
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
    manager.register(hotkey).expect("注册全局快捷键失败");
    // 保持 manager 存活（drop 后热键失效）
    std::mem::forget(manager);
    let receiver = GlobalHotKeyEvent::receiver();

    let screen_w = cx
        .primary_display()
        .map(|d| d.bounds().size.width.to_f64())
        .unwrap_or(1920.0);

    cx.spawn(async move |cx| {
        let watcher = device_query::DeviceState::new();
        let mut hover_since: Option<std::time::Instant> = None;
        let mut cooldown_until: Option<std::time::Instant> = None;
        let mut last_clipboard_text: Option<String> = None;
        loop {
            // ① 全局快捷键
            while let Ok(event) = receiver.try_recv() {
                if event.state == HotKeyState::Pressed {
                    cx.update(|cx| toggle_panel(cx)).ok();
                }
            }
            // ② 剪贴板捕获：仅在文本内容变化时写入，避免轮询重复产生数据库写入。
            let clipboard_text = cx
                .update(|cx| cx.read_from_clipboard().and_then(|item| item.text()))
                .ok()
                .flatten();
            if clipboard_text != last_clipboard_text {
                if let Some(text) = clipboard_text.clone() {
                    cx.update(|cx| crate::store::push_clip(cx, text)).ok();
                }
                last_clipboard_text = clipboard_text;
            }
            // ③ 触顶感应（鼠标移至屏幕顶部中央悬停 ≥ 100ms）
            let mouse = watcher.get_mouse();
            let (mx, my) = (mouse.coords.0 as f64, mouse.coords.1 as f64);
            let in_hotzone = my <= HOT_Y && (mx - screen_w / 2.0).abs() <= HOT_HALF_W;
            let now = std::time::Instant::now();
            if in_hotzone && !cx.update(|cx| is_open(cx)).unwrap_or(true) {
                hover_since = match hover_since {
                    None => Some(now),
                    Some(t) => Some(t),
                };
                if let Some(t) = hover_since {
                    let cooled = cooldown_until.map(|c| now > c).unwrap_or(true);
                    if (now - t).as_millis() as u64 >= HOVER_MS && cooled {
                        hover_since = None;
                        cooldown_until =
                            Some(now + std::time::Duration::from_millis(REOPEN_COOLDOWN_MS));
                        cx.update(|cx| show_panel(cx)).ok();
                    }
                }
            } else {
                hover_since = None;
            }
            cx.background_executor()
                .timer(std::time::Duration::from_millis(250))
                .await;
        }
    })
    .detach();
}
