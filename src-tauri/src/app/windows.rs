//! 窗口系统：hotzone / panel / main / pinned / reminder 的创建、定位与状态切换。
//!
//! 定位约定：以鼠标所在显示器为基准（多屏），统一在物理像素上换算逻辑尺寸；
//! panel 贴屏幕顶部居中，pinned 右下角级联，reminder 右上角纵向堆叠。

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindowBuilder,
};

use super::state::AppState;
use crate::events;

pub const PANEL_WIDTH: f64 = 480.0;
pub const PANEL_MIN_HEIGHT: f64 = 90.0;
pub const PANEL_MAX_HEIGHT: f64 = 600.0;
const HOTZONE_WIDTH: f64 = 240.0;
const HOTZONE_HEIGHT: f64 = 80.0;
const PINNED_SIZE: (f64, f64) = (230.0, 150.0);
const REMINDER_SIZE: (f64, f64) = (320.0, 208.0);
/// 思维导图窗口的初始尺寸。导图需要大画布，且窗口可自由缩放。
const MINDMAP_SIZE: (f64, f64) = (960.0, 700.0);

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

/// 感应区窗口 label 前缀；每块显示器一个，形如 `hotzone-0`。
pub const HOTZONE_PREFIX: &str = "hotzone-";

/// 在指定显示器上创建一个感应区窗口。
///
/// 位置与朝向跟随「面板唤出位置」：面板从哪条边弹出，就在那条边感应。
/// 窗口对鼠标完全穿透——置顶盖在屏幕边缘，若接收鼠标事件会挡住下层窗口
/// （浏览器标签栏、其他应用标题栏按钮）；悬停探测改由 hotzone_watcher 轮询全局光标完成。
/// 建窗时不传 position（builder 只认逻辑坐标，会按主屏缩放换算而落错屏），
/// 建好后用物理像素 set_position / set_size 落位。
fn create_hotzone_window(
    app: &AppHandle,
    index: usize,
    monitor: &tauri::Monitor,
    position: &str,
) -> tauri::Result<()> {
    let label = format!("{HOTZONE_PREFIX}{index}");
    let rect = hotzone_geometry(&WorkArea::of(monitor), position);
    eprintln!("[hotzone] 创建感应区 {label} position={position} rect={rect:?}");
    let hotzone = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("hotzone.html".into()))
        .title("Inkling Hotzone")
        .inner_size(HOTZONE_WIDTH, HOTZONE_HEIGHT)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .build()?;
    // Windows 上部分环境不会可靠继承 builder 的 skip_taskbar 配置，创建后再次显式设置。
    let _ = hotzone.set_skip_taskbar(true);
    let _ = hotzone.set_ignore_cursor_events(true);
    if let Err(error) = place_hotzone(app, &hotzone, &label, rect) {
        eprintln!("[hotzone] {label} 落位失败: {error}");
    }
    let _ = hotzone.show();
    Ok(())
}

/// 启动时创建核心窗口：hotzone 常驻、panel 常驻隐藏（main 由配置创建）。
pub fn create_core_windows(app: &AppHandle, silent: bool) -> tauri::Result<()> {
    let monitor = cursor_monitor(app)
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| tauri::Error::WindowNotFound)?;

    // 一次性读出建窗需要的偏好设置，避免为每项各锁一次 store。
    // 感应区与面板共用同一个「唤出位置」，必须在建感应区之前读出。
    let (position, main_acrylic) = app
        .state::<AppState>()
        .lock_store()
        .ok()
        .and_then(|store| store.get_settings().ok())
        // getter 借的是这个临时 Settings，必须在闭包里取走所有权。
        .map(|settings| (settings.panel_position().clone(), *settings.main_acrylic()))
        .unwrap_or_else(|| ("top".into(), true));

    // hotzone：透明感应区，不可聚焦、不抢焦点，且对鼠标完全穿透。
    // 需求：存在外接显示器时，每块屏幕顶部（跟随唤出位置的边）中央都能唤出面板，
    // 因此为**每块显示器**各建一个感应区窗口（label 形如 hotzone-0 / hotzone-1）。
    let monitors = app.available_monitors().unwrap_or_default();
    let monitors = if monitors.is_empty() {
        vec![monitor.clone()]
    } else {
        monitors
    };
    for (index, mon) in monitors.iter().enumerate() {
        // 记录每块屏的物理位置 / 尺寸 / 缩放：多屏混合 DPI 下定位问题全靠这行日志定位。
        eprintln!(
            "[monitor] #{index} name={:?} pos={:?} size={:?} scale={}",
            mon.name(),
            (mon.position().x, mon.position().y),
            (mon.size().width, mon.size().height),
            mon.scale_factor()
        );
        create_hotzone_window(app, index, mon, &position)?;
    }
    // 预置拓扑签名，避免 watcher 首轮对账把「首次记录」误报成「拓扑变化」。
    if let Ok(mut signature) = MONITOR_SIGNATURE.lock() {
        *signature = monitor_signature(&monitors);
    }

    // panel：预创建常驻隐藏，呼出只做 show + focus。位置用物理像素落到光标所在屏。
    let panel = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
        .title("Inkling Panel")
        .inner_size(PANEL_WIDTH, PANEL_MAX_HEIGHT)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .build()?;
    let _ = panel.set_skip_taskbar(true);
    place_panel(&panel, &WorkArea::of(&monitor), PANEL_MAX_HEIGHT, &position);
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

/// 打开思维导图窗口。
///
/// **每个笔记一个独立窗口**（label 形如 `mindmap-<id>`，新建用 `mindmap-new`）：
/// - 独立顶层窗口，不设 parent，因此关闭主窗口不会连带关掉它；
/// - 缩放与最大化都是这个窗口自己的事，不影响主窗口；
/// - 同一笔记重复打开时复用既有窗口并聚焦，而不是叠出第二个。
///
/// 与面板、编辑窗口不同，这里**保留系统标题栏**（decorations 默认 true）：
/// 导图需要频繁缩放与最大化，系统标题栏自带这些交互；画布也需要实心背景
/// 才看得清节点连线，透明毛玻璃反而干扰。
///
/// 必须是 async 命令（见 editor_open 的说明）：同步命令占着主线程，
/// 而 build() 需要主线程的事件循环去处理，会互相等待。
pub fn mindmap_open(app: &AppHandle, note_id: Option<String>) -> Result<(), String> {
    let label = match note_id.as_deref() {
        Some(id) => format!("mindmap-{id}"),
        None => "mindmap-new".to_string(),
    };

    // 已开着就聚焦，不叠第二个窗口。
    if let Some(existing) = app.get_webview_window(&label) {
        eprintln!("[mindmap] 复用既有窗口 {label}");
        let _ = existing.unminimize();
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    // 打开参数与编辑窗口同一套机制：暂存后由前端挂载时拉取。
    // WebviewUrl::App 收的是相对路径，`?` 会被转义，所以不能走查询串。
    let payload = note_id.clone().unwrap_or_default();
    app.state::<AppState>()
        .set_mindmap_payload(label.clone(), payload);

    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    // 居中于光标所在屏，并且不超出工作区。
    let width = MINDMAP_SIZE.0.min(work.size.width as f64 / scale - 40.0);
    let height = MINDMAP_SIZE.1.min(work.size.height as f64 / scale - 40.0);
    let x = work.position.x as f64 / scale + (work.size.width as f64 / scale - width) / 2.0;
    let y = work.position.y as f64 / scale + (work.size.height as f64 / scale - height) / 2.0;

    eprintln!("[mindmap] 创建窗口 {label} note_id={note_id:?}");
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("mindmap.html".into()))
        .title(if note_id.is_some() {
            "🧠 编辑思维导图 · Inkling"
        } else {
            "🧠 新建思维导图 · Inkling"
        })
        .inner_size(width, height)
        .min_inner_size(520.0, 400.0)
        .position(x, y)
        .resizable(true)
        .maximizable(true)
        .build()
        .map_err(|e| format!("创建思维导图窗口失败: {e}"))?;
    // 导图窗口是独立工作区，允许出现在任务栏，方便与主窗口来回切换。
    let _ = window.set_skip_taskbar(false);
    Ok(())
}

/// 思维导图窗口的打开参数：目标笔记 id，空串表示新建。
pub fn mindmap_payload(app: &AppHandle, label: &str) -> Option<String> {
    app.state::<AppState>().mindmap_payload(label)
}

/// 关闭指定的思维导图窗口。
pub fn mindmap_close(app: &AppHandle, label: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        eprintln!("[mindmap] 关闭窗口 {label}");
        let _ = window.close();
    }
    app.state::<AppState>().take_mindmap_payload(label);
    Ok(())
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

/// 显示器工作区（已除去任务栏）的**物理像素**矩形 + 缩放因子。
///
/// 从 `tauri::Monitor` 抽出来的纯数据：定位算法只依赖这几个数，可脱离真实显示器做单元测试。
///
/// 为什么用物理像素：多屏混合 DPI（主屏 1.5×、外接屏 1.0×）时，用 LogicalPosition 建窗 /
/// 移窗会按**当前所在屏**的缩放换算，窗口从主屏挪到外接屏会落错位置；`cursor_position`
/// 也是物理坐标。全程物理像素可以避开这一整类换算歧义。
#[derive(Debug, Clone, Copy, PartialEq)]
struct WorkArea {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    scale: f64,
}

impl WorkArea {
    fn of(monitor: &tauri::Monitor) -> Self {
        let work = monitor.work_area();
        Self {
            left: work.position.x as f64,
            top: work.position.y as f64,
            width: work.size.width as f64,
            height: work.size.height as f64,
            scale: monitor.scale_factor(),
        }
    }
}

/// 感应区窗口在某条屏幕边上的几何：(x, y, width, height)，**物理像素**。
///
/// 感应区必须与面板在**同一条边**：用户在哪条边悬停，面板就从哪条边弹出。
/// 顶/底用横条（逻辑 240 × 80），左/右用竖条（逻辑 80 × 240），按该屏缩放换算成物理像素，
/// 都紧贴工作区边缘、沿边居中。未知取值回落顶部，与 `panel_position` 的兜底一致。
fn hotzone_geometry(work: &WorkArea, position: &str) -> (f64, f64, f64, f64) {
    let (long, short) = (HOTZONE_WIDTH * work.scale, HOTZONE_HEIGHT * work.scale);
    match position {
        "bottom" => (
            work.left + (work.width - long) / 2.0,
            work.top + work.height - short,
            long,
            short,
        ),
        "left" => (
            work.left,
            work.top + (work.height - long) / 2.0,
            short,
            long,
        ),
        "right" => (
            work.left + work.width - short,
            work.top + (work.height - long) / 2.0,
            short,
            long,
        ),
        _ => (work.left + (work.width - long) / 2.0, work.top, long, short),
    }
}

/// 把感应区窗口摆到物理矩形上，并把矩形记入 AppState 供 hotzone_watcher 判定。
fn place_hotzone(
    app: &AppHandle,
    hotzone: &tauri::WebviewWindow,
    label: &str,
    rect: (f64, f64, f64, f64),
) -> Result<(), String> {
    let (x, y, w, h) = rect;
    hotzone
        .set_size(PhysicalSize::new(w.round() as u32, h.round() as u32))
        .map_err(|e| e.to_string())?;
    hotzone
        .set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32))
        .map_err(|e| e.to_string())?;
    app.state::<AppState>()
        .set_hotzone_rect(label.to_string(), (x, y, x + w, y + h));
    Ok(())
}

/// 根据当前偏好把感应区窗口移到对应屏幕边缘（尺寸也随横/竖条切换）。
///
/// 与 `reposition_panel` 成对：用户改「面板唤出位置」时两者必须一起动，
/// 否则又会回到「在顶部悬停、面板从底部弹出」的割裂状态。
pub fn reposition_hotzone(app: &AppHandle) -> Result<(), String> {
    let position = app
        .state::<AppState>()
        .lock_store()?
        .get_settings()?
        .panel_position()
        .clone();
    // 每块显示器一个感应区窗口，各自按本屏工作区重新摆放到目标边。
    let monitors = app
        .available_monitors()
        .map_err(|e| format!("获取显示器列表失败: {e}"))?;
    for (index, monitor) in monitors.iter().enumerate() {
        let label = format!("{HOTZONE_PREFIX}{index}");
        let Some(hotzone) = app.get_webview_window(&label) else {
            continue;
        };
        let rect = hotzone_geometry(&WorkArea::of(monitor), &position);
        eprintln!("[hotzone] {label} 移动到 {position} 边 rect={rect:?}");
        place_hotzone(app, &hotzone, &label, rect)?;
    }
    Ok(())
}

/// 显示器拓扑签名：每块屏的 (x, y, w, h, scale×1000)。
type MonitorSignature = Vec<(i32, i32, u32, u32, u64)>;

/// 上一次对账时的显示器拓扑签名。
static MONITOR_SIGNATURE: std::sync::Mutex<MonitorSignature> = std::sync::Mutex::new(Vec::new());

fn monitor_signature(monitors: &[tauri::Monitor]) -> MonitorSignature {
    monitors
        .iter()
        .map(|m| {
            (
                m.position().x,
                m.position().y,
                m.size().width,
                m.size().height,
                // f64 不能直接比较相等，按千分位取整后比较。
                (m.scale_factor() * 1000.0).round() as u64,
            )
        })
        .collect()
}

/// 显示器热插拔对账：让感应区窗口集合始终与当前显示器一一对应。
///
/// 由 hotzone_watcher 线程每 2 秒调用一次。拓扑（数量 / 位置 / 尺寸 / 缩放）未变时直接返回；
/// 变化时：缺的屏建窗、多出的窗关掉、已有的重摆。在非主线程上建窗与 async 命令建窗同理，
/// build() 内部会把真正的创建派发回主线程，不会死锁。
pub fn reconcile_hotzones(app: &AppHandle) {
    let Ok(monitors) = app.available_monitors() else {
        return;
    };
    if monitors.is_empty() {
        return;
    }
    let signature = monitor_signature(&monitors);
    {
        let Ok(mut last) = MONITOR_SIGNATURE.lock() else {
            return;
        };
        if *last == signature {
            return;
        }
        *last = signature;
    }
    eprintln!(
        "[monitor] 显示器拓扑变化，当前 {} 块屏，重新对账感应区",
        monitors.len()
    );

    let position = app
        .state::<AppState>()
        .lock_store()
        .ok()
        .and_then(|store| store.get_settings().ok())
        .map(|settings| settings.panel_position().clone())
        .unwrap_or_else(|| "top".into());

    // 缺的建、有的摆
    for (index, monitor) in monitors.iter().enumerate() {
        let label = format!("{HOTZONE_PREFIX}{index}");
        match app.get_webview_window(&label) {
            Some(hotzone) => {
                let rect = hotzone_geometry(&WorkArea::of(monitor), &position);
                if let Err(error) = place_hotzone(app, &hotzone, &label, rect) {
                    eprintln!("[hotzone] {label} 重摆失败: {error}");
                }
            }
            None => {
                if let Err(error) = create_hotzone_window(app, index, monitor, &position) {
                    eprintln!("[hotzone] {label} 热插拔建窗失败: {error}");
                }
            }
        }
    }
    // 多出的关：从 monitors.len() 往上探，直到某个 label 不存在为止
    let mut index = monitors.len();
    loop {
        let label = format!("{HOTZONE_PREFIX}{index}");
        let Some(hotzone) = app.get_webview_window(&label) else {
            break;
        };
        eprintln!("[hotzone] 显示器已移除，关闭 {label}");
        let _ = hotzone.close();
        app.state::<AppState>().remove_hotzone_rect(&label);
        index += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 灵动岛（spec: docs/superpowers/specs/2026-09-08-dynamic-island-design.md）
// ═══════════════════════════════════════════════════════════════════════

/// 灵动岛窗口 label。
pub const ISLAND_LABEL: &str = "island";
const ISLAND_MIN_WIDTH: f64 = 200.0;
const ISLAND_MAX_WIDTH: f64 = 800.0;
const ISLAND_MIN_HEIGHT: f64 = 28.0;
const ISLAND_MAX_HEIGHT: f64 = 72.0;
/// 悬停展开详情时的高度下限（逻辑像素）。
const ISLAND_EXPANDED_HEIGHT: f64 = 120.0;
/// 灵动岛与感应区之间的间距（逻辑像素）：两者矩形不重叠，悬停灵动岛不会误触发 3 秒唤出计时。
const ISLAND_GAP: f64 = 4.0;

/// 已 clamp 的灵动岛参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IslandParams {
    pub width: f64,
    pub height: f64,
    pub opacity: f64,
    pub cycle_seconds: i64,
}

/// 把设置值钳到允许范围（越界时记日志，按边界值生效）。
pub fn island_clamp(width: i64, height: i64, opacity: f64, cycle_seconds: i64) -> IslandParams {
    let w = (width as f64).clamp(ISLAND_MIN_WIDTH, ISLAND_MAX_WIDTH);
    let h = (height as f64).clamp(ISLAND_MIN_HEIGHT, ISLAND_MAX_HEIGHT);
    let o = if opacity.is_finite() {
        opacity.clamp(0.3, 1.0)
    } else {
        0.85
    };
    let c = cycle_seconds.clamp(2, 30);
    if w != width as f64 || h != height as f64 || o != opacity || c != cycle_seconds {
        eprintln!(
            "[island] 设置越界已钳制 width={width}→{w} height={height}→{h} opacity={opacity}→{o} cycle={cycle_seconds}→{c}"
        );
    }
    IslandParams {
        width: w,
        height: h,
        opacity: o,
        cycle_seconds: c,
    }
}

/// 灵动岛几何：(x, y, w, h) 物理像素。水平居中；纵向紧贴感应区下方（感应区高 80 + 间距 4）。
fn island_geometry(work: &WorkArea, width: f64, height: f64) -> (f64, f64, f64, f64) {
    let w = width * work.scale;
    let h = height * work.scale;
    (
        work.left + (work.width - w) / 2.0,
        work.top + (HOTZONE_HEIGHT + ISLAND_GAP) * work.scale,
        w,
        h,
    )
}

/// 灵动岛所在屏：主屏（需求：只在主屏显示），拿不到时回退光标所在屏。
fn island_monitor(app: &AppHandle) -> Option<tauri::Monitor> {
    app.primary_monitor()
        .ok()
        .flatten()
        .or_else(|| cursor_monitor(app))
}

/// 从设置读出灵动岛开关、穿透与钳制后的参数。
fn island_settings(app: &AppHandle) -> Result<(bool, bool, IslandParams), String> {
    let settings = app.state::<AppState>().lock_store()?.get_settings()?;
    Ok((
        *settings.island_enabled(),
        *settings.island_click_through(),
        island_clamp(
            *settings.island_width(),
            *settings.island_height(),
            *settings.island_opacity(),
            *settings.island_cycle_seconds(),
        ),
    ))
}

/// 把灵动岛窗口摆到物理矩形上并记账。
fn place_island(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    rect: (f64, f64, f64, f64),
) -> Result<(), String> {
    let (x, y, w, h) = rect;
    window
        .set_size(PhysicalSize::new(w.round() as u32, h.round() as u32))
        .map_err(|e| e.to_string())?;
    window
        .set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32))
        .map_err(|e| e.to_string())?;
    app.state::<AppState>()
        .set_island_rect(Some((x, y, x + w, y + h)));
    Ok(())
}

/// 创建灵动岛窗口（设置为关闭时不建）。
///
/// 与感应区一样：透明、置顶、不进任务栏、不抢焦点；穿透与否按设置。
/// 圆角由前端 CSS 胶囊背景实现——DWM 圆角对透明窗口不起作用。
pub fn create_island(app: &AppHandle) -> Result<(), String> {
    let (enabled, click_through, params) = island_settings(app)?;
    if !enabled {
        eprintln!("[island] 设置为关闭，跳过创建");
        return Ok(());
    }
    if app.get_webview_window(ISLAND_LABEL).is_some() {
        return Ok(());
    }
    let monitor = island_monitor(app).ok_or("未找到主显示器，跳过灵动岛")?;
    let rect = island_geometry(&WorkArea::of(&monitor), params.width, params.height);
    eprintln!("[island] 创建灵动岛 rect={rect:?} click_through={click_through} params={params:?}");
    let window =
        WebviewWindowBuilder::new(app, ISLAND_LABEL, WebviewUrl::App("island.html".into()))
            .title("Inkling Island")
            .inner_size(params.width, params.height)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .resizable(false)
            .shadow(false)
            .visible(false)
            .build()
            .map_err(|e| format!("创建灵动岛窗口失败: {e}"))?;
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_ignore_cursor_events(click_through);
    place_island(app, &window, rect)?;
    let _ = window.show();
    Ok(())
}

/// 设置变更后重新应用：显隐、尺寸、位置、穿透。
pub fn island_apply(app: &AppHandle) -> Result<(), String> {
    let (enabled, click_through, params) = island_settings(app)?;
    let existing = app.get_webview_window(ISLAND_LABEL);
    match (enabled, existing) {
        (false, Some(window)) => {
            eprintln!("[island] 设置关闭，销毁灵动岛");
            let _ = window.close();
            app.state::<AppState>().set_island_rect(None);
            Ok(())
        }
        (false, None) => Ok(()),
        (true, None) => create_island(app),
        (true, Some(window)) => {
            let monitor = island_monitor(app).ok_or("未找到主显示器")?;
            let _ = window.set_ignore_cursor_events(click_through);
            let rect = island_geometry(&WorkArea::of(&monitor), params.width, params.height);
            eprintln!("[island] 应用新设置 rect={rect:?} click_through={click_through}");
            place_island(app, &window, rect)
        }
    }
}

/// 悬停展开 / 收起：只改高度，顶部位置不变。
pub fn island_expand(app: &AppHandle, expanded: bool) -> Result<(), String> {
    let window = app
        .get_webview_window(ISLAND_LABEL)
        .ok_or("灵动岛窗口不存在")?;
    let (_, _, params) = island_settings(app)?;
    let monitor = island_monitor(app).ok_or("未找到主显示器")?;
    let height = if expanded {
        params.height.max(ISLAND_EXPANDED_HEIGHT)
    } else {
        params.height
    };
    let rect = island_geometry(&WorkArea::of(&monitor), params.width, height);
    place_island(app, &window, rect)
}

/// 呼出面板并要求它切到指定插件页（灵动岛点击使用）。
///
/// 意图 `{"page":…}` 先存进 AppState，再 show 面板；面板在 panel-shown 事件后主动来取。
/// 顺带广播一次 PANEL_NAVIGATE，面板若已可见能立即响应。
pub fn panel_show_page(app: &AppHandle, page: &str) -> Result<(), String> {
    eprintln!("[panel] 带页呼出 page={page}");
    let intent = serde_json::json!({ "page": page }).to_string();
    app.state::<AppState>()
        .set_pending_panel_intent(Some(intent));
    panel_show(app)?;
    let _ = app.emit(events::PANEL_NAVIGATE, page.to_string());
    Ok(())
}

/// 把一条笔记回显到面板编辑（主窗口笔记列表 / 日期详情的 ✏️，spec D16）。
///
/// 顺序与原型 `openNoteInPanel` 一致：先隐藏主窗口，再呼出面板；意图带 noteId，
/// 面板取走后切到笔记页并调用 `loadNote`。面板已可见时 `panel_show` 仍会广播 PANEL_SHOWN，
/// 前端每次都取意图，因此隐藏 / 可见两种情况走同一条路径。
pub fn panel_open_note(app: &AppHandle, note_id: &str) -> Result<(), String> {
    eprintln!("[panel] 回显笔记到面板 note_id={note_id}");
    let intent = serde_json::json!({ "page": "note", "noteId": note_id }).to_string();
    app.state::<AppState>()
        .set_pending_panel_intent(Some(intent));
    hide_main(app)?;
    panel_show(app)
}

/// 面板从显示器四边中点唤出的左上角坐标（**物理像素**）。
/// `width` / `height` 为面板逻辑尺寸，按该屏缩放换算；边距逻辑 6px。
fn panel_position(work: &WorkArea, width: f64, height: f64, position: &str) -> (f64, f64) {
    let (w, h, gap) = (width * work.scale, height * work.scale, 6.0 * work.scale);
    match position {
        "bottom" => (
            work.left + (work.width - w) / 2.0,
            work.top + work.height - h - gap,
        ),
        "left" => (work.left + gap, work.top + (work.height - h) / 2.0),
        "right" => (
            work.left + work.width - w - gap,
            work.top + (work.height - h) / 2.0,
        ),
        _ => (work.left + (work.width - w) / 2.0, work.top + gap),
    }
}

/// 面板当前的逻辑高度（由前端量测内容后经 panel_resize 写入；默认最大高度）。
///
/// 不回读 `panel.inner_size()` 再换算：面板可能正处在另一块缩放不同的屏上，换算会失真。
static PANEL_LOGICAL_HEIGHT: std::sync::Mutex<f64> = std::sync::Mutex::new(PANEL_MAX_HEIGHT);

fn panel_logical_height() -> f64 {
    PANEL_LOGICAL_HEIGHT
        .lock()
        .map(|h| *h)
        .unwrap_or(PANEL_MAX_HEIGHT)
}

/// 面板是否处于 Zen 专注模式（spec D15）：窗口铺满光标所在屏工作区，退出时按逻辑高度还原。
static PANEL_ZEN: AtomicBool = AtomicBool::new(false);

/// 进入 / 退出 Zen 专注模式。
///
/// 进入：把面板窗口放大到光标所在屏的工作区（物理像素，不遮任务栏），再置 `PANEL_ZEN`；
/// 退出：按 `PANEL_LOGICAL_HEIGHT` 与当前唤出位置重新 `place_panel`，成功后才清 `PANEL_ZEN`。
/// 进入与退出各自取**当时**光标所在屏（多屏下退出时可能落到另一块屏，spec §8 已知风险）。
///
/// 标志统一在几何成功应用之后再写：若退出时读设置失败就提前返回，标志仍为 true，
/// 窗口也仍铺满工作区，二者保持一致；否则会出现「标志已清、窗口还是全屏」的错位。
pub fn panel_set_zen(app: &AppHandle, on: bool) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    if on {
        fill_work_area(app, &panel)?;
    } else {
        let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
        let work = WorkArea::of(&monitor);
        let position = app
            .state::<AppState>()
            .lock_store()?
            .get_settings()?
            .panel_position()
            .clone();
        let height = panel_logical_height();
        eprintln!("[panel] 退出 Zen，按逻辑高度 {height} 还原到 {position} 边");
        place_panel(&panel, &work, height, &position);
    }
    // 几何已成功应用，此时再写标志，保证标志与窗口实际状态一致。
    PANEL_ZEN.store(on, Ordering::SeqCst);
    Ok(())
}

/// 把面板窗口铺满**光标所在屏**的工作区（物理像素，不遮任务栏）。
///
/// Zen 态的几何来源，`panel_set_zen(true)` 与 Zen 态下的 `reposition_panel` 共用，
/// 保证两条路径算出的矩形一致。
fn fill_work_area(app: &AppHandle, panel: &tauri::WebviewWindow) -> Result<(), String> {
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let work = WorkArea::of(&monitor);
    eprintln!(
        "[panel] Zen 铺满工作区 left={} top={} w={} h={}",
        work.left, work.top, work.width, work.height
    );
    let _ = panel.set_size(PhysicalSize::new(
        work.width.round() as u32,
        work.height.round() as u32,
    ));
    let _ = panel.set_position(PhysicalPosition::new(
        work.left.round() as i32,
        work.top.round() as i32,
    ));
    Ok(())
}

/// 把面板按逻辑尺寸摆到目标屏的目标边（物理像素落位）。
fn place_panel(panel: &tauri::WebviewWindow, work: &WorkArea, height: f64, position: &str) {
    let (x, y) = panel_position(work, PANEL_WIDTH, height, position);
    let _ = panel.set_size(PhysicalSize::new(
        (PANEL_WIDTH * work.scale).round() as u32,
        (height * work.scale).round() as u32,
    ));
    let _ = panel.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
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
///
/// Zen 态下改为重新铺满光标所在屏工作区而不是按逻辑高度 `place_panel`：
/// 面板可见且处于 Zen 时，`panel_show_page` / `panel_open_note` 仍会经 `panel_show` 走到这里，
/// 若此时按逻辑高度缩小窗口，`PANEL_ZEN` 却仍为 true，后续 `panel_resize` 全部被跳过，
/// 前端也仍认为自己在 Zen——窗口与状态就此错位。
pub fn reposition_panel(app: &AppHandle) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    if PANEL_ZEN.load(Ordering::SeqCst) {
        eprintln!("[panel] Zen 态重摆：保持铺满工作区");
        return fill_work_area(app, &panel);
    }
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let settings = app.state::<AppState>().lock_store()?.get_settings()?;
    place_panel(
        &panel,
        &WorkArea::of(&monitor),
        panel_logical_height(),
        settings.panel_position(),
    );
    Ok(())
}

/// 呼出面板：定位到光标所在屏顶部居中，show + focus。
/// 面板可见期间 hotzone_watcher 会自动停止感应，无需在此屏蔽感应区。
pub fn panel_show(app: &AppHandle) -> Result<(), String> {
    eprintln!("[panel] 收到呼出请求");
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    if let Err(error) = reposition_panel(app) {
        eprintln!("[panel] 定位面板失败: {error}");
        return Err(error);
    }
    let _ = panel.show();
    let _ = panel.unminimize();
    let _ = panel.set_focus();
    eprintln!(
        "[panel] 面板已显示 visible={:?} position={:?}",
        panel.is_visible().ok(),
        panel.outer_position().ok().map(|p| (p.x, p.y))
    );
    let _ = app.emit(events::PANEL_SHOWN, ());
    Ok(())
}

/// 收起面板。Zen 态先还原窗口尺寸（原型 hidePanel 的 setZenMode(false)），否则下次呼出仍是全屏窗口。
pub fn panel_hide(app: &AppHandle) -> Result<(), String> {
    if PANEL_ZEN.load(Ordering::SeqCst) {
        if let Err(error) = panel_set_zen(app, false) {
            eprintln!("[panel] 收起前退出 Zen 失败: {error}");
        }
    }
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.hide();
    }
    let _ = app.emit(events::PANEL_HIDDEN, ());
    Ok(())
}

/// 面板高度自适应（前端测量内容后调用）。
///
/// Zen 态只记录逻辑高度不动窗口：窗口此刻铺满工作区，退出 Zen 时按记录值还原。
pub fn panel_resize(app: &AppHandle, height: f64) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    let clamped = height.clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT);
    if let Ok(mut h) = PANEL_LOGICAL_HEIGHT.lock() {
        *h = clamped;
    }
    if PANEL_ZEN.load(Ordering::SeqCst) {
        return Ok(());
    }
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let position = app
        .state::<AppState>()
        .lock_store()?
        .get_settings()?
        .panel_position()
        .clone();
    place_panel(&panel, &WorkArea::of(&monitor), clamped, &position);
    Ok(())
}

/// 显示主窗口并导航到指定视图。
pub fn show_main(app: &AppHandle, view: &str) -> Result<(), String> {
    let main = app.get_webview_window("main").ok_or("主窗口未初始化")?;
    let _ = main.show();
    let _ = main.unminimize();
    let _ = main.set_focus();
    let _ = app.emit(events::NAVIGATE, view.to_string());
    let _ = app.emit(events::MAIN_SHOWN, ());
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
    // 需求：提醒弹窗只在主屏幕显示，不随光标跑到外接屏。
    let monitor = app
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| cursor_monitor(app))
        .ok_or("未找到可用显示器")?;
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

// ═══════════════════════════════════════════════════════════════════════
// 启动器搜索窗口（spec: docs/superpowers/specs/2026-09-08-launcher-search-design.md）
// ═══════════════════════════════════════════════════════════════════════

pub const LAUNCHER_LABEL: &str = "launcher";
const LAUNCHER_WIDTH: f64 = 640.0;
/// 输入框高度 + 9 条结果 × 每条 44 + 上下留白。
const LAUNCHER_HEIGHT: f64 = 56.0 + 9.0 * 44.0 + 12.0;

/// 呼出启动器：光标所在屏水平居中、垂直偏上（顶部 22%），物理像素落位后 show + focus。
///
/// 与面板不同，启动器需要键盘输入，因此**要抢焦点**（focused / set_focus）。
pub fn launcher_show(app: &AppHandle) -> Result<(), String> {
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let work = WorkArea::of(&monitor);
    let x = work.left + (work.width - LAUNCHER_WIDTH * work.scale) / 2.0;
    let y = work.top + work.height * 0.22;

    let window = match app.get_webview_window(LAUNCHER_LABEL) {
        Some(window) => window,
        None => {
            WebviewWindowBuilder::new(app, LAUNCHER_LABEL, WebviewUrl::App("launcher.html".into()))
                .title("Inkling Launcher")
                .inner_size(LAUNCHER_WIDTH, LAUNCHER_HEIGHT)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .visible(false)
                .build()
                .map_err(|e| format!("创建启动器窗口失败: {e}"))?
        }
    };
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_size(PhysicalSize::new(
        (LAUNCHER_WIDTH * work.scale).round() as u32,
        (LAUNCHER_HEIGHT * work.scale).round() as u32,
    ));
    let _ = window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
    let _ = window.show();
    let _ = window.set_focus();
    eprintln!("[launcher] 呼出搜索窗口 pos=({}, {})", x.round(), y.round());
    Ok(())
}

/// 隐藏启动器（失焦或 Esc）。
pub fn launcher_hide(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LAUNCHER_LABEL) {
        let _ = window.hide();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(left: f64, top: f64, width: f64, height: f64, scale: f64) -> WorkArea {
        WorkArea {
            left,
            top,
            width,
            height,
            scale,
        }
    }

    /// 工作区 (0, 0, 1000, 800)、缩放 1.0，各边的感应区几何。
    #[test]
    fn hotzone_geometry_follows_edge() {
        let w = work(0.0, 0.0, 1000.0, 800.0, 1.0);
        assert_eq!(hotzone_geometry(&w, "top"), (380.0, 0.0, 240.0, 80.0));
        assert_eq!(hotzone_geometry(&w, "bottom"), (380.0, 720.0, 240.0, 80.0));
        assert_eq!(hotzone_geometry(&w, "left"), (0.0, 280.0, 80.0, 240.0));
        assert_eq!(hotzone_geometry(&w, "right"), (920.0, 280.0, 80.0, 240.0));
    }

    /// 未知取值回落到顶部，与 panel_position 的兜底一致。
    #[test]
    fn hotzone_geometry_unknown_edge_falls_back_to_top() {
        let w = work(100.0, 50.0, 1000.0, 800.0, 1.0);
        assert_eq!(hotzone_geometry(&w, "diagonal"), (480.0, 50.0, 240.0, 80.0));
    }

    /// 1.5 倍缩放的主屏（2560×1600 物理、任务栏在底）：感应区尺寸按缩放放大，仍贴顶居中。
    #[test]
    fn hotzone_geometry_scales_to_physical_pixels() {
        let w = work(0.0, 0.0, 2560.0, 1528.0, 1.5);
        assert_eq!(hotzone_geometry(&w, "top"), (1100.0, 0.0, 360.0, 120.0));
    }

    /// 外接屏（位于主屏左侧、缩放 1.0）：矩形直接落在负坐标区间。
    #[test]
    fn hotzone_geometry_on_left_monitor_uses_negative_x() {
        let w = work(-2560.0, 77.0, 2560.0, 1400.0, 1.0);
        assert_eq!(hotzone_geometry(&w, "top"), (-1400.0, 77.0, 240.0, 80.0));
    }

    /// 面板落位：1.5 倍屏顶部居中，宽 480 逻辑 → 720 物理，边距 6 逻辑 → 9 物理。
    #[test]
    fn panel_position_uses_physical_pixels() {
        let w = work(0.0, 0.0, 2560.0, 1528.0, 1.5);
        assert_eq!(panel_position(&w, 480.0, 600.0, "top"), (920.0, 9.0));
        assert_eq!(
            panel_position(&w, 480.0, 600.0, "bottom"),
            (920.0, 1528.0 - 900.0 - 9.0)
        );
    }

    /// 灵动岛参数钳制：四个维度越界都按边界生效，非法透明度回默认。
    #[test]
    fn island_clamp_bounds() {
        let p = island_clamp(100, 10, 2.0, 0);
        assert_eq!(
            (p.width, p.height, p.opacity, p.cycle_seconds),
            (200.0, 28.0, 1.0, 2)
        );
        let p = island_clamp(9000, 999, 0.1, 999);
        assert_eq!(
            (p.width, p.height, p.opacity, p.cycle_seconds),
            (800.0, 72.0, 0.3, 30)
        );
        let p = island_clamp(360, 36, f64::NAN, 4);
        assert_eq!(p.opacity, 0.85);
        let p = island_clamp(360, 36, 0.85, 4);
        assert_eq!(
            (p.width, p.height, p.opacity, p.cycle_seconds),
            (360.0, 36.0, 0.85, 4)
        );
    }

    /// 灵动岛几何：居中，纵向落在感应区（80）+ 间距（4）之下；1.5 倍缩放全部按物理像素放大。
    #[test]
    fn island_geometry_sits_below_hotzone() {
        let w = work(0.0, 0.0, 1000.0, 800.0, 1.0);
        assert_eq!(island_geometry(&w, 360.0, 36.0), (320.0, 84.0, 360.0, 36.0));
        let w = work(0.0, 0.0, 2560.0, 1528.0, 1.5);
        assert_eq!(
            island_geometry(&w, 360.0, 36.0),
            (1010.0, 126.0, 540.0, 54.0)
        );
    }
}
