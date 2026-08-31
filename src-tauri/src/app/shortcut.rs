//! 全局快捷键：默认 Ctrl/Cmd+Shift+Space 呼出面板；支持设置页改键。

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::app::state::AppState;
use crate::app::windows;

/// 解析并注册快捷键；注册成功后写入 settings。返回规范化后的快捷键描述。
pub fn rebind(app: &AppHandle, combo: &str) -> Result<String, String> {
    let shortcut: Shortcut = combo
        .parse::<Shortcut>()
        .map_err(|e| format!("快捷键格式无效: {e}"))?;
    let manager = app.global_shortcut();
    let state = app.state::<AppState>();
    let previous = state
        .lock_store()?
        .get_settings()
        .map(|s| s.shortcut)
        .unwrap_or_default();
    if let Ok(old) = previous.parse::<Shortcut>() {
        let _ = manager.unregister(old);
    }
    manager
        .register(shortcut)
        .map_err(|e| format!("注册快捷键失败（可能与其他应用冲突）: {e}"))?;
    let described = shortcut.into_string();
    state
        .lock_store()?
        .save_settings(&crate::domain::models::Settings {
            shortcut: described.clone(),
            ..state.lock_store()?.get_settings()?
        })?;
    Ok(described)
}

/// 启动时注册默认/已保存的快捷键。
pub fn register_startup(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let combo = state
        .lock_store()?
        .get_settings()
        .map(|s| s.shortcut)
        .unwrap_or_else(|_| "Ctrl+Shift+Space".into());
    let shortcut: Shortcut = combo
        .parse()
        .unwrap_or_else(|_| "CmdOrCtrl+Shift+Space".parse().expect("内置快捷键必须有效"));
    let handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                let panel = app.get_webview_window("panel");
                let visible = panel
                    .map(|w| w.is_visible().unwrap_or(false))
                    .unwrap_or(false);
                if visible {
                    let _ = windows::panel_hide(app);
                } else {
                    let _ = windows::panel_show(app);
                }
            }
        })
        .map_err(|e| format!("注册全局快捷键失败: {e}"))?;
    let _ = handle;
    Ok(())
}
