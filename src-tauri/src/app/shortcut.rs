//! 全局快捷键：默认 Ctrl/Cmd+Shift+Space 呼出面板、Alt+Space 呼出启动器；支持设置页改键。
//!
//! 两个快捷键互斥：不能设成同一个组合，否则一次按下无法区分意图。

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::app::state::AppState;
use crate::app::windows;

/// 面板快捷键的默认值。
const DEFAULT_PANEL: &str = "Ctrl+Shift+Space";
/// 启动器快捷键的默认值。
const DEFAULT_LAUNCHER: &str = "Alt+Space";

/// 规范化组合键描述，便于比较是否冲突。
fn normalize(combo: &str) -> Result<String, String> {
    combo
        .parse::<Shortcut>()
        .map(|s| s.into_string())
        .map_err(|e| format!("快捷键格式无效: {e}"))
}

/// 改绑面板快捷键；与启动器快捷键冲突时拒绝。写入 settings 后返回规范化描述。
pub fn rebind(app: &AppHandle, combo: &str) -> Result<String, String> {
    let described = normalize(combo)?;
    let state = app.state::<AppState>();
    let current = state.lock_store()?.get_settings()?;
    if normalize(current.launcher_shortcut()).ok().as_deref() == Some(described.as_str()) {
        return Err("与启动器快捷键冲突，请换一个".into());
    }
    let manager = app.global_shortcut();
    if let Ok(old) = current.shortcut().parse::<Shortcut>() {
        let _ = manager.unregister(old);
    }
    register_panel(app, &described)?;
    let mut settings = state.lock_store()?.get_settings()?;
    settings.set_shortcut(described.clone());
    state.lock_store()?.save_settings(&settings)?;
    Ok(described)
}

/// 改绑启动器快捷键；与面板快捷键冲突时拒绝。
pub fn rebind_launcher(app: &AppHandle, combo: &str) -> Result<String, String> {
    let described = normalize(combo)?;
    let state = app.state::<AppState>();
    let current = state.lock_store()?.get_settings()?;
    if normalize(current.shortcut()).ok().as_deref() == Some(described.as_str()) {
        return Err("与面板快捷键冲突，请换一个".into());
    }
    let manager = app.global_shortcut();
    if let Ok(old) = current.launcher_shortcut().parse::<Shortcut>() {
        let _ = manager.unregister(old);
    }
    register_launcher(app, &described)?;
    let mut settings = state.lock_store()?.get_settings()?;
    settings.set_launcher_shortcut(described.clone());
    state.lock_store()?.save_settings(&settings)?;
    Ok(described)
}

/// 注册面板快捷键：按下切换面板显隐。
fn register_panel(app: &AppHandle, combo: &str) -> Result<(), String> {
    let shortcut: Shortcut = combo.parse().map_err(|e| format!("快捷键格式无效: {e}"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                let visible = app
                    .get_webview_window("panel")
                    .map(|w| w.is_visible().unwrap_or(false))
                    .unwrap_or(false);
                if visible {
                    let _ = windows::panel_hide(app);
                } else {
                    let _ = windows::panel_show(app);
                }
            }
        })
        .map_err(|e| format!("注册面板快捷键失败（可能与其他应用冲突）: {e}"))
}

/// 注册启动器快捷键：按下切换启动器窗口显隐。
fn register_launcher(app: &AppHandle, combo: &str) -> Result<(), String> {
    let shortcut: Shortcut = combo.parse().map_err(|e| format!("快捷键格式无效: {e}"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                let visible = app
                    .get_webview_window("launcher")
                    .map(|w| w.is_visible().unwrap_or(false))
                    .unwrap_or(false);
                if visible {
                    let _ = windows::launcher_hide(app);
                } else {
                    let _ = windows::launcher_show(app);
                }
            }
        })
        .map_err(|e| format!("注册启动器快捷键失败（可能与其他应用冲突）: {e}"))
}

/// 启动时注册面板与启动器两个快捷键（读已保存值，失败回落默认值）。
pub fn register_startup(app: &AppHandle) -> Result<(), String> {
    let settings = app.state::<AppState>().lock_store()?.get_settings()?;
    let panel = normalize(settings.shortcut()).unwrap_or_else(|_| DEFAULT_PANEL.into());
    if let Err(error) = register_panel(app, &panel) {
        eprintln!("[shortcut] 注册面板快捷键失败: {error}");
    }
    let launcher =
        normalize(settings.launcher_shortcut()).unwrap_or_else(|_| DEFAULT_LAUNCHER.into());
    if let Err(error) = register_launcher(app, &launcher) {
        eprintln!("[shortcut] 注册启动器快捷键失败: {error}");
    }
    Ok(())
}
