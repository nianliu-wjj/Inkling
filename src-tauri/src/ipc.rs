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

#[tauri::command]
pub fn quit_app(app: AppHandle) -> Result<(), String> {
    windows::quit_app(&app);
    Ok(())
}

#[tauri::command]
pub fn pin_create(app: AppHandle, kind: String, id: String) -> Result<(), String> {
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
    if !note.is_draft {
        emit_all(&app, events::NOTES_CHANGED, note.id.clone());
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
        emit_all(&app, events::CLIPBOARD_CHANGED, entry.id.clone());
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
    let hash = if let Some(path) = file.filter(|_| entry.content_type == "image") {
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
        crate::domain::clipboard::hash_content(&format!("image:{width}x{height}").as_bytes())
    } else {
        let text = if entry.content.is_empty() {
            entry.preview.clone()
        } else {
            entry.content.clone()
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
    pub remind_at: Option<String>,
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
        remind_at: payload.remind_at,
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
    let parent_id = input.parent_id.clone();
    let todo = if let Some(parent) = parent_id.filter(|_| input.id.is_none()) {
        state
            .lock_store()?
            .create_child_todo(&parent, &todo_input(input))?
    } else {
        state.lock_store()?.save_todo(&todo_input(input))?
    };
    emit_all(&app, events::TODOS_CHANGED, todo.id.clone());
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
    remind_at: Option<String>,
    repeat_rule: Option<String>,
) -> Result<Todo, String> {
    let todo =
        state
            .lock_store()?
            .set_todo_reminder(&id, remind_at.as_deref(), repeat_rule.as_deref())?;
    emit_all(&app, events::TODOS_CHANGED, id);
    Ok(todo)
}

#[tauri::command]
pub fn todo_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.lock_store()?.delete_todo(&id)?;
    emit_all(&app, events::TODOS_CHANGED, id);
    emit_all(&app, events::STATS_CHANGED, ());
    Ok(())
}

/// 提醒卡片「稍后提醒」：只更新 remind_at。
#[tauri::command]
pub fn todo_snooze(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    minutes: i64,
) -> Result<Todo, String> {
    let next = (chrono::Utc::now() + chrono::Duration::minutes(minutes)).to_rfc3339();
    let store = state.lock_store()?;
    let todo = store.set_todo_reminder(&id, Some(&next), None)?;
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
    if settings.start_on_boot {
        let _ = app.autolaunch().enable();
    } else {
        let _ = app.autolaunch().disable();
    }
    emit_all(&app, events::SETTINGS_CHANGED, settings);
    // 面板已打开时立即应用新的唤出方向。
    let _ = windows::reposition_panel(&app);
    Ok(())
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
