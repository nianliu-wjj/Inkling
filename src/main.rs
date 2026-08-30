//! Inkling（念头捕手）桌面端入口 — 基于 GPUI。
//!
//! 基础功能阶段：单窗口（标题栏 / 侧边栏 / 三视图 / 设置与统计入口 / 多主题）。

mod app;
mod macros;
mod settings;
mod stats;
mod theme;
mod views;

use app::{key_bindings, InboxApp};
use gpui::{
    px, size, App, AppContext, Application, Bounds, Focusable, TitlebarOptions, WindowBounds,
    WindowOptions,
};

fn main() {
    Application::new().run(|cx: &mut App| {
        let settings = settings::Settings::load();
        cx.bind_keys(key_bindings());

        let bounds = Bounds::centered(None, size(px(880.), px(680.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Inkling".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| InboxApp::new(settings, cx)),
            )
            .expect("打开主窗口失败");

        window
            .update(cx, |view, window, _cx| {
                window.focus(&view.focus_handle(_cx));
            })
            .ok();

        cx.activate(true);
    });
}
