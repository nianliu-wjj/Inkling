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

fn load_store(path: &Path) -> rusqlite::Result<Store> {
    let conn = open_db(path)?;
    let mut notes = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, content, created_at, updated_at FROM notes ORDER BY archived_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (id, content, created_at, updated_at) = row?;
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
pub fn notes(cx: &mut App) -> Vec<Note> {
    store(cx).notes.clone()
}
pub fn todos(cx: &mut App) -> Vec<TodoItem> {
    store(cx).todos.clone()
}
pub fn clips(cx: &mut App) -> Vec<ClipItem> {
    store(cx).clips.clone()
}
pub fn draft(cx: &mut App) -> String {
    store(cx).draft_content.clone()
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
    let content = content.trim().to_string();
    if content.is_empty() {
        return;
    }
    let s = store(cx);
    let note_id = id("note");
    let now = now_string();
    if with_db(s, |conn| { let tx = conn.transaction()?; tx.execute("INSERT INTO notes(id,content,created_at,updated_at,archived_at) VALUES(?1,?2,?3,?3,?3)", params![note_id, content, now])?; record_activity(&tx, "note_archived", &note_id)?; tx.execute("DELETE FROM note_drafts", [])?; tx.commit()?; Ok(()) }).is_ok() {
        s.notes.insert(0, Note { id: note_id, content, tags: vec![], created_at: now.clone(), updated_at: now }); s.draft_id = None; s.draft_content.clear();
    }
}

pub fn push_clip(cx: &mut App, content: String) {
    let content = content.trim().to_string();
    if content.is_empty() {
        return;
    }
    let s = store(cx);
    let hash = hash_content(&content);
    let captured = now_string();
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
                "UPDATE clips SET captured_at=?1 WHERE id=?2",
                params![captured, existing],
            )?;
        } else {
            tx.execute("INSERT INTO clips(id,content,kind,content_hash,captured_at) VALUES(?1,?2,'text',?3,?4)", params![clip_id, content, hash, captured])?;
            record_activity(&tx, "clipboard_captured", &clip_id)?;
        }
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        if let Some(pos) = s.clips.iter().position(|c| c.content() == content) {
            let mut item = s.clips.remove(pos);
            item.set_captured_at(captured);
            s.clips.insert(0, item);
        } else {
            s.clips.insert(
                0,
                ClipItem {
                    id: clip_id,
                    content,
                    kind: "text".into(),
                    captured_at: captured,
                    favorite: false,
                },
            );
        }
        s.clips.truncate(500);
    }
}

pub fn toggle_todo(cx: &mut App, index: usize) -> bool {
    let s = store(cx);
    let Some(item) = s.todos.get(index).cloned() else {
        return false;
    };
    if item.done() {
        return false;
    }
    let completed = now_string();
    let result = with_db(s, |conn| {
        let tx = conn.transaction()?;
        tx.execute("UPDATE todos SET status='done',completed_at=?1,updated_at=?1 WHERE id=?2 AND status='open'", params![completed, item.id()])?;
        record_activity(&tx, "todo_completed", &item.id())?;
        tx.commit()?;
        Ok(())
    });
    if result.is_ok() {
        if let Some(value) = s.todos.iter_mut().find(|value| value.id() == item.id()) {
            value.set_done(true);
            value.set_completed_at(Some(completed.clone()));
            value.set_updated_at(completed);
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
        let parent_due: Option<String> = parent_id.as_ref().and_then(|parent| {
            tx.query_row("SELECT due_at FROM todos WHERE id=?1", [parent], |r| {
                r.get(0)
            })
            .optional()
            .ok()
            .flatten()
        });
        let final_due = parent_due
            .map(|parent| {
                if parent > due_at {
                    parent
                } else {
                    due_at.clone()
                }
            })
            .unwrap_or_else(|| due_at.clone());
        tx.execute("INSERT INTO todos(id,content,due_at,status,priority,parent_id,created_at,updated_at) VALUES(?1,?2,?3,'open','medium',?4,?5,?5)", params![todo_id, text, final_due, parent_id, now])?;
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
        conn.execute(
            "UPDATE todos SET priority=?1,updated_at=?2 WHERE id=?3 AND status='open'",
            params![priority.as_str(), now, id],
        )?;
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

pub fn update_todo(cx: &mut App, item: &TodoItem) -> bool {
    if item.done() {
        return false;
    }
    let s = store(cx);
    let now = now_string();
    if with_db(s, |conn| { let tx = conn.transaction()?; tx.execute("UPDATE todos SET content=?1,due_at=?2,remind_at=?3,repeat_rule=?4,priority=?5,remark=?6,updated_at=?7 WHERE id=?8 AND status='open'", params![item.text(), item.due_at(), item.remind_at(), item.repeat_rule(), item.priority().as_str(), item.remark(), now, item.id()])?; tx.execute("DELETE FROM todo_tags WHERE todo_id=?1", [item.id()])?; for tag in item.tags() { tx.execute("INSERT OR IGNORE INTO todo_tags(todo_id,tag) VALUES(?1,?2)", params![item.id(), tag])?; } tx.commit()?; Ok(()) }).is_ok() { if let Some(todo) = s.todos.iter_mut().find(|todo| todo.id() == item.id()) { let mut copy = item.clone(); copy.set_updated_at(now); *todo = copy; } true } else { false }
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
