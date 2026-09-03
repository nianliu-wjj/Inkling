//! Inkling（念头捕手）Tauri 2 主入口。
//!
//! 架构：app（窗口/托盘/快捷键/状态）、domain（纯业务）、data（SQLite/文件）、
//! services（剪贴板轮询/提醒调度/导出）、ipc（命令层）。
//! 铁律：渲染进程不直接碰 SQL，Rust 不直接碰 DOM。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod data;
mod domain;
mod events;
mod ipc;
mod platform;
mod services;

use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

fn main() {
    let silent = platform::is_silent_start();
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--silent"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app_handle| {
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("获取应用数据目录失败: {e}"))?;
            let store = data::Store::open(data_dir).map_err(std::io::Error::other)?;
            let settings = store.get_settings().map_err(std::io::Error::other)?;
            app_handle.manage(app::state::AppState::with_store(store));
            let app = app_handle.handle().clone();

            app::windows::create_core_windows(&app, silent).map_err(std::io::Error::other)?;
            app::tray::build_tray(&app).map_err(std::io::Error::other)?;
            app::shortcut::register_startup(&app).map_err(std::io::Error::other)?;
            if settings.start_on_boot {
                let _ = app.autolaunch().enable();
            }

            services::clipboard_watcher::start(app.clone());
            services::reminder::start(app.clone());
            services::hotzone_watcher::start(app.clone());
            // 启动时按保留策略清理一次过期剪贴板。
            let handle_for_cleanup = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let _ = services::clipboard_watcher::cleanup(&handle_for_cleanup);
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // main 关闭按钮 → 隐藏保持托盘常驻。
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            // 编辑窗口销毁 → 广播，面板据此恢复失焦收起计时。
            // 放在窗口事件而非 editor_close 命令里，才能覆盖 Alt+F4 等前端未参与的关闭路径。
            if window.label() == "editor" {
                if let tauri::WindowEvent::Destroyed = event {
                    let _ = window.app_handle().emit(events::EDITOR_CLOSED, ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            ipc::panel_show,
            ipc::panel_hide,
            ipc::panel_resize,
            ipc::editor_open,
            ipc::editor_close,
            ipc::editor_payload,
            ipc::editor_ready,
            ipc::show_main,
            ipc::hide_main,
            ipc::quit_app,
            ipc::set_main_acrylic,
            ipc::pin_create,
            ipc::pin_close,
            ipc::pin_set_editing,
            ipc::reminder_close,
            ipc::rebind_shortcut,
            ipc::notes_list,
            ipc::note_draft,
            ipc::note_save,
            ipc::note_delete,
            ipc::note_set_pinned,
            ipc::clipboard_list,
            ipc::clipboard_capture,
            ipc::clipboard_write,
            ipc::clipboard_paste,
            ipc::clipboard_update,
            ipc::clipboard_pin,
            ipc::clipboard_delete,
            ipc::clipboard_cleanup,
            ipc::todos_list,
            ipc::todo_save,
            ipc::todo_complete,
            ipc::todo_priority,
            ipc::todo_due,
            ipc::todo_reminder,
            ipc::todo_delete,
            ipc::todo_snooze,
            ipc::todo_dismiss_reminder,
            ipc::settings_get,
            ipc::settings_save,
            ipc::stats_heatmap,
            ipc::stats_trend,
            ipc::stats_summary,
            ipc::stats_day,
            ipc::export_items,
            ipc::data_dir
        ])
        .run(tauri::generate_context!())
        .expect("启动 Inkling 失败");
}
