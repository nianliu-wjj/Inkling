//! 系统托盘：左键打开主窗口，右键弹出 Inkling 主菜单。

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use super::windows;

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let history = MenuItem::with_id(app, "history", "📚 历史归档", true, None::<&str>)?;
    let stats = MenuItem::with_id(app, "stats", "📊 统计报表", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "⚙️ 偏好设置", true, None::<&str>)?;
    let quit = PredefinedMenuItem::quit(app, Some("⏻ 退出 Inkling"))?;
    let menu = Menu::with_items(app, &[&history, &stats, &settings, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default icon".into()))?;

    // macOS 分支需要重新赋值，非 macOS 下 mut 不被使用，显式放行告警。
    #[allow(unused_mut)]
    let mut builder = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("✒️ Inkling · 念头捕手")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "history" => {
                let _ = windows::show_main(app, "notes");
            }
            "stats" => {
                let _ = windows::show_main(app, "stats");
            }
            "settings" => {
                let _ = windows::show_main(app, "settings");
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle().clone();
                // 左键：主窗口可见则隐藏（toggle），否则打开默认笔记视图。
                let visible = app
                    .get_webview_window("main")
                    .map(|w| w.is_visible().unwrap_or(false))
                    .unwrap_or(false);
                if visible {
                    let _ = windows::hide_main(&app);
                } else {
                    let _ = windows::show_main(&app, "notes");
                }
            }
        });
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }
    builder.build(app)?;
    Ok(())
}
