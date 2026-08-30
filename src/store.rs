//! 持久化数据层：笔记、剪贴板和待办统一存储在应用数据目录 SQLite。
//!
//! UI 只通过本模块读写业务数据。写操作先完成 SQLite 事务，再刷新内存快照，
//! 从而保证主窗口与顶部呼出面板看到同一份数据。

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use gpui::{App, Global};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::settings::ClipRetention;

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
 content_hash TEXT NOT NULL UNIQUE, captured_at TEXT NOT NULL, favorite INTEGER NOT NULL DEFAULT 0
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
    (now_secs() + 86_400).to_string()
}

/// 将内部 Unix 秒时间戳转换为稳定、可读的本地日期时间文本。
pub fn display_timestamp(timestamp: &str) -> String {
    let Ok(seconds) = timestamp.parse::<i64>() else {
        return timestamp.to_string();
    };
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let hour = day_seconds / 3_600;
    let minute = (day_seconds % 3_600) / 60;
    format!(
        "{} {:02}:{:02}",
        crate::stats::civil_from_days(days),
        hour,
        minute
    )
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
    let mut stmt = conn.prepare("SELECT id, content, kind, captured_at, favorite FROM clips ORDER BY captured_at DESC LIMIT 500")?;
    let rows = stmt.query_map([], |r| {
        Ok(ClipItem {
            id: r.get(0)?,
            content: r.get(1)?,
            kind: r.get(2)?,
            captured_at: r.get(3)?,
            favorite: r.get::<_, i64>(4)? != 0,
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
pub fn clips(cx: &mut App) -> Vec<ClipItem> {
    let mut clips = store(cx).clips.clone();
    clips.sort_by(|a, b| {
        b.favorite()
            .cmp(&a.favorite())
            .then_with(|| b.captured_at().cmp(&a.captured_at()))
    });
    clips
}

/// 根据设置清理剪贴板历史。清理同时作用于 SQLite 和内存快照，避免重启前后看到不同数据。
pub fn apply_clip_retention(cx: &mut App, retention: ClipRetention) {
    let s = store(cx);
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
}

fn clip_date(timestamp: &str) -> String {
    timestamp
        .parse::<i64>()
        .ok()
        .map(|seconds| crate::stats::civil_from_days(seconds.div_euclid(86_400)))
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

fn with_db<F>(s: &mut Store, f: F) -> rusqlite::Result<()>
where
    F: FnOnce(&mut Connection) -> rusqlite::Result<()>,
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

pub fn update_note(cx: &mut App, id: &str, content: String, tags: Vec<String>) -> bool {
    let content = content.trim().to_string();
    if content.is_empty() {
        return false;
    }
    let tags = normalize_tags(&tags, 3, 5);
    let s = store(cx);
    let now = now_string();
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE notes SET content=?1,updated_at=?2 WHERE id=?3 AND storage_path IS NULL",
            params![content, now, id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.execute("DELETE FROM note_tags WHERE note_id=?1", [id])?;
        for tag in &tags {
            tx.execute(
                "INSERT OR IGNORE INTO note_tags(note_id,tag) VALUES(?1,?2)",
                params![id, tag],
            )?;
        }
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        if let Some(note) = s.notes.iter_mut().find(|note| note.id() == id) {
            note.set_content(content);
            note.set_tags(tags);
            note.set_updated_at(now);
        }
        true
    } else {
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
                    captured_at: captured,
                    favorite: false,
                },
            );
        }
        s.clips.truncate(500);
    }
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
            "UPDATE clips SET content=?1,kind=?2,content_hash=?3,captured_at=?4 WHERE id=?5",
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
            clip.set_captured_at(captured);
        }
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
    let s = store(cx);
    let todo_id = id("todo");
    let now = now_string();
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let due_seconds = due_at.parse::<u64>().ok();
        if due_seconds.is_none() || due_seconds < Some(now_secs()) {
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
    let deleted = with_db(s, |conn| {
        let changed = conn.execute("DELETE FROM clips WHERE id=?1", [id])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    });
    if deleted.is_ok() {
        s.clips.retain(|clip| clip.id() != id);
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
    let s = store(cx);
    let now = now_string();
    let tags = normalize_tags(&item.tags(), 3, 10);
    let remark = item.remark().chars().take(200).collect::<String>();
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        let changed = tx.execute("UPDATE todos SET content=?1,due_at=?2,remind_at=?3,repeat_rule=?4,priority=?5,remark=?6,updated_at=?7 WHERE id=?8 AND status='open'", params![item.text(), item.due_at(), item.remind_at(), item.repeat_rule(), item.priority().as_str(), remark, now, item.id()])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
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
