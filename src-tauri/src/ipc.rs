//! IPC 命令层：#[tauri::command] 入口。命令只做参数搬运与事件广播，业务在 domain/data。

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::app::state::AppState;
use crate::app::windows;
use crate::data::{notes as notes_data, todos as todos_data};
use crate::domain::models::{
    ClipboardEntry, DayActivity, DayDetailItem, MonthTrend, Note, Settings, StatsSummary, Todo,
};
use crate::events;

fn emit_all<T: Clone + serde::Serialize>(app: &AppHandle, name: &str, payload: T) {
    let _ = app.emit(name, payload);
    // 待办变化会改变托盘提示的内容（当天未完成清单），在广播的同一处顺带刷新，
    // 避免每个改待办的命令各自记得调一次。
    if name == events::TODOS_CHANGED {
        crate::app::tray::refresh_tooltip(app);
    }
}

// ── 窗口控制 ────────────────────────────────────────────────

#[tauri::command]
pub fn panel_show(app: AppHandle) -> Result<(), String> {
    windows::panel_show(&app)
}

#[tauri::command]
pub fn panel_hide(app: AppHandle) -> Result<(), String> {
    windows::panel_hide(&app)
}

#[tauri::command]
pub fn panel_resize(app: AppHandle, height: f64) -> Result<(), String> {
    windows::panel_resize(&app, height)
}

/// 打开独立编辑窗口，`payload` 为前端序列化的打开参数 JSON。
///
/// 必须是 **async** 命令：同步命令跑在主线程上，而创建窗口的 `build()` 需要主线程的
/// 事件循环去处理，同步执行会互相等待——窗口句柄虽然建出来了，但 WebView 不初始化、
/// 后续 show 也不生效。async 命令跑在线程池里，`build()` 内部再 dispatch 回主线程。
#[tauri::command]
pub async fn editor_open(app: AppHandle, payload: String) -> Result<(), String> {
    windows::editor_open(&app, payload)
}

#[tauri::command]
pub fn editor_close(app: AppHandle) -> Result<(), String> {
    windows::editor_close(&app)
}

/// 编辑窗口挂载后拉取本次打开参数。
#[tauri::command]
pub fn editor_payload(app: AppHandle) -> Result<Option<String>, String> {
    Ok(windows::editor_payload(&app))
}

/// 打开思维导图窗口。`note_id` 为空表示新建。
///
/// 必须是 async 命令：同步命令跑在主线程上，而 build() 需要主线程的事件循环
/// 去处理，两边互等——窗口句柄虽然建出来了，但 WebView 不初始化。
#[tauri::command]
pub async fn mindmap_open(app: AppHandle, note_id: Option<String>) -> Result<(), String> {
    windows::mindmap_open(&app, note_id.filter(|id| !id.is_empty()))
}

/// 思维导图窗口挂载时拉取自己的打开参数（按窗口 label 区分）。
#[tauri::command]
pub fn mindmap_payload(app: AppHandle, label: String) -> Result<Option<String>, String> {
    Ok(windows::mindmap_payload(&app, &label))
}

#[tauri::command]
pub fn mindmap_close(app: AppHandle, label: String) -> Result<(), String> {
    windows::mindmap_close(&app, &label)
}

/// 编辑窗口渲染完成，请求显示（避免先弹出空白遮罩再填内容）。
#[tauri::command]
pub fn editor_ready(app: AppHandle) -> Result<(), String> {
    windows::editor_ready(&app)
}

#[tauri::command]
pub fn show_main(app: AppHandle, view: String) -> Result<(), String> {
    windows::show_main(&app, &view)
}

#[tauri::command]
pub fn hide_main(app: AppHandle) -> Result<(), String> {
    windows::hide_main(&app)
}

/// 切换归档主窗口的毛玻璃效果（偏好设置项）。
///
/// 运行时可调，无需销毁重建窗口——窗口在 tauri.conf.json 中已声明
/// transparent: true（创建期属性），此处只负责 apply/clear 效果层。
#[tauri::command]
pub fn set_main_acrylic(app: AppHandle, enabled: bool) -> Result<(), String> {
    crate::app::windows::set_main_acrylic(&app, enabled)
}

/// 最小化主窗口到任务栏。
#[tauri::command]
pub fn minimize_main(app: AppHandle) -> Result<(), String> {
    windows::minimize_main(&app)
}

/// 切换主窗口最大化，返回切换后的状态。
#[tauri::command]
pub fn toggle_maximize_main(app: AppHandle) -> Result<bool, String> {
    windows::toggle_maximize_main(&app)
}

/// 查询主窗口是否最大化。
#[tauri::command]
pub fn main_is_maximized(app: AppHandle) -> Result<bool, String> {
    windows::main_is_maximized(&app)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) -> Result<(), String> {
    windows::quit_app(&app);
    Ok(())
}

/// 创建置顶浮窗。
///
/// 必须是 **async** 命令（与 `editor_open` / `mindmap_open` 同款）：同步命令跑在主线程上，
/// 而 `build()` 需要主线程的事件循环去处理，两边互等——窗口句柄建出来了但 WebView 永不初始化，
/// 且此后该窗口发起的所有 invoke 全部挂起（2026-09-10 阶段一验收实机复现）。
#[tauri::command]
pub async fn pin_create(app: AppHandle, kind: String, id: String) -> Result<(), String> {
    windows::pin_create(&app, &kind, &id)
}

#[tauri::command]
pub fn pin_close(app: AppHandle, label: String) -> Result<(), String> {
    windows::pin_close(&app, &label)
}

#[tauri::command]
pub fn pin_set_editing(app: AppHandle, label: String, expanded: bool) -> Result<(), String> {
    windows::pin_set_editing(&app, &label, expanded)
}

#[tauri::command]
pub fn reminder_close(app: AppHandle, todo_id: String) -> Result<(), String> {
    windows::reminder_close(&app, &todo_id)
}

#[tauri::command]
pub fn rebind_shortcut(app: AppHandle, combo: String) -> Result<String, String> {
    crate::app::shortcut::rebind(&app, &combo)
}

// ── 笔记 ────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotePayload {
    pub id: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub editor_mode: String,
    #[serde(default)]
    pub mindmap_data: Option<String>,
    pub draft: bool,
}

#[tauri::command]
pub fn notes_list(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    state.lock_store()?.list_notes()
}

#[tauri::command]
pub fn note_draft(state: State<'_, AppState>) -> Result<Option<Note>, String> {
    state.lock_store()?.active_draft()
}

/// 按 id 读取单条笔记（面板回显使用）；不存在返回 None。
#[tauri::command]
pub fn note_get(state: State<'_, AppState>, id: String) -> Result<Option<Note>, String> {
    state.lock_store()?.get_note(&id)
}

#[tauri::command]
pub fn note_save(
    app: AppHandle,
    state: State<'_, AppState>,
    input: NotePayload,
) -> Result<Note, String> {
    let note = state.lock_store()?.save_note(&notes_data::NoteInput {
        id: input.id,
        content: input.content,
        tags: input.tags,
        editor_mode: input.editor_mode,
        mindmap_data: input.mindmap_data,
        draft: input.draft,
    })?;
    if !note.is_draft() {
        emit_all(&app, events::NOTES_CHANGED, note.id().clone());
        emit_all(&app, events::STATS_CHANGED, ());
    }
    Ok(note)
}

#[tauri::command]
pub fn note_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.lock_store()?.delete_note(&id)?;
    emit_all(&app, events::NOTES_CHANGED, id);
    emit_all(&app, events::STATS_CHANGED, ());
    Ok(())
}

#[tauri::command]
pub fn note_set_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<Note, String> {
    let store = state.lock_store()?;
    store
        .db
        .execute(
            "UPDATE notes SET pinned=?, updated_at=? WHERE id=?",
            rusqlite::params![pinned as i64, crate::data::now(), id],
        )
        .map_err(crate::data::db_err)?;
    let note = store.note(&id)?;
    drop(store);
    emit_all(&app, events::NOTES_CHANGED, id);
    Ok(note)
}

// ── 剪贴板 ──────────────────────────────────────────────────

#[tauri::command]
pub fn clipboard_list(state: State<'_, AppState>) -> Result<Vec<ClipboardEntry>, String> {
    state.lock_store()?.list_clipboard()
}

#[tauri::command]
pub fn clipboard_capture(app: AppHandle) -> Result<Option<ClipboardEntry>, String> {
    // 手动捕获：读取当前系统剪贴板文本。
    let text = read_system_text().ok_or("读取系统剪贴板失败")?;
    if text.is_empty() {
        return Ok(None);
    }
    let hash = crate::domain::clipboard::hash_content(text.as_bytes());
    let entry = crate::services::clipboard_watcher::capture_text(&app, text, hash, false);
    if let Some(entry) = &entry {
        emit_all(&app, events::CLIPBOARD_CHANGED, entry.id().clone());
        emit_all(&app, events::STATS_CHANGED, ());
    }
    Ok(entry)
}

fn read_system_text() -> Option<String> {
    let mut board = arboard::Clipboard::new().ok()?;
    board.get_text().ok()
}

/// 写回系统剪贴板并登记回声哈希（防止应用记录自身写回）。
#[tauri::command]
pub fn clipboard_write(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let (entry, file) = {
        let store = state.lock_store()?;
        let (entry, file) = store.paste_payload(&id)?;
        (entry, file)
    };
    let mut board = arboard::Clipboard::new().map_err(|e| format!("访问剪贴板失败: {e}"))?;
    let hash = if let Some(path) = file.filter(|_| entry.content_type() == "image") {
        let bytes = std::fs::read(&path).map_err(|e| format!("读取图片附件失败: {e}"))?;
        let image = image::load_from_memory(&bytes).map_err(|e| format!("解码图片失败: {e}"))?;
        let rgba = image.to_rgba8();
        let (width, height) = (rgba.width() as usize, rgba.height() as usize);
        board
            .set_image(arboard::ImageData {
                width,
                height,
                bytes: rgba.into_raw().into(),
            })
            .map_err(|e| format!("写回剪贴板失败: {e}"))?;
        crate::domain::clipboard::hash_content(format!("image:{width}x{height}").as_bytes())
    } else {
        let text = if entry.content().is_empty() {
            entry.preview().clone()
        } else {
            entry.content().clone()
        };
        board
            .set_text(&text)
            .map_err(|e| format!("写回剪贴板失败: {e}"))?;
        crate::domain::clipboard::hash_content(text.as_bytes())
    };
    app.state::<AppState>().set_echo(Some(hash));
    Ok(())
}

/// 粘贴到光标处。
///
/// 与 `clipboard_write`（仅写回剪贴板）不同，本命令完成一次完整的「粘贴」动作：
/// 1. 把条目写入系统剪贴板；
/// 2. 收起呼出面板——面板是获得焦点的窗口，只有隐藏它，
///    用户原来正在编辑的应用才会重新成为前台窗口；
/// 3. 等待焦点切换落定后模拟 Ctrl/Cmd+V，把内容送到光标所在位置。
///
/// 第 3 步必须在焦点归还之后执行，否则按键会被面板自身吃掉，因此中间有一小段延时。
#[tauri::command]
pub fn clipboard_paste(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // 先写入剪贴板（复用既有逻辑，图片/文本分支一致）。
    clipboard_write(app.clone(), state, id)?;

    // 收起面板，把前台焦点还给用户原本所在的应用。
    crate::app::windows::panel_hide(&app)?;

    // 焦点切换是异步的，立刻发按键会落在正在消失的面板上。
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(120));
        if let Err(error) = send_paste_keystroke() {
            eprintln!("[clipboard] 模拟粘贴按键失败：{error}");
        }
        let _ = app;
    });

    Ok(())
}

/// 模拟一次「粘贴」按键：macOS 用 Cmd+V，其余平台用 Ctrl+V。
fn send_paste_keystroke() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("初始化输入模拟失败: {e}"))?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| format!("按下修饰键失败: {e}"))?;
    let result = enigo.key(Key::Unicode('v'), Direction::Click);
    // 无论主键是否成功，都必须松开修饰键，否则会把用户键盘卡在按下状态。
    let release = enigo.key(modifier, Direction::Release);

    result.map_err(|e| format!("发送粘贴按键失败: {e}"))?;
    release.map_err(|e| format!("释放修饰键失败: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn clipboard_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    content: String,
) -> Result<ClipboardEntry, String> {
    let entry = state.lock_store()?.update_clipboard(&id, &content)?;
    emit_all(&app, events::CLIPBOARD_CHANGED, id);
    Ok(entry)
}

#[tauri::command]
pub fn clipboard_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<(), String> {
    state.lock_store()?.set_clipboard_pinned(&id, pinned)?;
    emit_all(&app, events::CLIPBOARD_CHANGED, id);
    Ok(())
}

#[tauri::command]
pub fn clipboard_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.lock_store()?.delete_clipboard(&id)?;
    emit_all(&app, events::CLIPBOARD_CHANGED, id);
    Ok(())
}

#[tauri::command]
pub fn clipboard_cleanup(app: AppHandle) -> Result<usize, String> {
    crate::services::clipboard_watcher::cleanup(&app)
}

// ── 待办 ────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoPayload {
    pub id: Option<String>,
    pub content: String,
    pub due_at: String,
    /// 提醒偏移分钟数；`None` = 不提醒。
    pub remind_offset_minutes: Option<i64>,
    #[serde(default)]
    pub remind_desktop: bool,
    #[serde(default)]
    pub remind_email: bool,
    pub repeat_rule: Option<String>,
    pub priority: String,
    pub remark: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub allow_past: bool,
}

fn todo_input(payload: TodoPayload) -> todos_data::TodoInput {
    todos_data::TodoInput {
        id: payload.id,
        content: payload.content,
        due_at: payload.due_at,
        remind_offset_minutes: payload.remind_offset_minutes,
        remind_desktop: payload.remind_desktop,
        remind_email: payload.remind_email,
        repeat_rule: payload.repeat_rule,
        priority: payload.priority,
        remark: payload.remark,
        tags: payload.tags,
        parent_id: payload.parent_id,
        allow_past: payload.allow_past,
    }
}

#[tauri::command]
pub fn todos_list(state: State<'_, AppState>) -> Result<Vec<Todo>, String> {
    state.lock_store()?.list_todos()
}

#[tauri::command]
pub fn todo_save(
    app: AppHandle,
    state: State<'_, AppState>,
    input: TodoPayload,
) -> Result<Todo, String> {
    // 勾了邮件却没配好 SMTP 时直接拦截，避免用户以为已生效却收不到信。
    if input.remind_email && !crate::services::mailer::is_configured(&app) {
        return Err("尚未配置邮件提醒，请先在设置页填写 SMTP 信息".into());
    }
    let parent_id = input.parent_id.clone();
    let todo = if let Some(parent) = parent_id.filter(|_| input.id.is_none()) {
        state
            .lock_store()?
            .create_child_todo(&parent, &todo_input(input))?
    } else {
        state.lock_store()?.save_todo(&todo_input(input))?
    };
    emit_all(&app, events::TODOS_CHANGED, todo.id().clone());
    emit_all(&app, events::STATS_CHANGED, ());
    Ok(todo)
}

#[tauri::command]
pub fn todo_complete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    completed: bool,
) -> Result<Vec<Todo>, String> {
    let todos = state.lock_store()?.complete_todo(&id, completed)?;
    emit_all(&app, events::TODOS_CHANGED, id);
    emit_all(&app, events::STATS_CHANGED, ());
    Ok(todos)
}

#[tauri::command]
pub fn todo_priority(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    priority: String,
) -> Result<Todo, String> {
    let todo = state.lock_store()?.set_todo_priority(&id, &priority)?;
    emit_all(&app, events::TODOS_CHANGED, id);
    Ok(todo)
}

#[tauri::command]
pub fn todo_due(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    due_at: String,
) -> Result<Todo, String> {
    let todo = state.lock_store()?.set_todo_due(&id, &due_at)?;
    emit_all(&app, events::TODOS_CHANGED, id);
    emit_all(&app, events::STATS_CHANGED, ());
    Ok(todo)
}

#[tauri::command]
pub fn todo_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    offset_minutes: Option<i64>,
    desktop: bool,
    email: bool,
    repeat_rule: Option<String>,
) -> Result<Todo, String> {
    // 勾了邮件却没配好 SMTP 时直接拦截，避免用户以为已生效却收不到信。
    if email && !crate::services::mailer::is_configured(&app) {
        return Err("尚未配置邮件提醒，请先在设置页填写 SMTP 信息".into());
    }
    let todo = state.lock_store()?.set_todo_reminder(
        &id,
        offset_minutes,
        desktop,
        email,
        repeat_rule.as_deref(),
    )?;
    emit_all(&app, events::TODOS_CHANGED, id);
    Ok(todo)
}

/// 发送一封测试邮件，用于验证 SMTP 配置。同步执行，错误原样回传界面。
#[tauri::command]
pub fn mail_test(app: AppHandle) -> Result<(), String> {
    crate::services::mailer::send_now(
        &app,
        &crate::services::mailer::MailRequest {
            subject: "[Inkling] 测试邮件".into(),
            body: "这是一封来自 Inkling 的测试邮件。收到它说明邮件提醒已配置成功。".into(),
        },
    )
}

#[tauri::command]
pub fn todo_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.lock_store()?.delete_todo(&id)?;
    emit_all(&app, events::TODOS_CHANGED, id);
    emit_all(&app, events::STATS_CHANGED, ());
    Ok(())
}

/// 提醒卡片「稍后提醒」：写入一次性的额外提醒时刻。
#[tauri::command]
pub fn todo_snooze(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    minutes: i64,
) -> Result<Todo, String> {
    let store = state.lock_store()?;
    let todo = store.snooze_todo(&id, minutes)?;
    drop(store);
    emit_all(&app, events::TODOS_CHANGED, id);
    Ok(todo)
}

/// 提醒卡片「不再提醒」：关闭抑制。
#[tauri::command]
pub fn todo_dismiss_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let store = state.lock_store()?;
    store
        .db
        .execute(
            "UPDATE todos SET remind_off=1, updated_at=? WHERE id=?",
            rusqlite::params![crate::data::now(), id],
        )
        .map_err(crate::data::db_err)?;
    drop(store);
    emit_all(&app, events::TODOS_CHANGED, id);
    Ok(())
}

// ── 设置 / 统计 / 导出 ─────────────────────────────────────

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<Settings, String> {
    state.lock_store()?.get_settings()
}

#[tauri::command]
pub fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    state.lock_store()?.save_settings(&settings)?;
    if *settings.start_on_boot() {
        let _ = app.autolaunch().enable();
    } else {
        let _ = app.autolaunch().disable();
    }
    emit_all(&app, events::SETTINGS_CHANGED, settings);
    // 唤出方向变更：感应区与面板一起移到新的屏幕边缘（面板已打开时立即生效）。
    if let Err(error) = windows::reposition_hotzone(&app) {
        eprintln!("[settings] 移动感应区失败: {error}");
    }
    let _ = windows::reposition_panel(&app);
    // 灵动岛：显隐 / 尺寸 / 穿透随设置即时生效。
    if let Err(error) = windows::island_apply(&app) {
        eprintln!("[settings] 应用灵动岛设置失败: {error}");
    }
    Ok(())
}

/// 灵动岛悬停展开 / 收起（前端在悬停状态变化时调用）。
#[tauri::command]
pub fn island_expand(app: AppHandle, expanded: bool) -> Result<(), String> {
    windows::island_expand(&app, expanded)
}

/// 面板显示后取走本次呼出意图（JSON：`{"page":…,"noteId"?:…}`）；没有则返回 None。
#[tauri::command]
pub fn panel_take_intent(state: State<'_, AppState>) -> Option<String> {
    state.take_pending_panel_intent()
}

/// 把笔记回显到面板编辑：隐藏主窗口 → 呼出面板 → 面板取意图后载入笔记。
///
/// 不建窗，但走 hide_main + panel_show 的窗口序列；与 `launcher_show` 同款用 async，
/// 避免同步命令占住主线程与事件循环互等。
#[tauri::command]
pub async fn panel_open_note(app: AppHandle, note_id: String) -> Result<(), String> {
    windows::panel_open_note(&app, &note_id)
}

/// 进入 / 退出 Zen 专注模式（面板窗口放大到工作区 / 还原）。
///
/// 几何序列（set_size + set_position）且退出后前端紧接可能调用 panel_hide，按 async 写以免阻塞主线程。
#[tauri::command]
pub async fn panel_set_zen(app: AppHandle, on: bool) -> Result<(), String> {
    windows::panel_set_zen(&app, on)
}

// ═══ 启动器搜索 ═══

/// 搜索：返回 Top-K 命中。
#[tauri::command]
pub fn launcher_search(
    state: State<'_, crate::services::launcher::LauncherState>,
    query: String,
) -> Vec<crate::services::launcher::model::Hit> {
    state.search(&query)
}

/// 启动一个命中项。`mode`：open / admin / reveal。文件命中按 path 启动，故直接收 path/kind
/// （命中项自带，无需按 id 回查——文件索引命中不在内存候选表里）。`query` 用于记录查询亲和度。
#[tauri::command]
pub fn launcher_launch(
    app: AppHandle,
    state: State<'_, crate::services::launcher::LauncherState>,
    path: String,
    kind: String,
    mode: String,
    query: String,
) -> Result<(), String> {
    use crate::services::launcher::launch::{launch, LaunchMode};
    use crate::services::launcher::model::{Candidate, Kind};

    let parsed_kind = match kind.as_str() {
        "app" => Kind::App,
        "uwp" => Kind::Uwp,
        "file" => Kind::File,
        "folder" => Kind::Folder,
        "command" => Kind::Command,
        other => return Err(format!("未知类型 {other}")),
    };

    // 内置命令直接在此执行。
    if parsed_kind == Kind::Command {
        match path.as_str() {
            "cmd:settings" => windows::show_main(&app, "settings")?,
            "cmd:rebuild" => crate::services::launcher::rebuild_async(app.clone()),
            "cmd:quit" => windows::quit_app(&app),
            other => return Err(format!("未知命令 {other}")),
        }
        let _ = windows::launcher_hide(&app);
        return Ok(());
    }

    let launch_mode = LaunchMode::parse(&mode).ok_or("未知启动方式")?;
    let candidate = Candidate {
        id: 0,
        kind: parsed_kind,
        name: String::new(),
        path: path.clone(),
        keywords: Vec::new(),
        bias: 0.0,
    };
    launch(&candidate, launch_mode)?;
    state.record_launch(&path, &query);
    // 普通启动后收起启动器；打开所在文件夹保留窗口方便继续操作。
    if launch_mode != LaunchMode::Reveal {
        let _ = windows::launcher_hide(&app);
    }
    Ok(())
}

/// 立即重建索引（后台线程）。
#[tauri::command]
pub fn launcher_rebuild(app: AppHandle) {
    crate::services::launcher::rebuild_async(app);
}

/// 索引状态（条目数 / 代际 / 生成时间 / 是否正在重建）。
#[tauri::command]
pub fn launcher_status(
    state: State<'_, crate::services::launcher::LauncherState>,
) -> crate::services::launcher::LauncherStatus {
    state.status()
}

#[tauri::command]
pub fn launcher_hide(app: AppHandle) -> Result<(), String> {
    windows::launcher_hide(&app)
}

/// 显示启动台浮窗（主窗口启动台页的「呼出浮窗启动台」按钮）。
///
/// 必须是 async 命令：首次呼出要 `WebviewWindowBuilder::build()` 建窗，同步命令跑在主线程上会与
/// 事件循环互等（窗口句柄建出但 WebView 永不初始化，整个应用冻结），与 `editor_open` / `pin_create` 同款。
#[tauri::command]
pub async fn launcher_show(app: AppHandle) -> Result<(), String> {
    windows::launcher_show(&app)
}

/// 改绑启动器全局快捷键。
#[tauri::command]
pub fn rebind_launcher_shortcut(app: AppHandle, combo: String) -> Result<String, String> {
    crate::app::shortcut::rebind_launcher(&app, &combo)
}

#[tauri::command]
pub fn stats_heatmap(
    state: State<'_, AppState>,
    days: Option<u32>,
) -> Result<Vec<DayActivity>, String> {
    state
        .lock_store()?
        .heatmap(days.unwrap_or(182).clamp(28, 365))
}

#[tauri::command]
pub fn stats_trend(state: State<'_, AppState>) -> Result<Vec<MonthTrend>, String> {
    state.lock_store()?.trend()
}

#[tauri::command]
pub fn stats_summary(state: State<'_, AppState>) -> Result<StatsSummary, String> {
    state.lock_store()?.stats_summary()
}

#[tauri::command]
pub fn stats_day(state: State<'_, AppState>, date: String) -> Result<Vec<DayDetailItem>, String> {
    state.lock_store()?.day_detail(&date)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    /// note:id / todo:id / clip:id 列表
    pub refs: Vec<String>,
    pub format: String,
    pub output_dir: Option<String>,
}

#[tauri::command]
pub fn export_items(state: State<'_, AppState>, payload: ExportPayload) -> Result<String, String> {
    let format =
        crate::services::export::ExportFormat::from(&payload.format).ok_or("不支持的导出格式")?;
    let mut items = Vec::new();
    {
        let store = state.lock_store()?;
        for reference in &payload.refs {
            let (kind, id) = reference.split_once(':').ok_or("导出引用格式无效")?;
            match kind {
                "note" => items.push(crate::services::export::ExportItem::Note(store.note(id)?)),
                "todo" => items.push(crate::services::export::ExportItem::Todo(store.todo(id)?)),
                "clip" => {
                    let entry = store.clipboard_entry(id)?.ok_or("剪贴板条目不存在")?;
                    items.push(crate::services::export::ExportItem::Clip(entry));
                }
                _ => return Err("导出类型无效".into()),
            }
        }
    }
    let name = format!(
        "Inkling-导出-{}",
        crate::data::local_date_key(chrono::Utc::now())
    );
    crate::services::export::export_items(
        &items,
        format,
        payload.output_dir.map(std::path::PathBuf::from),
        &name,
    )
}

/// 数据与附件目录位置（设置页「打开数据目录」使用）。
#[tauri::command]
pub fn data_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.lock_store()?.data_dir.to_string_lossy().to_string())
}

/// 把 base64 内容解码后写到指定绝对路径（思维导图导出使用）。
///
/// 前端拿到库导出的 dataURL 后剥掉 `data:*;base64,` 前缀再传入；路径来自系统保存对话框，
/// 不做目录创建——对话框选出的目录必然存在。先写临时文件再重命名，避免半截文件。
fn write_base64_to_path(path: &str, base64_content: &str) -> Result<String, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_content.trim())
        .map_err(|e| format!("base64 解码失败: {e}"))?;
    let target = std::path::PathBuf::from(path);
    let temp = target.with_extension(format!(
        "{}.tmp",
        target.extension().and_then(|e| e.to_str()).unwrap_or("bin")
    ));
    std::fs::write(&temp, &bytes).map_err(|e| format!("写入文件失败: {e}"))?;
    std::fs::rename(&temp, &target).map_err(|e| format!("提交文件失败: {e}"))?;
    eprintln!("[files] 已写入 {} ({} 字节)", target.display(), bytes.len());
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub fn write_file_base64(path: String, base64: String) -> Result<String, String> {
    write_base64_to_path(&path, &base64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_base64_decodes_and_writes() {
        let dir = std::env::temp_dir().join(format!("inkling-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hello.txt");
        // "hello" 的 base64
        let written = write_base64_to_path(path.to_str().unwrap(), "aGVsbG8=").unwrap();
        assert_eq!(std::fs::read_to_string(&written).unwrap(), "hello");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_base64_rejects_bad_input() {
        let path = std::env::temp_dir().join("inkling-bad.bin");
        assert!(write_base64_to_path(path.to_str().unwrap(), "***not base64***").is_err());
    }
}
