//! Inkling（念头捕手）桌面端入口 — 基于 GPUI。
//!
//! 基础功能阶段：单窗口（标题栏 / 侧边栏 / 三视图 / 设置与统计入口 / 多主题）。

mod app;
mod macros;
mod panel;
mod pin;
mod reminder;
mod settings;
mod stats;
mod store;
mod summon;
mod text_input;
mod tray;
mod theme;
mod views;

use app::{key_bindings, open_main_window};
use gpui::{App, Application};

fn main() {
    Application::new().run(|cx: &mut App| {
        let settings = settings::Settings::load();
        let silent_autostart = std::env::args().any(|arg| arg == "--autostart");
        store::init(cx);
        store::apply_clip_retention(cx, settings.clip_retention());
        cx.bind_keys(key_bindings());
        cx.bind_keys(text_input::key_bindings());
        summon::init(cx);
        reminder::init(cx);
        tray::init(cx);

        // 通过 Windows Run 注册表启动时只驻留后台，不抢占焦点，也不自动打开主窗口。
        // 用户仍可通过全局快捷键、触顶感应、托盘或提醒窗口唤起需要的界面。
        if !silent_autostart {
            open_main_window(cx, settings, app::ActiveView::Notes);
        }
    });
}

