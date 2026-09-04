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

    // hotzone：透明感应区，不可聚焦、不抢焦点，且对鼠标完全穿透。
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
    // 感应区置顶盖在屏幕顶部中央，若接收鼠标事件会挡住下层窗口（浏览器标签栏、
    // 其他应用的标题栏按钮）导致无法点击。因此永久穿透，悬停探测改由
    // services::hotzone_watcher 按全局光标坐标完成。
    let _ = hotzone.set_ignore_cursor_events(true);

    // panel：预创建常驻隐藏，呼出只做 show + focus。
    // 一次性读出建窗需要的偏好设置，避免为每项各锁一次 store。
    let (position, main_acrylic) = app
        .state::<AppState>()
        .lock_store()
        .ok()
        .and_then(|store| store.get_settings().ok())
        .map(|settings| (settings.panel_position, settings.main_acrylic))
        .unwrap_or_else(|| ("top".into(), true));
    let (px, py) = panel_position(&monitor, PANEL_WIDTH, PANEL_MAX_HEIGHT, &position);
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
        // 按偏好设置应用毛玻璃：窗口以 transparent 创建，若不显式应用效果层，
        // 主窗口会是「透明但无背景」，直接透出桌面内容。
        crate::platform::apply_main_backdrop(&main, main_acrylic);
        // 主窗口同样是无边框窗口，默认直角；圆角与毛玻璃开关无关，独立设置。
        crate::platform::apply_rounded_corners(&main);
        if !silent {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    Ok(())
}

/// 打开编辑窗口：铺满光标所在显示器工作区的透明窗口，遮罩压暗 + 对话框居中。
///
/// **每次打开都重建窗口**，而不是复用一个常驻隐藏窗口：WebView2 在窗口 hide 后会被
/// 挂起，Tauri 靠 eval 投递的事件（含 tauri://focus）此时全部丢失，复用窗口时第二次
/// 打开只会显示一个空的全屏透明窗口——它还会吞掉整屏点击。新建窗口的 WebView 必然
/// 执行一次挂载逻辑，前端在那里主动拉取参数，时序上确定。
///
/// 参数经 `AppState` 暂存而不是拼进 URL：`WebviewUrl::App` 收的是相对路径，
/// 查询串里的 `?` 会被当作路径字符转义掉，前端读不到。
///
/// 窗口**创建即可见但先对鼠标穿透**，等前端拿到参数、渲染出对话框后再调用
/// `editor_ready` 接管鼠标与焦点。不能用 visible(false) 创建后等前端就绪再 show：
/// WebView2 对隐藏窗口不做初始化，前端永远不会挂载，等待就成了死锁。
/// 先穿透也顺带消掉一个隐患——内容未就绪的全屏透明窗口不会吞掉整屏点击。
///
/// `payload` 是调用方序列化好的 JSON（模式 / 待办 ID / 父级 ID / 预填日期 / 初始焦点），
/// Rust 侧不解析内容，只做透传，保持窗口层与业务字段解耦。
///
/// 只取 work_area 而非整块屏幕，是为了不遮住任务栏——遮罩本身已经形成
/// 「模态对话框」的观感，没必要连任务栏一起盖掉。
pub fn editor_open(app: &AppHandle, payload: String) -> Result<(), String> {
    // 上一个编辑窗口还在（例如用户又点了另一条待办）时先销毁，保证只有一个模态。
    if let Some(existing) = app.get_webview_window("editor") {
        let _ = existing.close();
    }

    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();

    // 必须先暂存再建窗：新窗口的前端一挂载就会来拉参数。
    eprintln!("[editor] 打开编辑窗口，参数={payload}");
    app.state::<AppState>().set_editor_payload(Some(payload));

    let editor = WebviewWindowBuilder::new(app, "editor", WebviewUrl::App("editor.html".into()))
        .title("Inkling Editor")
        .inner_size(
            work.size.width as f64 / scale,
            work.size.height as f64 / scale,
        )
        .position(
            work.position.x as f64 / scale,
            work.position.y as f64 / scale,
        )
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .build()
        .map_err(|e| format!("创建编辑窗口失败: {e}"))?;
    let _ = editor.set_skip_taskbar(true);
    // 遮罩铺满整屏，圆角只会在屏幕四角露出缺口，因此编辑窗口不设圆角。
    // 内容就绪前不接收鼠标，避免一个空的全屏透明窗口挡住底下所有点击。
    let _ = editor.set_ignore_cursor_events(true);
    // 与置顶浮窗（pin_create）一致：隐藏创建后立即 show，WebView2 才会开始初始化。
    let _ = editor.show();
    Ok(())
}

/// 编辑窗口挂载后拉取本次打开参数。
pub fn editor_payload(app: &AppHandle) -> Option<String> {
    let payload = app.state::<AppState>().editor_payload();
    eprintln!("[editor] 前端拉取打开参数，命中={}", payload.is_some());
    payload
}

/// 编辑窗口内容就绪：接管鼠标事件并聚焦（由前端在对话框首帧渲染后调用）。
pub fn editor_ready(app: &AppHandle) -> Result<(), String> {
    let editor = app.get_webview_window("editor").ok_or("编辑窗口不存在")?;
    eprintln!("[editor] 内容就绪，接管鼠标与焦点");
    let _ = editor.set_ignore_cursor_events(false);
    let _ = editor.show();
    let _ = editor.set_focus();
    Ok(())
}

/// 关闭（销毁）编辑窗口。
///
/// EDITOR_CLOSED 的广播由窗口销毁事件统一负责（见 main.rs 的 on_window_event），
/// 这样用户用 Alt+F4 等方式关闭时面板同样能恢复失焦收起计时。
pub fn editor_close(app: &AppHandle) -> Result<(), String> {
    eprintln!("[editor] 关闭编辑窗口");
    if let Some(editor) = app.get_webview_window("editor") {
        let _ = editor.close();
    }
    app.state::<AppState>().set_editor_payload(None);
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

/// 计算面板从显示器四边中点唤出的逻辑坐标。
fn panel_position(monitor: &tauri::Monitor, width: f64, height: f64, position: &str) -> (f64, f64) {
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let left = work.position.x as f64 / scale;
    let top = work.position.y as f64 / scale;
    let work_width = work.size.width as f64 / scale;
    let work_height = work.size.height as f64 / scale;
    match position {
        "bottom" => (
            left + (work_width - width) / 2.0,
            top + work_height - height - 6.0,
        ),
        "left" => (left + 6.0, top + (work_height - height) / 2.0),
        "right" => (
            left + work_width - width - 6.0,
            top + (work_height - height) / 2.0,
        ),
        _ => (left + (work_width - width) / 2.0, top + 6.0),
    }
}

/// 对指定窗口应用或撤销毛玻璃效果。
///
/// 实现统一收敛在 `crate::platform::apply_backdrop`，此处只做转发，
/// 避免窗口层与平台层各维护一套效果逻辑。
///
/// 注意：效果能否可见取决于窗口创建时是否设置了 `transparent(true)`——
/// 该属性是创建期属性，运行时不可更改；而 apply/clear 是运行时可调的，
/// 这正是「毛玻璃开关」无需销毁重建窗口即可即时生效的原因。
pub fn apply_window_effect(window: &tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    crate::platform::apply_backdrop(window, enabled);
    Ok(())
}

/// 切换归档主窗口的毛玻璃（偏好设置项）。
pub fn set_main_acrylic(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不存在")?;
    apply_window_effect(&window, enabled)
}

/// 根据当前偏好将已创建的面板移动到对应屏幕边缘。
pub fn reposition_panel(app: &AppHandle) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let settings = app.state::<AppState>().lock_store()?.get_settings()?;
    let size = panel
        .inner_size()
        .map(|value| value.to_logical::<f64>(monitor.scale_factor()))
        .unwrap_or(LogicalSize::new(PANEL_WIDTH, PANEL_MAX_HEIGHT));
    let (x, y) = panel_position(&monitor, PANEL_WIDTH, size.height, &settings.panel_position);
    panel
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

/// 呼出面板：定位到光标所在屏顶部居中，show + focus。
/// 面板可见期间 hotzone_watcher 会自动停止感应，无需在此屏蔽感应区。
pub fn panel_show(app: &AppHandle) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    reposition_panel(app)?;
    let _ = panel.show();
    let _ = panel.unminimize();
    let _ = panel.set_focus();
    let _ = app.emit(events::PANEL_SHOWN, ());
    Ok(())
}

/// 收起面板。
pub fn panel_hide(app: &AppHandle) -> Result<(), String> {
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.hide();
    }
    let _ = app.emit(events::PANEL_HIDDEN, ());
    Ok(())
}

/// 面板高度自适应（前端测量内容后调用）。
pub fn panel_resize(app: &AppHandle, height: f64) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    let clamped = height.clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT);
    let _ = panel.set_size(LogicalSize::new(PANEL_WIDTH, clamped));
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let position = app
        .state::<AppState>()
        .lock_store()?
        .get_settings()?
        .panel_position;
    let (x, y) = panel_position(&monitor, PANEL_WIDTH, clamped, &position);
    let _ = panel.set_position(LogicalPosition::new(x, y));
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
/// 最小化主窗口到任务栏。
///
/// 与 `hide_main` 的区别：最小化仍留在任务栏，用户可点回来；
/// hide 是彻底隐藏、只能从托盘唤起。标题栏的「最小化」与「关闭」分别对应这两者。
pub fn minimize_main(app: &AppHandle) -> Result<(), String> {
    let main = app.get_webview_window("main").ok_or("主窗口未初始化")?;
    eprintln!("[window] 最小化主窗口");
    main.minimize().map_err(|e| e.to_string())
}

/// 切换主窗口的最大化状态，返回切换后是否为最大化。
pub fn toggle_maximize_main(app: &AppHandle) -> Result<bool, String> {
    let main = app.get_webview_window("main").ok_or("主窗口未初始化")?;
    let maximized = main.is_maximized().map_err(|e| e.to_string())?;
    if maximized {
        main.unmaximize().map_err(|e| e.to_string())?;
    } else {
        main.maximize().map_err(|e| e.to_string())?;
    }
    eprintln!("[window] 主窗口最大化 = {}", !maximized);
    Ok(!maximized)
}

/// 主窗口当前是否最大化（前端据此切换按钮图标与窗口圆角）。
pub fn main_is_maximized(app: &AppHandle) -> Result<bool, String> {
    let main = app.get_webview_window("main").ok_or("主窗口未初始化")?;
    main.is_maximized().map_err(|e| e.to_string())
}

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
    crate::platform::apply_rounded_corners(&window);
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
    crate::platform::apply_rounded_corners(&window);
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
