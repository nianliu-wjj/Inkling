//! 窗口系统：hotzone / panel / main / pinned / reminder 的创建、定位与状态切换。
//!
//! 定位约定：以鼠标所在显示器为基准（多屏），统一在物理像素上换算逻辑尺寸；
//! panel 贴屏幕顶部居中，pinned 右下角级联，reminder 右上角纵向堆叠。

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder,
};

use super::state::AppState;
use crate::events;

pub const PANEL_WIDTH: f64 = 480.0;
pub const PANEL_MIN_HEIGHT: f64 = 120.0;
pub const PANEL_MAX_HEIGHT: f64 = 600.0;
const HOTZONE_WIDTH: f64 = 240.0;
const HOTZONE_HEIGHT: f64 = 80.0;
const PINNED_SIZE: (f64, f64) = (230.0, 150.0);
const REMINDER_SIZE: (f64, f64) = (320.0, 208.0);

/// 光标所在显示器（回退主屏）。
pub fn cursor_monitor(app: &AppHandle) -> Option<tauri::Monitor> {
    let cursor: PhysicalPosition<f64> = app.cursor_position().ok()?;
    let ix = cursor.x as i64;
    let iy = cursor.y as i64;
    app.available_monitors()
        .ok()?
        .into_iter()
        .find(|m| {
            let p = m.position();
            let s = m.size();
            let left = p.x as i64;
            let top = p.y as i64;
            let right = left + s.width as i64;
            let bottom = top + s.height as i64;
            ix >= left && ix < right && iy >= top && iy < bottom
        })
        .or_else(|| app.primary_monitor().ok().flatten())
}

/// 启动时创建核心窗口：hotzone 常驻、panel 常驻隐藏（main 由配置创建）。
pub fn create_core_windows(app: &AppHandle, silent: bool) -> tauri::Result<()> {
    let monitor = cursor_monitor(app)
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| tauri::Error::WindowNotFound)?;

    // hotzone：透明感应区，不可聚焦、不抢焦点。
    let (hx, hy) = top_center(&monitor, HOTZONE_WIDTH, 0.0);
    let hotzone = WebviewWindowBuilder::new(app, "hotzone", WebviewUrl::App("hotzone.html".into()))
        .title("Inkling Hotzone")
        .inner_size(HOTZONE_WIDTH, HOTZONE_HEIGHT)
        .position(hx, hy)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .resizable(false)
        .shadow(false)
        .build()?;
    // Windows 上部分环境不会可靠继承 builder 的 skip_taskbar 配置，创建后再次显式设置。
    let _ = hotzone.set_skip_taskbar(true);
    let _ = hotzone.set_ignore_cursor_events(false);

    // panel：预创建常驻隐藏，呼出只做 show + focus。
    let (px, py) = top_center(&monitor, PANEL_WIDTH, 6.0);
    let panel = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
        .title("Inkling Panel")
        .inner_size(PANEL_WIDTH, PANEL_MAX_HEIGHT)
        .position(px, py)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .build()?;
    let _ = panel.set_skip_taskbar(true);
    crate::platform::apply_panel_effects(&panel);

    // main：配置中 visible=false，这里按启动模式决定是否展示。
    if let Some(main) = app.get_webview_window("main") {
        // 主窗口是唯一允许出现在任务栏中的窗口。
        let _ = main.set_skip_taskbar(false);
        crate::platform::apply_main_backdrop(&main);
        if !silent {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    Ok(())
}

/// 计算宽度 `width` 的窗口在某显示器顶部居中的逻辑坐标。
fn top_center(monitor: &tauri::Monitor, width: f64, top_offset: f64) -> (f64, f64) {
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let x_logical = work.position.x as f64 / scale + (work.size.width as f64 / scale - width) / 2.0;
    let y_logical = work.position.y as f64 / scale + top_offset;
    (x_logical, y_logical)
}

/// 屏幕顶部居中 + 竖直方向偏移。
fn top_center_offset(monitor: &tauri::Monitor, width: f64, offset: f64) -> (f64, f64) {
    top_center(monitor, width, offset)
}

/// 呼出面板：定位到光标所在屏顶部居中，show + focus，并屏蔽感应区防止误触。
/// 对指定窗口应用或撤销毛玻璃效果。
///
/// Windows 走 Acrylic，macOS 走 Vibrancy（HudWindow），Linux 无对应实现直接跳过。
/// 注意：效果能否可见取决于窗口创建时是否设置了 `transparent(true)`——
/// 该属性是创建期属性，运行时不可更改；而这里的 apply/clear 是运行时可调的，
/// 这正是「毛玻璃开关」无需销毁重建窗口即可即时生效的原因。
pub fn apply_window_effect(window: &tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_acrylic, clear_acrylic};
        let result = if enabled {
            apply_acrylic(window, Some((18, 18, 24, 125)))
        } else {
            clear_acrylic(window)
        };
        // 不支持的系统版本只记录，不阻断——前端仍会切换 data-acrylic 降级配色。
        if let Err(error) = result {
            eprintln!("[windows] 应用毛玻璃失败（将降级为实色）：{error}");
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
        if enabled {
            let _ = apply_vibrancy(window, NSVisualEffectMaterial::HudWindow, None, None);
        }
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (window, enabled);
        Ok(())
    }
}

/// 切换归档主窗口的毛玻璃（偏好设置项）。
pub fn set_main_acrylic(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不存在")?;
    apply_window_effect(&window, enabled)
}

pub fn panel_show(app: &AppHandle) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let (x, y) = top_center(&monitor, PANEL_WIDTH, 6.0);
    let _ = panel.set_position(LogicalPosition::new(x, y));
    let _ = panel.show();
    let _ = panel.unminimize();
    let _ = panel.set_focus();
    if let Some(hotzone) = app.get_webview_window("hotzone") {
        let _ = hotzone.set_ignore_cursor_events(true);
    }
    let _ = app.emit(events::PANEL_SHOWN, ());
    Ok(())
}

/// 收起面板并恢复感应区。
pub fn panel_hide(app: &AppHandle) -> Result<(), String> {
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.hide();
    }
    if let Some(hotzone) = app.get_webview_window("hotzone") {
        let _ = hotzone.set_ignore_cursor_events(false);
    }
    let _ = app.emit(events::PANEL_HIDDEN, ());
    Ok(())
}

/// 面板高度自适应（前端测量内容后调用）。
pub fn panel_resize(app: &AppHandle, height: f64) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    let clamped = height.clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT);
    let _ = panel.set_size(LogicalSize::new(PANEL_WIDTH, clamped));
    Ok(())
}

/// 显示主窗口并导航到指定视图。
pub fn show_main(app: &AppHandle, view: &str) -> Result<(), String> {
    let main = app.get_webview_window("main").ok_or("主窗口未初始化")?;
    let _ = main.show();
    let _ = main.unminimize();
    let _ = main.set_focus();
    let _ = app.emit(events::NAVIGATE, view.to_string());
    Ok(())
}

/// 隐藏主窗口（保持托盘常驻）。
pub fn hide_main(app: &AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    Ok(())
}

fn pinned_count(app: &AppHandle) -> usize {
    app.webview_windows()
        .keys()
        .filter(|l| l.starts_with("pinned-"))
        .count()
}

/// 创建（或唤起）桌面置顶浮窗。
pub fn pin_create(app: &AppHandle, kind: &str, id: &str) -> Result<(), String> {
    let label = format!("pinned-{kind}-{id}");
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let index = pinned_count(app);
    let right = (work.position.x + work.size.width as i32) as f64 / scale
        - PINNED_SIZE.0
        - 16.0
        - (index % 4) as f64 * (PINNED_SIZE.0 + 14.0);
    let bottom = (work.position.y + work.size.height as i32) as f64 / scale
        - PINNED_SIZE.1
        - 16.0
        - (index / 4) as f64 * (PINNED_SIZE.1 + 14.0);

    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("pinned.html".into()))
        .title("Inkling Pin")
        .inner_size(PINNED_SIZE.0, PINNED_SIZE.1)
        .min_inner_size(180.0, 110.0)
        .position(right.max(work.position.x as f64 / scale), bottom.max(0.0))
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .shadow(false)
        .visible(false)
        .build()
        .map_err(|e| format!("创建置顶浮窗失败: {e}"))?;
    let _ = window.set_skip_taskbar(true);
    let _ = window.show();
    Ok(())
}

/// 关闭置顶浮窗。
pub fn pin_close(app: &AppHandle, label: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.close();
    }
    Ok(())
}

/// 置顶浮窗编辑态：展开为可编辑尺寸。
pub fn pin_set_editing(app: &AppHandle, label: &str, expanded: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        let size = if expanded {
            (300.0, 320.0)
        } else {
            PINNED_SIZE
        };
        let _ = window.set_size(LogicalSize::new(size.0, size.1));
    }
    Ok(())
}

fn reminder_count(app: &AppHandle) -> usize {
    app.webview_windows()
        .keys()
        .filter(|l| l.starts_with("reminder-"))
        .count()
}

/// 弹出右上角提醒卡片（同一待办复用既有窗口）。
pub fn reminder_show(app: &AppHandle, todo_id: &str) -> Result<(), String> {
    let label = format!("reminder-{todo_id}");
    if let Some(window) = app.get_webview_window(&label) {
        let _ = app.emit_to(label.clone(), events::REMINDER_FIRED, todo_id.to_string());
        let _ = window.show();
        return Ok(());
    }
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let index = reminder_count(app);
    let x = (work.position.x + work.size.width as i32) as f64 / scale - REMINDER_SIZE.0 - 20.0;
    let y = work.position.y as f64 / scale + 20.0 + index as f64 * (REMINDER_SIZE.1 + 12.0);

    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("reminder.html".into()))
        .title("Inkling 提醒")
        .inner_size(REMINDER_SIZE.0, REMINDER_SIZE.1)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| format!("创建提醒窗口失败: {e}"))?;
    let _ = window.set_skip_taskbar(true);
    let _ = window.show();
    let _ = app.emit_to(label, events::REMINDER_FIRED, todo_id.to_string());
    Ok(())
}

/// 关闭提醒卡片。
pub fn reminder_close(app: &AppHandle, todo_id: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(format!("reminder-{todo_id}").as_str()) {
        let _ = window.close();
    }
    Ok(())
}

/// 应用退出。
pub fn quit_app(app: &AppHandle) {
    app.exit(0);
}

/// 供 ipc 调用的顶层重导出（避免 ipc 直接依赖内部定位细节）。
pub fn offset_top_center(monitor: &tauri::Monitor, width: f64, offset: f64) -> (f64, f64) {
    top_center_offset(monitor, width, offset)
}

/// 状态存取快捷入口。
pub fn state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}
