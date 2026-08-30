//! 持久化数据层：笔记、剪贴板和待办统一存储在应用数据目录 SQLite。
//!
//! UI 只通过本模块读写业务数据。写操作先完成 SQLite 事务，再刷新内存快照，
//! 从而保证主窗口与顶部呼出面板看到同一份数据。

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::TimeZone;
use gpui::{App, Global, Image, ImageFormat};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::settings::{ClipRetention, Settings};

static ID_SEQ: AtomicU64 = AtomicU64::new(1);

crate::accessors! {
    #[derive(Clone, Debug)]
    pub struct Note {
        id: String,
        content: String,
        tags: Vec<String>,
        created_at: String,
        updated_at: String,
    }
}

crate::accessors! {
    #[derive(Clone, Debug)]
    pub struct ClipItem {
        id: String,
        content: String,
        kind: String,
        storage_path: Option<String>,
        captured_at: String,
        favorite: bool,
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum Priority {
    High,
    #[default]
    Medium,
    Low,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::High => "高",
            Self::Medium => "中",
            Self::Low => "低",
        }
    }
    pub fn rank(self) -> u8 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => 2,
        }
    }
    fn parse(value: &str) -> Self {
        match value {
            "high" => Self::High,
            "low" => Self::Low,
            _ => Self::Medium,
        }
    }
}

crate::accessors! {
    #[derive(Clone, Debug)]
    pub struct TodoItem {
        id: String,
        text: String,
        done: bool,
        due_at: String,
        completed_at: Option<String>,
        remind_at: Option<String>,
        repeat_rule: Option<String>,
        priority: Priority,
        tags: Vec<String>,
        remark: String,
        parent_id: Option<String>,
        created_at: String,
        updated_at: String,
    }
}

crate::accessors! {
    #[derive(Clone, Debug)]
    pub struct Reminder {
        id: String,
        todo_id: String,
        text: String,
        trigger_at: String,
    }
}

#[derive(Default)]
pub struct Store {
    notes: Vec<Note>,
    todos: Vec<TodoItem>,
    clips: Vec<ClipItem>,
    draft_id: Option<String>,
    draft_content: String,
    db_path: Option<PathBuf>,
    last_error: Option<String>,
}

impl Global for Store {}

fn app_data_dir() -> PathBuf {
    if let Ok(value) = std::env::var("APPDATA") {
        return PathBuf::from(value).join("inkling");
    }
    #[cfg(target_os = "macos")]
    if let Ok(value) = std::env::var("HOME") {
        return PathBuf::from(value).join("Library/Application Support/inkling");
    }
    if let Ok(value) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(value).join("inkling");
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".local/share/inkling")
}

fn db_path() -> PathBuf {
    app_data_dir().join("inkling.sqlite3")
}

fn open_db(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.execute_batch(SCHEMA)?;
    // 为早期版本数据库补充图片文件路径字段；重复执行时忽略“列已存在”。
    if let Err(error) = conn.execute("ALTER TABLE clips ADD COLUMN storage_path TEXT", []) {
        if !error.to_string().contains("duplicate column name") {
            return Err(error);
        }
    }
    Ok(conn)
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS notes (
 id TEXT PRIMARY KEY, content TEXT NOT NULL DEFAULT '', storage_path TEXT,
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL, archived_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS note_tags (
 note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
 tag TEXT NOT NULL, PRIMARY KEY(note_id, tag)
);
CREATE TABLE IF NOT EXISTS note_drafts (
 id TEXT PRIMARY KEY, content TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS clips (
 id TEXT PRIMARY KEY, content TEXT NOT NULL, kind TEXT NOT NULL DEFAULT 'text',
 storage_path TEXT, content_hash TEXT NOT NULL UNIQUE, captured_at TEXT NOT NULL, favorite INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS todos (
 id TEXT PRIMARY KEY, content TEXT NOT NULL, due_at TEXT NOT NULL,
 completed_at TEXT, status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','done')),
 remind_at TEXT, repeat_rule TEXT, priority TEXT NOT NULL DEFAULT 'medium',
 remark TEXT NOT NULL DEFAULT '', parent_id TEXT REFERENCES todos(id) ON DELETE CASCADE,
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_todos_due ON todos(due_at, status);
CREATE INDEX IF NOT EXISTS idx_todos_parent ON todos(parent_id);
CREATE TABLE IF NOT EXISTS reminder_events (
 id TEXT PRIMARY KEY, todo_id TEXT NOT NULL REFERENCES todos(id) ON DELETE CASCADE,
 trigger_at TEXT NOT NULL, triggered_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_reminder_events_todo ON reminder_events(todo_id);
CREATE TABLE IF NOT EXISTS todo_tags (
 todo_id TEXT NOT NULL REFERENCES todos(id) ON DELETE CASCADE,
 tag TEXT NOT NULL, PRIMARY KEY(todo_id, tag)
);
CREATE TABLE IF NOT EXISTS activity_events (
 id TEXT PRIMARY KEY, event_type TEXT NOT NULL, entity_id TEXT NOT NULL, occurred_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS stats_daily (
 date TEXT PRIMARY KEY, note_archived_count INTEGER NOT NULL DEFAULT 0,
 clipboard_captured_count INTEGER NOT NULL DEFAULT 0, todo_created_count INTEGER NOT NULL DEFAULT 0,
 todo_completed_count INTEGER NOT NULL DEFAULT 0, updated_at TEXT NOT NULL
);
"#;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
fn now_string() -> String {
    now_secs().to_string()
}

pub fn default_due_at() -> String {
    (now_secs() + 3_600).to_string()
}

pub fn reminder_after(seconds: u64) -> String {
    (now_secs() + seconds).to_string()
}

/// 将内部 Unix 秒时间戳转换为稳定、可读的本地日期时间文本。
pub fn display_timestamp(timestamp: &str) -> String {
    let Ok(seconds) = timestamp.parse::<i64>() else {
        return timestamp.to_string();
    };
    chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0)
        .map(|value| value.with_timezone(&chrono::Local))
        .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| timestamp.to_string())
}

/// 将界面输入的 Unix 秒或本地日期时间文本统一转换为 Unix 秒字符串。
/// 文本输入是 GPUI 当前版本下日期控件不可用时的降级入口。
fn parse_timestamp(value: &str) -> Option<String> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds.to_string());
    }
    for format in ["%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(local) = chrono::NaiveDateTime::parse_from_str(value, format) {
            return chrono::Local
                .from_local_datetime(&local)
                .single()
                .map(|date| date.timestamp().to_string());
        }
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .and_then(|local| chrono::Local.from_local_datetime(&local).single())
        .map(|date| date.timestamp().to_string())
}
fn id(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        now_secs(),
        ID_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}
fn hash_content(value: &str) -> String {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn hash_bytes(value: &[u8]) -> String {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn image_extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Webp => "webp",
        ImageFormat::Gif => "gif",
        ImageFormat::Svg => "svg",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Tiff => "tiff",
    }
}

fn image_format_from_path(path: &Path) -> Option<ImageFormat> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::Webp),
        "gif" => Some(ImageFormat::Gif),
        "svg" => Some(ImageFormat::Svg),
        "bmp" => Some(ImageFormat::Bmp),
        "tif" | "tiff" => Some(ImageFormat::Tiff),
        _ => None,
    }
}

fn clip_image_path(hash: &str, format: ImageFormat) -> PathBuf {
    app_data_dir()
        .join("clips")
        .join(format!("{hash}.{}", image_extension(format)))
}

fn write_binary_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "图片路径无父目录"));
    };
    std::fs::create_dir_all(parent)?;
    let temp = path.with_extension(format!("{}.tmp", image_extension(image_format_from_path(path).unwrap_or(ImageFormat::Png))));
    std::fs::write(&temp, bytes)?;
    if let Err(error) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        if !path.exists() {
            return Err(error);
        }
    }
    Ok(())
}

fn remove_clip_file(path: Option<&str>) {
    if let Some(path) = path {
        let _ = std::fs::remove_file(path);
    }
}

fn today() -> String {
    crate::stats::today_str()
}

fn tags_for(
    conn: &Connection,
    table: &str,
    id_col: &str,
    id: &str,
) -> rusqlite::Result<Vec<String>> {
    // table/id_col 是固定的内部标识，不接收用户输入。
    let sql = format!("SELECT tag FROM {table} WHERE {id_col} = ?1 ORDER BY rowid");
    let mut stmt = conn.prepare(&sql)?;
    let values = stmt.query_map([id], |row| row.get(0))?.collect();
    values
}

fn normalize_tags(tags: &[String], max_tags: usize, max_len: usize) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let value = tag.trim();
        if value.is_empty() {
            continue;
        }
        let value = value.chars().take(max_len).collect::<String>();
        if !normalized.iter().any(|existing| existing == &value) {
            normalized.push(value);
        }
        if normalized.len() >= max_tags {
            break;
        }
    }
    normalized
}

fn load_store(path: &Path) -> rusqlite::Result<Store> {
    let conn = open_db(path)?;
    let mut notes = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, content, storage_path, created_at, updated_at FROM notes ORDER BY archived_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })?;
    for row in rows {
        let (id, db_content, storage_path, created_at, updated_at) = row?;
        let content = storage_path
            .as_deref()
            .and_then(|relative| std::fs::read_to_string(app_data_dir().join(relative)).ok())
            .unwrap_or(db_content);
        notes.push(Note {
            tags: tags_for(&conn, "note_tags", "note_id", &id)?,
            id,
            content,
            created_at,
            updated_at,
        });
    }
    drop(stmt);

    let mut todos = Vec::new();
    let mut stmt = conn.prepare("SELECT id, content, due_at, completed_at, status, remind_at, repeat_rule, priority, remark, parent_id, created_at, updated_at FROM todos ORDER BY due_at ASC, created_at ASC")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get(3)?,
            r.get::<_, String>(4)?,
            r.get(5)?,
            r.get(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, String>(8)?,
            r.get(9)?,
            r.get::<_, String>(10)?,
            r.get::<_, String>(11)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            text,
            due_at,
            completed_at,
            status,
            remind_at,
            repeat_rule,
            priority,
            remark,
            parent_id,
            created_at,
            updated_at,
        ) = row?;
        todos.push(TodoItem {
            id: id.clone(),
            text,
            done: status == "done",
            due_at,
            completed_at,
            remind_at,
            repeat_rule,
            priority: Priority::parse(&priority),
            tags: tags_for(&conn, "todo_tags", "todo_id", &id)?,
            remark,
            parent_id,
            created_at,
            updated_at,
        });
    }
    drop(stmt);

    let mut clips = Vec::new();
    let mut stmt = conn.prepare("SELECT id, content, kind, storage_path, captured_at, favorite FROM clips ORDER BY captured_at DESC LIMIT 500")?;
    let rows = stmt.query_map([], |r| {
        Ok(ClipItem {
            id: r.get(0)?,
            content: r.get(1)?,
            kind: r.get(2)?,
            storage_path: r.get(3)?,
            captured_at: r.get(4)?,
            favorite: r.get::<_, i64>(5)? != 0,
        })
    })?;
    for row in rows {
        clips.push(row?);
    }
    let draft = conn
        .query_row(
            "SELECT id, content FROM note_drafts ORDER BY updated_at DESC LIMIT 1",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    Ok(Store {
        notes,
        todos,
        clips,
        draft_id: draft.as_ref().map(|v| v.0.clone()),
        draft_content: draft.map(|v| v.1).unwrap_or_default(),
        db_path: Some(path.to_path_buf()),
        last_error: None,
    })
}

fn store(cx: &mut App) -> &mut Store {
    if !cx.has_global::<Store>() {
        init(cx);
    }
    cx.global_mut::<Store>()
}

pub fn init(cx: &mut App) {
    if cx.has_global::<Store>() {
        return;
    }
    let path = db_path();
    match load_store(&path) {
        Ok(mut value) => {
            value.db_path = Some(path);
            cx.set_global(value);
        }
        Err(error) => {
            cx.set_global(Store {
                db_path: Some(path),
                last_error: Some(error.to_string()),
                ..Default::default()
            });
        }
    }
}

pub fn error(cx: &mut App) -> Option<String> {
    store(cx).last_error.clone()
}

/// 将本地归档导出到应用数据目录的 exports 子目录，并通过临时文件替换保证写入完整。
pub fn export_archive(cx: &mut App, format: &str) -> Result<String, String> {
    let extension = match format {
        "md" | "txt" | "html" => format,
        _ => return Err("当前支持 Markdown、TXT 和 HTML 导出".into()),
    };
    let s = store(cx);
    let notes = s.notes.clone();
    let clips = s.clips.clone();
    let todos = s.todos.clone();
    let mut markdown = String::from("# Inkling 归档\n\n## 笔记\n\n");
    for note in notes {
        markdown.push_str(&format!(
            "### {}\n\n{}\n\n",
            display_timestamp(&note.created_at()),
            note.content()
        ));
        if !note.tags().is_empty() {
            markdown.push_str(&format!(
                "标签：{}\n\n",
                note.tags()
                    .iter()
                    .map(|tag| format!("#{tag}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
    }
    markdown.push_str("## 剪贴板\n\n");
    for clip in clips {
        markdown.push_str(&format!(
            "- [{}] {} {}\n",
            display_timestamp(&clip.captured_at()),
            clip.kind(),
            clip.content().replace('\n', " ")
        ));
    }
    markdown.push_str("\n## 待办\n\n");
    for todo in todos {
        let marker = if todo.done() { "x" } else { " " };
        markdown.push_str(&format!(
            "- [{}] {}（{}，优先级：{}）\n",
            marker,
            todo.text(),
            display_timestamp(&todo.due_at()),
            todo.priority().label()
        ));
        if !todo.remark().is_empty() {
            markdown.push_str(&format!("  - 备注：{}\n", todo.remark()));
        }
    }
    let output = match extension {
        "md" => markdown,
        "txt" => markdown
            .replace("# ", "")
            .replace("## ", "")
            .replace("### ", ""),
        "html" => format!(
            "<!doctype html><meta charset=\"utf-8\"><title>Inkling 归档</title><pre>{}</pre>",
            html_escape(&markdown)
        ),
        _ => unreachable!(),
    };
    let dir = app_data_dir().join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = now_secs();
    let path = dir.join(format!("inkling-{stamp}.{extension}"));
    let temp = path.with_extension(format!("{extension}.tmp"));
    std::fs::write(&temp, output).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &path).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
}
pub fn notes(cx: &mut App) -> Vec<Note> {
    store(cx).notes.clone()
}
pub fn todos(cx: &mut App) -> Vec<TodoItem> {
    store(cx).todos.clone()
}

fn repeat_step_seconds(rule: Option<&str>) -> Option<u64> {
    match rule {
        Some("daily") => Some(86_400),
        Some("weekly") => Some(604_800),
        _ => None,
    }
}

/// 取出当前到期且尚未触发的提醒。事件键由待办 ID 与触发时刻组成，保证重启、
/// 重复轮询和多个窗口不会重复弹出同一提醒。
pub fn take_due_reminders(cx: &mut App) -> Vec<Reminder> {
    let now = now_secs();
    let s = store(cx);
    let due = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let mut stmt = tx.prepare(
            "SELECT id, content, due_at, remind_at, repeat_rule FROM todos WHERE status='open'",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        let mut due = Vec::new();
        for (todo_id, text, due_at, remind_at, repeat_rule) in rows {
            let Some(due_seconds) = due_at.parse::<u64>().ok() else {
                continue;
            };
            let triggers = if let Some(ref remind_at) = remind_at {
                remind_at
                    .parse::<u64>()
                    .ok()
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                vec![
                    due_seconds.saturating_sub(1_800),
                    due_seconds.saturating_sub(300),
                    due_seconds,
                ]
            };
            for trigger in triggers {
                if trigger > now {
                    continue;
                }
                let trigger_at = trigger.to_string();
                let event_id = format!("{todo_id}:{trigger_at}");
                let inserted = tx.execute(
                    "INSERT OR IGNORE INTO reminder_events(id,todo_id,trigger_at,triggered_at) VALUES(?1,?2,?3,?4)",
                    params![event_id, todo_id, trigger_at, now.to_string()],
                )?;
                if inserted > 0 {
                    // 重复提醒只推进同一待办的下一次提醒时间，不复制待办。
                    if remind_at.is_some() {
                        if let Some(step) = repeat_step_seconds(repeat_rule.as_deref()) {
                            let next = trigger.saturating_add(step).to_string();
                            tx.execute(
                                "UPDATE todos SET remind_at=?1,updated_at=?2 WHERE id=?3 AND status='open' AND remind_at=?4",
                                params![next, now.to_string(), todo_id, trigger_at],
                            )?;
                        }
                    }
                    due.push(Reminder {
                        id: event_id,
                        todo_id: todo_id.clone(),
                        text: text.clone(),
                        trigger_at,
                    });
                }
            }
        }
        tx.commit()?;
        Ok(due)
    })
    .unwrap_or_default();
    for reminder in &due {
        if let Some(todo) = s.todos.iter_mut().find(|todo| todo.id() == reminder.todo_id()) {
            if let Some(step) = repeat_step_seconds(todo.repeat_rule().as_deref()) {
                todo.set_remind_at(Some(
                    reminder
                        .trigger_at()
                        .parse::<u64>()
                        .unwrap_or(now)
                        .saturating_add(step)
                        .to_string(),
                ));
                todo.set_updated_at(now.to_string());
            }
        }
    }
    due
}

/// 设置下一次提醒时间。仅允许未完成待办修改提醒，完成事项不会被重新唤醒。
pub fn set_todo_remind_at(cx: &mut App, id: &str, remind_at: Option<String>) -> bool {
    if let Some(value) = remind_at.as_ref() {
        if value.parse::<u64>().is_err() {
            return false;
        }
    }
    let s = store(cx);
    let now = now_string();
    let result = with_db(s, |conn| {
        let changed = conn.execute(
            "UPDATE todos SET remind_at=?1,updated_at=?2 WHERE id=?3 AND status='open'",
            params![remind_at, now, id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        // 手动设置/贪睡代表用户创建了新的提醒计划，旧实例不应阻止新计划触发。
        conn.execute("DELETE FROM reminder_events WHERE todo_id=?1", [id])?;
        Ok(())
    });
    if result.is_ok() {
        if let Some(todo) = s
            .todos
            .iter_mut()
            .find(|todo| todo.id() == id && !todo.done())
        {
            todo.set_remind_at(remind_at);
            todo.set_updated_at(now);
        }
        true
    } else {
        false
    }
}
pub fn clips(cx: &mut App) -> Vec<ClipItem> {
    let mut clips = store(cx).clips.clone();
    clips.sort_by(|a, b| {
        b.favorite()
            .cmp(&a.favorite())
            .then_with(|| b.captured_at().cmp(&a.captured_at()))
    });
    clips
}

/// 启动长期驻留期间的剪贴板保留策略清理。
///
/// `Session` 只在启动时执行一次，`Never` 不需要调度；当天和自定义天数
/// 每 15 分钟检查一次，覆盖跨天和应用长时间不重启的场景。
pub fn start_clip_retention_scheduler(cx: &mut App) {
    cx.spawn(async move |cx| loop {
        cx.update(|cx| {
            let retention = Settings::load().clip_retention();
            if !matches!(retention, ClipRetention::Never | ClipRetention::Session) {
                apply_clip_retention(cx, retention);
            }
        })
        .ok();
        cx.background_executor()
            .timer(Duration::from_secs(15 * 60))
            .await;
    })
    .detach();
}

/// 根据设置清理剪贴板历史。清理同时作用于 SQLite 和内存快照，避免重启前后看到不同数据。
pub fn apply_clip_retention(cx: &mut App, retention: ClipRetention) {
    let s = store(cx);
    let previous_paths = s
        .clips
        .iter()
        .filter_map(|clip| clip.storage_path().clone())
        .collect::<Vec<_>>();
    let now = now_secs();
    let today = today();
    let path = s.db_path.clone().unwrap_or_else(db_path);
    let result = open_db(&path).and_then(|conn| {
        let conn = conn;
        match retention {
            ClipRetention::Never => Ok(()),
            ClipRetention::Session => {
                conn.execute("DELETE FROM clips", [])?;
                Ok(())
            }
            ClipRetention::Today => {
                conn.execute("DELETE FROM clips WHERE date(datetime(CAST(captured_at AS INTEGER), 'unixepoch', 'localtime'))<>?1", [today.clone()])?;
                Ok(())
            }
            ClipRetention::Custom(days) => {
                let cutoff = now.saturating_sub(u64::from(days) * 86_400);
                conn.execute("DELETE FROM clips WHERE CAST(captured_at AS INTEGER) < ?1", [cutoff.to_string()])?;
                Ok(())
            }
        }
    });
    if let Err(error) = result {
        s.last_error = Some(error.to_string());
        return;
    }
    match retention {
        ClipRetention::Never => {}
        ClipRetention::Session => s.clips.clear(),
        ClipRetention::Today => s
            .clips
            .retain(|clip| clip_date(&clip.captured_at()) == today),
        ClipRetention::Custom(days) => {
            let cutoff = now.saturating_sub(u64::from(days) * 86_400).to_string();
            s.clips.retain(|clip| clip.captured_at() >= cutoff);
        }
    }
    let remaining = s
        .clips
        .iter()
        .filter_map(|clip| clip.storage_path().clone())
        .collect::<std::collections::HashSet<_>>();
    for path in previous_paths {
        if !remaining.contains(&path) {
            remove_clip_file(Some(&path));
        }
    }
}

fn clip_date(timestamp: &str) -> String {
    timestamp
        .parse::<i64>()
        .ok()
        .and_then(|seconds| chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0))
        .map(|value| value.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

pub fn draft(cx: &mut App) -> String {
    store(cx).draft_content.clone()
}

/// 返回指定日期的真实统计。笔记和剪贴板使用幂等活动聚合，待办按冻结口径计算：
/// 总数取计划完成日，完成数取实际完成日，逾期数按查询时刻派生。
pub fn daily_stat(cx: &mut App, date: &str) -> crate::stats::DayStat {
    daily_stats(cx, &[date.to_string()])
        .into_iter()
        .next()
        .unwrap_or_else(|| crate::stats::DayStat::from_counts(date, 0, 0, 0, 0, 0))
}

pub fn daily_stats(cx: &mut App, dates: &[String]) -> Vec<crate::stats::DayStat> {
    let s = store(cx);
    let path = s.db_path.clone().unwrap_or_else(db_path);
    let conn = match open_db(&path) {
        Ok(conn) => conn,
        Err(error) => {
            s.last_error = Some(error.to_string());
            return dates
                .iter()
                .map(|date| crate::stats::DayStat::from_counts(date, 0, 0, 0, 0, 0))
                .collect();
        }
    };
    dates
        .iter()
        .map(|date| match query_daily_stat(&conn, date) {
            Ok(value) => value,
            Err(error) => {
                s.last_error = Some(error.to_string());
                crate::stats::DayStat::from_counts(date, 0, 0, 0, 0, 0)
            }
        })
        .collect()
}

fn query_daily_stat(conn: &Connection, date: &str) -> rusqlite::Result<crate::stats::DayStat> {
    let (notes, clips): (u32, u32) = conn.query_row(
        "SELECT COALESCE(note_archived_count, 0), COALESCE(clipboard_captured_count, 0) FROM stats_daily WHERE date=?1",
        [date],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?.unwrap_or((0, 0));
    let todos = conn.query_row(
        "SELECT COUNT(*) FROM todos WHERE date(datetime(CAST(due_at AS INTEGER), 'unixepoch', 'localtime'))=?1",
        [date],
        |row| row.get::<_, u32>(0),
    )?;
    let done = conn.query_row(
        "SELECT COUNT(*) FROM todos WHERE completed_at IS NOT NULL AND date(datetime(CAST(completed_at AS INTEGER), 'unixepoch', 'localtime'))=?1",
        [date],
        |row| row.get::<_, u32>(0),
    )?;
    let overdue = conn.query_row(
        "SELECT COUNT(*) FROM todos WHERE status='open' AND CAST(due_at AS INTEGER) < CAST(strftime('%s','now') AS INTEGER) AND date(datetime(CAST(due_at AS INTEGER), 'unixepoch', 'localtime'))=?1",
        [date],
        |row| row.get::<_, u32>(0),
    )?;
    Ok(crate::stats::DayStat::from_counts(
        date, notes, clips, todos, done, overdue,
    ))
}

fn with_db<F, T>(s: &mut Store, f: F) -> rusqlite::Result<T>
where
    F: FnOnce(&mut Connection) -> rusqlite::Result<T>,
{
    let path = s.db_path.clone().unwrap_or_else(db_path);
    let mut conn = open_db(&path)?;
    let result = f(&mut conn);
    if let Err(ref error) = result {
        s.last_error = Some(error.to_string());
    }
    result
}

fn record_activity(conn: &Connection, event_type: &str, entity_id: &str) -> rusqlite::Result<()> {
    let event_id = format!("{event_type}:{entity_id}");
    conn.execute("INSERT OR IGNORE INTO activity_events(id,event_type,entity_id,occurred_at) VALUES(?1,?2,?3,?4)", params![event_id, event_type, entity_id, now_string()])?;
    let column = match event_type {
        "note_archived" => "note_archived_count",
        "clipboard_captured" => "clipboard_captured_count",
        "todo_created" => "todo_created_count",
        "todo_completed" => "todo_completed_count",
        _ => return Ok(()),
    };
    let sql = format!("INSERT INTO stats_daily(date,{column},updated_at) VALUES(?1,1,?2) ON CONFLICT(date) DO UPDATE SET {column}={column}+1, updated_at=excluded.updated_at");
    conn.execute(&sql, params![today(), now_string()])?;
    Ok(())
}

pub fn save_draft(cx: &mut App, content: String) {
    let s = store(cx);
    let draft_id = s.draft_id.clone().unwrap_or_else(|| id("draft"));
    if with_db(s, |conn| { conn.execute("INSERT INTO note_drafts(id,content,updated_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET content=excluded.content,updated_at=excluded.updated_at", params![draft_id, content, now_string()])?; Ok(()) }).is_ok() {
        s.draft_id = Some(draft_id);
        s.draft_content = content;
    }
}

pub fn add_note(cx: &mut App, content: String) {
    add_note_with_tags(cx, content, Vec::new());
}

pub fn add_note_with_tags(cx: &mut App, content: String, tags: Vec<String>) {
    let content = content.trim().to_string();
    if content.is_empty() {
        return;
    }
    let s = store(cx);
    let note_id = id("note");
    let now = now_string();
    let large_note = content.len() > 1_048_576;
    let relative_path = large_note.then(|| format!("notes/{note_id}.md"));
    let absolute_path = relative_path
        .as_deref()
        .map(|relative| app_data_dir().join(relative));

    // 大笔记先以临时文件原子落盘，数据库事务失败时清理，避免出现“索引存在但正文丢失”。
    if let Some(path) = absolute_path.as_ref() {
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                s.last_error = Some("无法创建大笔记存储目录".into());
                return;
            }
        }
        let tmp = path.with_extension("md.tmp");
        if std::fs::write(&tmp, &content).is_err() || std::fs::rename(&tmp, path).is_err() {
            let _ = std::fs::remove_file(&tmp);
            s.last_error = Some("大笔记落盘失败".into());
            return;
        }
    }
    let db_content = if large_note {
        String::new()
    } else {
        content.clone()
    };
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO notes(id,content,storage_path,created_at,updated_at,archived_at) VALUES(?1,?2,?3,?4,?4,?4)",
            params![note_id, db_content, relative_path, now],
        )?;
        for tag in normalize_tags(&tags, 3, 5) {
            tx.execute(
                "INSERT OR IGNORE INTO note_tags(note_id,tag) VALUES(?1,?2)",
                params![note_id, tag],
            )?;
        }
        record_activity(&tx, "note_archived", &note_id)?;
        tx.execute("DELETE FROM note_drafts", [])?;
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        s.notes.insert(
            0,
            Note {
                id: note_id,
                content,
                tags: normalize_tags(&tags, 3, 5),
                created_at: now.clone(),
                updated_at: now,
            },
        );
        s.draft_id = None;
        s.draft_content.clear();
    } else if let Some(path) = absolute_path {
        let _ = std::fs::remove_file(path);
    }
}

pub fn update_note(cx: &mut App, note_id: &str, content: String, tags: Vec<String>) -> bool {
    let content = content.trim().to_string();
    if content.is_empty() {
        return false;
    }
    let tags = normalize_tags(&tags, 3, 5);
    let s = store(cx);
    let now = now_string();
    let old_storage_path = with_db(s, |conn| {
        let value = conn
            .query_row(
                "SELECT storage_path FROM notes WHERE id=?1",
                [note_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?;
        Ok(value.flatten())
    })
    .ok()
    .flatten();
    if old_storage_path.is_none() && !s.notes.iter().any(|note| note.id() == note_id) {
        return false;
    }

    let large_note = content.len() > 1_048_576;
    let new_storage_path =
        large_note.then(|| format!("notes/{note_id}-{}.md", crate::store::id("note-revision")));
    let new_absolute_path = new_storage_path
        .as_deref()
        .map(|relative| app_data_dir().join(relative));
    if let Some(path) = new_absolute_path.as_ref() {
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                s.last_error = Some("无法创建大笔记存储目录".into());
                return false;
            }
        }
        let tmp = path.with_extension("md.tmp");
        if std::fs::write(&tmp, &content).is_err() || std::fs::rename(&tmp, path).is_err() {
            let _ = std::fs::remove_file(&tmp);
            s.last_error = Some("大笔记落盘失败".into());
            return false;
        }
    }
    let db_content = if large_note {
        String::new()
    } else {
        content.clone()
    };
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE notes SET content=?1,storage_path=?2,updated_at=?3 WHERE id=?4",
            params![db_content, new_storage_path, now, note_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.execute("DELETE FROM note_tags WHERE note_id=?1", [note_id])?;
        for tag in &tags {
            tx.execute(
                "INSERT OR IGNORE INTO note_tags(note_id,tag) VALUES(?1,?2)",
                params![note_id, tag],
            )?;
        }
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        if let Some(note) = s.notes.iter_mut().find(|note| note.id() == note_id) {
            note.set_content(content);
            note.set_tags(tags);
            note.set_updated_at(now);
        }
        if let Some(old_path) = old_storage_path {
            if Some(old_path.clone()) != new_storage_path {
                let _ = std::fs::remove_file(app_data_dir().join(old_path));
            }
        }
        true
    } else {
        if let Some(path) = new_absolute_path {
            let _ = std::fs::remove_file(path);
        }
        false
    }
}

fn classify_clip(content: &str) -> &'static str {
    let trimmed = content.trim();
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        "link"
    } else if trimmed.contains("fn ")
        || trimmed.contains("=>")
        || trimmed.contains("<div")
        || trimmed.contains("</")
        || (trimmed.contains(
            "{
",
        ) && trimmed.contains("}"))
    {
        "code"
    } else {
        "text"
    }
}

pub fn push_clip(cx: &mut App, content: String) {
    if content.trim().is_empty() {
        return;
    }
    let s = store(cx);
    let hash = hash_content(&content);
    let captured = now_string();
    let kind = classify_clip(&content).to_string();
    let clip_id = id("clip");
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM clips WHERE content_hash=?1",
                [hash.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            tx.execute(
                "UPDATE clips SET kind=?1,captured_at=?2 WHERE id=?3",
                params![kind, captured, existing],
            )?;
        } else {
            tx.execute("INSERT INTO clips(id,content,kind,content_hash,captured_at) VALUES(?1,?2,?3,?4,?5)", params![clip_id, content, kind, hash, captured])?;
            record_activity(&tx, "clipboard_captured", &clip_id)?;
        }
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        if let Some(pos) = s.clips.iter().position(|c| c.content() == content) {
            let mut item = s.clips.remove(pos);
            item.set_captured_at(captured);
            item.set_kind(kind);
            s.clips.insert(0, item);
        } else {
            s.clips.insert(
                0,
                ClipItem {
                    id: clip_id,
                    content,
                    kind: kind.into(),
                    storage_path: None,
                    captured_at: captured,
                    favorite: false,
                },
            );
        }
        s.clips.truncate(500);
    }
}

/// 持久化剪贴板图片。图片文件使用内容哈希命名，数据库只保存元数据和文件路径。
pub fn push_clip_image(cx: &mut App, image: Image) {
    const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
    let s = store(cx);
    if image.bytes.is_empty() {
        s.last_error = Some("剪贴板图片为空，已忽略".into());
        return;
    }
    if image.bytes.len() > MAX_IMAGE_BYTES {
        s.last_error = Some("剪贴板图片超过 20 MiB，已忽略".into());
        return;
    }
    let hash = hash_bytes(&image.bytes);
    let captured = now_string();
    let clip_id = id("clip");
    let image_path = clip_image_path(&hash, image.format);
    let storage_path = image_path.to_string_lossy().to_string();
    if let Err(error) = write_binary_atomic(&image_path, &image.bytes) {
        s.last_error = Some(format!("保存剪贴板图片失败：{error}"));
        return;
    }

    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM clips WHERE content_hash=?1",
                [hash.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let actual_id = if let Some(existing) = existing {
            tx.execute(
                "UPDATE clips SET content='[图片]',kind='image',storage_path=?1,captured_at=?2 WHERE id=?3",
                params![storage_path, captured, existing],
            )?;
            existing
        } else {
            tx.execute(
                "INSERT INTO clips(id,content,kind,storage_path,content_hash,captured_at) VALUES(?1,'[图片]','image',?2,?3,?4)",
                params![clip_id, storage_path, hash, captured],
            )?;
            record_activity(&tx, "clipboard_captured", &clip_id)?;
            clip_id.clone()
        };
        tx.commit()?;
        Ok(actual_id)
    });

    match result {
        Ok(actual_id) => {
            if let Some(position) = s.clips.iter().position(|clip| clip.id() == actual_id) {
                let mut item = s.clips.remove(position);
                item.set_content("[图片]".to_string());
                item.set_kind("image".to_string());
                item.set_storage_path(Some(storage_path));
                item.set_captured_at(captured);
                s.clips.insert(0, item);
            } else {
                s.clips.insert(
                    0,
                    ClipItem {
                        id: actual_id,
                        content: "[图片]".to_string(),
                        kind: "image".to_string(),
                        storage_path: Some(storage_path),
                        captured_at: captured,
                        favorite: false,
                    },
                );
            }
            s.clips.truncate(500);
        }
        Err(_) => {
            // with_db 已记录错误；文件按哈希复用，保留后可在下一次捕获时直接复用。
        }
    }
}

pub fn load_clip_image(clip: &ClipItem) -> Option<Image> {
    if clip.kind() != "image" {
        return None;
    }
    let path = PathBuf::from(clip.storage_path().as_ref()?);
    let format = image_format_from_path(&path)?;
    let bytes = std::fs::read(path).ok()?;
    Some(Image::from_bytes(format, bytes))
}

pub fn set_clip_favorite(cx: &mut App, id: &str, favorite: bool) -> bool {
    let s = store(cx);
    let result = with_db(s, |conn| {
        let changed = conn.execute(
            "UPDATE clips SET favorite=?1 WHERE id=?2",
            params![favorite as i64, id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    });
    if result.is_ok() {
        if let Some(clip) = s.clips.iter_mut().find(|clip| clip.id() == id) {
            clip.set_favorite(favorite);
        }
        true
    } else {
        false
    }
}

pub fn update_clip_content(cx: &mut App, id: &str, content: String) -> bool {
    let content = content.trim().to_string();
    if content.is_empty() {
        return false;
    }
    let hash = hash_content(&content);
    let kind = classify_clip(&content).to_string();
    let captured = now_string();
    let s = store(cx);
    let old_storage_path = s
        .clips
        .iter()
        .find(|clip| clip.id() == id)
        .and_then(|clip| clip.storage_path().clone());
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let duplicate: Option<String> = tx
            .query_row(
                "SELECT id FROM clips WHERE content_hash=?1 AND id<>?2",
                params![hash, id],
                |row| row.get(0),
            )
            .optional()?;
        if duplicate.is_some() {
            return Err(rusqlite::Error::InvalidParameterName(
                "duplicate clipboard content".into(),
            ));
        }
        let changed = tx.execute(
            "UPDATE clips SET content=?1,kind=?2,storage_path=NULL,content_hash=?3,captured_at=?4 WHERE id=?5",
            params![content, kind, hash, captured, id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        if let Some(clip) = s.clips.iter_mut().find(|clip| clip.id() == id) {
            clip.set_content(content);
            clip.set_kind(kind);
            clip.set_storage_path(None);
            clip.set_captured_at(captured);
        }
        remove_clip_file(old_storage_path.as_deref());
        true
    } else {
        false
    }
}

pub fn toggle_todo(cx: &mut App, index: usize) -> bool {
    let id = store(cx).todos.get(index).map(|todo| todo.id().clone());
    id.map(|id| complete_todo(cx, &id)).unwrap_or(false)
}

pub fn complete_todo(cx: &mut App, id: &str) -> bool {
    let s = store(cx);
    let Some(item) = s.todos.iter().find(|item| item.id() == id).cloned() else {
        return false;
    };
    // 完成事项不可逆，避免 UI 重排或重复点击产生“取消完成”语义。
    if item.done() {
        return false;
    }
    let completed = now_string();
    let mut completed_ids = vec![item.id().clone()];
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE todos SET status='done',completed_at=?1,updated_at=?1 WHERE id=?2 AND status='open'",
            params![completed, item.id()],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        record_activity(&tx, "todo_completed", &item.id())?;

        // 子任务全部完成后，沿父级链自动完成父待办；整个状态转移与子任务完成处于同一事务。
        let mut parent = item.parent_id().clone();
        while let Some(parent_id) = parent {
            let has_open_child: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM todos WHERE parent_id=?1 AND status='open')",
                [&parent_id],
                |row| row.get(0),
            )?;
            if has_open_child {
                break;
            }
            let parent_changed = tx.execute(
                "UPDATE todos SET status='done',completed_at=?1,updated_at=?1 WHERE id=?2 AND status='open'",
                params![completed, parent_id],
            )?;
            if parent_changed == 0 {
                break;
            }
            record_activity(&tx, "todo_completed", &parent_id)?;
            completed_ids.push(parent_id.clone());
            parent = tx
                .query_row(
                    "SELECT parent_id FROM todos WHERE id=?1",
                    [&parent_id],
                    |row| row.get(0),
                )
                .optional()?;
        }
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        for completed_id in completed_ids {
            if let Some(value) = s.todos.iter_mut().find(|value| value.id() == completed_id) {
                value.set_done(true);
                value.set_completed_at(Some(completed.clone()));
                value.set_updated_at(completed.clone());
            }
        }
        true
    } else {
        false
    }
}

pub fn add_todo(
    cx: &mut App,
    text: String,
    due_at: String,
    parent_id: Option<String>,
) -> Option<String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return None;
    }
    let Some(due_seconds) = parse_timestamp(&due_at).and_then(|value| value.parse::<u64>().ok()) else {
        store(cx).last_error = Some("计划完成时间格式无效，请使用 YYYY-MM-DD HH:MM".into());
        return None;
    };
    let due_at = due_seconds.to_string();
    let s = store(cx);
    let todo_id = id("todo");
    let now = now_string();
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        if due_seconds < now_secs() {
            return Err(rusqlite::Error::InvalidParameterName(
                "todo due_at must not be in the past".into(),
            ));
        }
        // 子任务最多 5 个且只能挂在顶级待办下；开放父级要求子任务不晚于父级。
        if let Some(parent) = parent_id.as_ref() {
            let parent_meta: Option<(String, Option<String>)> = tx
                .query_row(
                    "SELECT status,parent_id FROM todos WHERE id=?1",
                    [parent],
                    |row| Ok((row.get(0)?, row.get::<_, Option<String>>(1)?)),
                )
                .optional()?;
            let Some((parent_status, grandparent)) = parent_meta else {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            };
            if grandparent.is_some() {
                return Err(rusqlite::Error::InvalidParameterName(
                    "nested child tasks are not allowed".into(),
                ));
            }
            let child_count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM todos WHERE parent_id=?1",
                [parent],
                |row| row.get(0),
            )?;
            if child_count >= 5 {
                return Err(rusqlite::Error::InvalidParameterName(
                    "a todo can have at most five children".into(),
                ));
            }
            if parent_status == "open" {
                let parent_due: String =
                    tx.query_row("SELECT due_at FROM todos WHERE id=?1", [parent], |row| {
                        row.get(0)
                    })?;
                if due_at.parse::<u64>().ok() > parent_due.parse::<u64>().ok() {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "child due_at cannot exceed parent due_at".into(),
                    ));
                }
            }
        }
        // 子任务使用自己的计划完成时间；父级仅顺延到 max(原父级时间, 新子任务时间)。
        tx.execute("INSERT INTO todos(id,content,due_at,status,priority,parent_id,created_at,updated_at) VALUES(?1,?2,?3,'open','medium',?4,?5,?5)", params![todo_id, text, due_at, parent_id, now])?;
        if let Some(parent) = parent_id.as_ref() {
            tx.execute("UPDATE todos SET status='open',completed_at=NULL,due_at=CASE WHEN due_at<?1 THEN ?1 ELSE due_at END,updated_at=?2 WHERE id=?3", params![due_at, now, parent])?;
        }
        record_activity(&tx, "todo_created", &todo_id)?;
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        s.todos.push(TodoItem {
            id: todo_id.clone(),
            text,
            due_at,
            done: false,
            completed_at: None,
            remind_at: None,
            repeat_rule: None,
            priority: Priority::Medium,
            tags: vec![],
            remark: String::new(),
            parent_id,
            created_at: now.clone(),
            updated_at: now,
        });
        Some(todo_id)
    } else {
        None
    }
}

pub fn set_priority(cx: &mut App, id: &str, priority: Priority) -> bool {
    let s = store(cx);
    let now = now_string();
    if with_db(s, |conn| {
        let changed = conn.execute(
            "UPDATE todos SET priority=?1,updated_at=?2 WHERE id=?3 AND status='open'",
            params![priority.as_str(), now, id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    })
    .is_ok()
    {
        if let Some(todo) = s
            .todos
            .iter_mut()
            .find(|todo| todo.id() == id && !todo.done())
        {
            todo.set_priority(priority);
            todo.set_updated_at(now);
        }
        true
    } else {
        false
    }
}

pub fn delete_note(cx: &mut App, id: &str) -> bool {
    let s = store(cx);
    let storage_path: Option<String> = {
        let path = s.db_path.clone().unwrap_or_else(db_path);
        open_db(&path).ok().and_then(|conn| {
            conn.query_row("SELECT storage_path FROM notes WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()
            .ok()
            .flatten()
        })
    };
    let deleted = with_db(s, |conn| {
        let changed = conn.execute("DELETE FROM notes WHERE id=?1", [id])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    });
    if deleted.is_ok() {
        if let Some(relative) = storage_path {
            let _ = std::fs::remove_file(app_data_dir().join(relative));
        }
        s.notes.retain(|note| note.id() != id);
        true
    } else {
        false
    }
}

pub fn delete_clip(cx: &mut App, id: &str) -> bool {
    let s = store(cx);
    let storage_path = s
        .clips
        .iter()
        .find(|clip| clip.id() == id)
        .and_then(|clip| clip.storage_path().clone());
    let deleted = with_db(s, |conn| {
        let changed = conn.execute("DELETE FROM clips WHERE id=?1", [id])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    });
    if deleted.is_ok() {
        s.clips.retain(|clip| clip.id() != id);
        remove_clip_file(storage_path.as_deref());
        true
    } else {
        false
    }
}

/// 删除待办及其全部子任务。数据库外键负责级联，内存快照同步移除整棵子树。
pub fn delete_todo(cx: &mut App, id: &str) -> bool {
    let s = store(cx);
    let deleted = with_db(s, |conn| {
        let changed = conn.execute("DELETE FROM todos WHERE id=?1", [id])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    });
    if deleted.is_ok() {
        let mut removed = std::collections::HashSet::from([id.to_string()]);
        loop {
            let before = removed.len();
            for todo in &s.todos {
                if todo
                    .parent_id()
                    .as_ref()
                    .is_some_and(|parent| removed.contains(parent))
                {
                    removed.insert(todo.id().clone());
                }
            }
            if removed.len() == before {
                break;
            }
        }
        s.todos.retain(|todo| !removed.contains(&todo.id()));
        true
    } else {
        false
    }
}

pub fn update_todo_text(cx: &mut App, id: &str, text: String) -> bool {
    let text = text.trim().to_string();
    if text.is_empty() {
        return false;
    }
    let s = store(cx);
    let now = now_string();
    let result = with_db(s, |conn| {
        let changed = conn.execute(
            "UPDATE todos SET content=?1,updated_at=?2 WHERE id=?3 AND status='open'",
            params![text, now, id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    });
    if result.is_ok() {
        if let Some(todo) = s
            .todos
            .iter_mut()
            .find(|todo| todo.id() == id && !todo.done())
        {
            todo.set_text(text);
            todo.set_updated_at(now);
        }
        true
    } else {
        false
    }
}

pub fn update_todo(cx: &mut App, item: &TodoItem) -> bool {
    if item.done() || item.text().trim().is_empty() {
        return false;
    }
    let Some(due_at) = parse_timestamp(&item.due_at()) else {
        return false;
    };
    let remind_at = match item.remind_at().as_ref() {
        Some(value) => {
            let Some(value) = parse_timestamp(value) else {
                return false;
            };
            Some(value)
        }
        None => None,
    };
    if item.repeat_rule().as_deref().is_some_and(|value| value != "daily" && value != "weekly") {
        return false;
    }
    let s = store(cx);
    let now = now_string();
    let tags = normalize_tags(&item.tags(), 3, 10);
    let remark = item.remark().chars().take(200).collect::<String>();
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        if let Some(parent_id) = item.parent_id().as_ref() {
            let parent_due: String =
                tx.query_row("SELECT due_at FROM todos WHERE id=?1", [parent_id], |row| {
                    row.get(0)
                })?;
            if item.due_at().parse::<u64>().unwrap_or(u64::MAX)
                > parent_due.parse::<u64>().unwrap_or(0)
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "child due_at cannot exceed parent due_at".into(),
                ));
            }
        }
        let changed = tx.execute("UPDATE todos SET content=?1,due_at=?2,remind_at=?3,repeat_rule=?4,priority=?5,remark=?6,updated_at=?7 WHERE id=?8 AND status='open'", params![item.text(), due_at, remind_at, item.repeat_rule(), item.priority().as_str(), remark, now, item.id()])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.execute(
            "DELETE FROM reminder_events WHERE todo_id=?1 AND CAST(trigger_at AS INTEGER) >= CAST(?2 AS INTEGER)",
            params![item.id(), now_secs().to_string()],
        )?;
        tx.execute("DELETE FROM todo_tags WHERE todo_id=?1", [item.id()])?;
        for tag in &tags {
            tx.execute(
                "INSERT OR IGNORE INTO todo_tags(todo_id,tag) VALUES(?1,?2)",
                params![item.id(), tag],
            )?;
        }
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        if let Some(todo) = s.todos.iter_mut().find(|todo| todo.id() == item.id()) {
            let mut copy = item.clone();
            copy.set_due_at(due_at);
            copy.set_remind_at(remind_at);
            copy.set_tags(tags);
            copy.set_remark(remark);
            copy.set_updated_at(now);
            *todo = copy;
        }
        true
    } else {
        false
    }
}

pub fn note_by_id(cx: &mut App, id: &str) -> Option<Note> {
    store(cx).notes.iter().find(|note| note.id() == id).cloned()
}
pub fn todo_children(cx: &mut App, parent_id: &str) -> Vec<TodoItem> {
    store(cx)
        .todos
        .iter()
        .filter(|todo| todo.parent_id().as_deref() == Some(parent_id))
        .cloned()
        .collect()
}
pub fn is_overdue(todo: &TodoItem) -> bool {
    !todo.done() && todo.due_at() < now_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_trimmed_deduplicated_and_bounded() {
        let tags = vec![
            "  rust  ".into(),
            "gpui".into(),
            "rust".into(),
            "sqlite-long".into(),
            "fourth".into(),
        ];
        assert_eq!(normalize_tags(&tags, 3, 5), vec!["rust", "gpui", "sqlit"]);
    }

    #[test]
    fn clipboard_classification_handles_links_and_code() {
        assert_eq!(classify_clip("https://example.com"), "link");
        assert_eq!(classify_clip("fn main() {\n}"), "code");
        assert_eq!(classify_clip("普通文本"), "text");
    }

    #[test]
    fn html_export_escapes_markup() {
        assert_eq!(html_escape("<&>\""), "&lt;&amp;&gt;&quot;");
    }
}
