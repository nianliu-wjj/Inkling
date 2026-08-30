#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, State,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use uuid::Uuid;

const MAX_NOTE_BYTES: usize = 1_048_576;

struct AppState(Mutex<Store>);

struct Store {
    db: Connection,
    data_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Note {
    id: String,
    content: String,
    tags: Vec<String>,
    is_draft: bool,
    pinned: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClipboardEntry {
    id: String,
    content_type: String,
    content: String,
    preview: String,
    pinned: bool,
    copied_at: String,
    modified_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Todo {
    id: String,
    content: String,
    due_at: String,
    completed_at: Option<String>,
    status: String,
    remind_at: Option<String>,
    repeat_rule: Option<String>,
    priority: String,
    remark: String,
    parent_id: Option<String>,
    tags: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ActivityDay {
    date: String,
    notes: i64,
    clips: i64,
    todos: i64,
    completed: i64,
    overdue: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Settings {
    collapse_policy: String,
    clipboard_retention_days: i64,
    start_on_boot: bool,
    shortcut: String,
    remark_style: String,
    theme: String,
}

#[derive(Debug, Deserialize)]
struct NoteInput {
    id: Option<String>,
    content: String,
    tags: Vec<String>,
    draft: bool,
}

#[derive(Debug, Deserialize)]
struct TodoInput {
    id: Option<String>,
    content: String,
    due_at: String,
    remind_at: Option<String>,
    repeat_rule: Option<String>,
    priority: String,
    remark: String,
    tags: Vec<String>,
    parent_id: Option<String>,
}

impl Store {
    fn open(data_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(data_dir.join("notes"))
            .map_err(|e| format!("创建应用数据目录失败: {e}"))?;
        let db_path = data_dir.join("inkling.sqlite3");
        let db = Connection::open(db_path).map_err(|e| format!("打开数据库失败: {e}"))?;
        db.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS notes (
               id TEXT PRIMARY KEY,
               content TEXT,
               plain_text TEXT NOT NULL DEFAULT '',
               file_path TEXT,
               is_draft INTEGER NOT NULL DEFAULT 0,
               pinned INTEGER NOT NULL DEFAULT 0,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS tags (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               name TEXT NOT NULL,
               normalized TEXT NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS note_tags (
               note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
               tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
               PRIMARY KEY(note_id, tag_id)
             );
             CREATE TABLE IF NOT EXISTS clipboard_entries (
               id TEXT PRIMARY KEY,
               content_type TEXT NOT NULL,
               content TEXT NOT NULL DEFAULT '',
               preview TEXT NOT NULL DEFAULT '',
               content_hash TEXT NOT NULL UNIQUE,
               pinned INTEGER NOT NULL DEFAULT 0,
               copied_at TEXT NOT NULL,
               modified_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS todos (
               id TEXT PRIMARY KEY,
               content TEXT NOT NULL,
               due_at TEXT NOT NULL,
               completed_at TEXT,
               status TEXT NOT NULL DEFAULT 'open',
               remind_at TEXT,
               repeat_rule TEXT,
               priority TEXT NOT NULL DEFAULT 'medium',
               remark TEXT NOT NULL DEFAULT '',
               parent_id TEXT REFERENCES todos(id) ON DELETE CASCADE,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_todos_parent ON todos(parent_id);
             CREATE TABLE IF NOT EXISTS todo_tags (
               todo_id TEXT NOT NULL REFERENCES todos(id) ON DELETE CASCADE,
               tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
               PRIMARY KEY(todo_id, tag_id)
             );
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .map_err(|e| format!("初始化数据库失败: {e}"))?;
        Ok(Self { db, data_dir })
    }

    fn tags(&self, table: &str, id_column: &str, id: &str) -> Result<Vec<String>, String> {
        let sql = format!("SELECT t.name FROM tags t JOIN {table} x ON x.tag_id=t.id WHERE x.{id_column}=? ORDER BY t.name");
        let mut stmt = self.db.prepare(&sql).map_err(db_err)?;
        let result = stmt
            .query_map([id], |row| row.get(0))
            .map_err(db_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_err);
        result
    }

    fn replace_tags(
        &self,
        table: &str,
        id_column: &str,
        id: &str,
        tags: &[String],
    ) -> Result<(), String> {
        let delete_sql = format!("DELETE FROM {table} WHERE {id_column}=?");
        self.db.execute(&delete_sql, [id]).map_err(db_err)?;
        let insert_sql =
            format!("INSERT OR IGNORE INTO {table} ({id_column}, tag_id) VALUES (?, ?)");
        for raw in tags
            .iter()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .take(20)
        {
            let normalized = raw.to_lowercase();
            self.db
                .execute(
                    "INSERT OR IGNORE INTO tags(name, normalized) VALUES(?, ?)",
                    params![raw, normalized],
                )
                .map_err(db_err)?;
            let tag_id: i64 = self
                .db
                .query_row(
                    "SELECT id FROM tags WHERE normalized=?",
                    [normalized],
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            self.db
                .execute(&insert_sql, params![id, tag_id])
                .map_err(db_err)?;
        }
        Ok(())
    }

    fn note(&self, id: &str) -> Result<Note, String> {
        self.db.query_row("SELECT id, content, file_path, is_draft, pinned, created_at, updated_at FROM notes WHERE id=?", [id], |r| {
            let content: Option<String> = r.get(1)?;
            let file_path: Option<String> = r.get(2)?;
            Ok((r.get::<_, String>(0)?, content.unwrap_or_default(), file_path, r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, String>(5)?, r.get::<_, String>(6)?))
        }).map_err(db_err).and_then(|(id, mut content, file_path, draft, pinned, created_at, updated_at)| {
            if content.is_empty() {
                if let Some(path) = file_path { content = fs::read_to_string(self.data_dir.join(path)).unwrap_or_default(); }
            }
            Ok(Note { id: id.clone(), content, tags: self.tags("note_tags", "note_id", id.as_str())?, is_draft: draft != 0, pinned: pinned != 0, created_at, updated_at })
        })
    }

    fn todo(&self, id: &str) -> Result<Todo, String> {
        self.db.query_row("SELECT id, content, due_at, completed_at, status, remind_at, repeat_rule, priority, remark, parent_id, created_at, updated_at FROM todos WHERE id=?", [id], |r| {
            Ok(Todo { id: r.get(0)?, content: r.get(1)?, due_at: r.get(2)?, completed_at: r.get(3)?, status: r.get(4)?, remind_at: r.get(5)?, repeat_rule: r.get(6)?, priority: r.get(7)?, remark: r.get(8)?, parent_id: r.get(9)?, tags: Vec::new(), created_at: r.get(10)?, updated_at: r.get(11)? })
        }).map_err(db_err).and_then(|mut todo| { todo.tags = self.tags("todo_tags", "todo_id", &todo.id)?; Ok(todo) })
    }
}

fn db_err(error: rusqlite::Error) -> String {
    format!("数据库操作失败: {error}")
}
fn now() -> String {
    Utc::now().to_rfc3339()
}
fn normalized_tags(tags: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() || tag.chars().count() > 20 {
            continue;
        }
        if !result.iter().any(|x: &String| x.eq_ignore_ascii_case(tag)) {
            result.push(tag.to_string());
        }
    }
    result
}
fn normalized_todo_tags(tags: &[String]) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    for raw in tags {
        let tag = raw.trim();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > 10 {
            return Err("待办标签最多 10 个字".into());
        }
        if !result
            .iter()
            .any(|item: &String| item.eq_ignore_ascii_case(tag))
        {
            result.push(tag.to_string());
        }
    }
    if result.len() > 3 {
        return Err("待办最多只能有 3 个标签".into());
    }
    Ok(result)
}

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
fn is_valid_priority(value: &str) -> bool {
    matches!(value, "high" | "medium" | "low")
}

#[tauri::command]
fn list_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let mut stmt = store
        .db
        .prepare("SELECT id FROM notes WHERE is_draft=0 ORDER BY pinned DESC, updated_at DESC")
        .map_err(db_err)?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err)?;
    ids.iter().map(|id| store.note(id)).collect()
}

#[tauri::command]
fn save_note(input: NoteInput, state: State<'_, AppState>) -> Result<Note, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let timestamp = now();
    let existing: Option<(String, String)> = store
        .db
        .query_row(
            "SELECT created_at, COALESCE(file_path, '') FROM notes WHERE id=?",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(db_err)?;
    let created_at = existing
        .as_ref()
        .map(|x| x.0.clone())
        .unwrap_or_else(|| timestamp.clone());
    let old_file_path = existing
        .as_ref()
        .and_then(|x| (!x.1.is_empty()).then_some(x.1.clone()));
    let tags = normalized_tags(&input.tags);
    let (content, file_path) = if input.content.len() > MAX_NOTE_BYTES && !input.draft {
        let relative = format!("notes/{id}.md");
        let target = store.data_dir.join(&relative);
        let temp = target.with_extension("md.tmp");
        fs::write(&temp, input.content.as_bytes()).map_err(|e| format!("写入大笔记失败: {e}"))?;
        if let Err(error) = fs::rename(&temp, &target) {
            if target.exists() {
                fs::remove_file(&target).map_err(|e| format!("替换大笔记失败: {e}"))?;
                fs::rename(&temp, &target)
                    .map_err(|e| format!("提交大笔记失败: {e}; 原始错误: {error}"))?;
            } else {
                return Err(format!("提交大笔记失败: {error}"));
            }
        }
        (String::new(), Some(relative))
    } else {
        (input.content.clone(), None)
    };
    store.db.execute("INSERT INTO notes(id, content, plain_text, file_path, is_draft, created_at, updated_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET content=excluded.content, plain_text=excluded.plain_text, file_path=excluded.file_path, is_draft=excluded.is_draft, updated_at=excluded.updated_at", params![id, content, input.content, file_path, input.draft as i64, created_at, timestamp]).map_err(db_err)?;
    store.replace_tags("note_tags", "note_id", &id, &tags)?;
    if let Some(old_path) = old_file_path {
        if file_path.as_deref() != Some(old_path.as_str()) {
            let _ = fs::remove_file(store.data_dir.join(old_path));
        }
    }
    store.note(&id)
}

#[tauri::command]
fn delete_note(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    if let Some(path) = store
        .db
        .query_row("SELECT file_path FROM notes WHERE id=?", [&id], |r| {
            r.get::<_, Option<String>>(0)
        })
        .optional()
        .map_err(db_err)?
        .flatten()
    {
        let _ = fs::remove_file(store.data_dir.join(path));
    }
    store
        .db
        .execute("DELETE FROM notes WHERE id=?", [&id])
        .map_err(db_err)?;
    Ok(())
}

#[tauri::command]
fn list_clipboard(state: State<'_, AppState>) -> Result<Vec<ClipboardEntry>, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let mut stmt = store.db.prepare("SELECT id, content_type, content, preview, pinned, copied_at, modified_at FROM clipboard_entries ORDER BY pinned DESC, modified_at DESC").map_err(db_err)?;
    let result = stmt
        .query_map([], |r| {
            Ok(ClipboardEntry {
                id: r.get(0)?,
                content_type: r.get(1)?,
                content: r.get(2)?,
                preview: r.get(3)?,
                pinned: r.get::<_, i64>(4)? != 0,
                copied_at: r.get(5)?,
                modified_at: r.get(6)?,
            })
        })
        .map_err(db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err);
    result
}

#[tauri::command]
fn save_clipboard(
    content: String,
    content_type: String,
    state: State<'_, AppState>,
) -> Result<ClipboardEntry, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let hash = hash_content(&content);
    let timestamp = now();
    if let Some(existing_id) = store
        .db
        .query_row(
            "SELECT id FROM clipboard_entries WHERE content_hash=?",
            [&hash],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(db_err)?
    {
        return store
            .db
            .query_row(
                "SELECT id, content_type, content, preview, pinned, copied_at, modified_at FROM clipboard_entries WHERE id=?",
                [&existing_id],
                |r| Ok(ClipboardEntry { id: r.get(0)?, content_type: r.get(1)?, content: r.get(2)?, preview: r.get(3)?, pinned: r.get::<_, i64>(4)? != 0, copied_at: r.get(5)?, modified_at: r.get(6)? }),
            )
            .map_err(db_err);
    }
    let id = Uuid::new_v4().to_string();
    store.db.execute(
        "INSERT INTO clipboard_entries(id, content_type, content, preview, content_hash, copied_at, modified_at) VALUES(?,?,?,?,?,?,?)",
        params![id, content_type, content, content.chars().take(240).collect::<String>(), hash, timestamp, timestamp],
    ).map_err(db_err)?;
    store.db.query_row(
        "SELECT id, content_type, content, preview, pinned, copied_at, modified_at FROM clipboard_entries WHERE id=?",
        [&id],
        |r| Ok(ClipboardEntry { id: r.get(0)?, content_type: r.get(1)?, content: r.get(2)?, preview: r.get(3)?, pinned: r.get::<_, i64>(4)? != 0, copied_at: r.get(5)?, modified_at: r.get(6)? }),
    ).map_err(db_err)
}

#[tauri::command]
fn update_clipboard(id: String, content: String, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let timestamp = now();
    store.db.execute("UPDATE clipboard_entries SET content=?, preview=?, content_hash=?, modified_at=? WHERE id=?", params![content, content.chars().take(240).collect::<String>(), hash_content(&content), timestamp, id]).map_err(db_err)?;
    Ok(())
}

#[tauri::command]
fn set_clipboard_pinned(
    id: String,
    pinned: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    store
        .db
        .execute(
            "UPDATE clipboard_entries SET pinned=? WHERE id=?",
            params![pinned as i64, id],
        )
        .map_err(db_err)?;
    Ok(())
}

#[tauri::command]
fn delete_clipboard(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    store
        .db
        .execute("DELETE FROM clipboard_entries WHERE id=?", [&id])
        .map_err(db_err)?;
    Ok(())
}

#[tauri::command]
fn list_todos(state: State<'_, AppState>) -> Result<Vec<Todo>, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let mut stmt = store.db.prepare("SELECT id FROM todos ORDER BY CASE status WHEN 'open' THEN 0 ELSE 1 END, due_at ASC, CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, created_at ASC").map_err(db_err)?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err)?;
    ids.iter().map(|id| store.todo(id)).collect()
}

fn validate_todo(store: &Store, input: &TodoInput, id: Option<&str>) -> Result<(), String> {
    if input.content.trim().is_empty() {
        return Err("待办内容不能为空".into());
    }
    if !is_valid_priority(&input.priority) {
        return Err("无效的优先级".into());
    }
    if input.remark.chars().count() > 200 {
        return Err("待办备注最多 200 个字".into());
    }
    normalized_todo_tags(&input.tags)?;
    if let Some(parent_id) = &input.parent_id {
        if id.is_some_and(|value| value == parent_id) {
            return Err("待办不能成为自己的子任务".into());
        }
        let parent = store.todo(parent_id)?;
        if parent.parent_id.is_some() {
            return Err("子任务不可继续嵌套".into());
        }
        let count: i64 = store
            .db
            .query_row(
                "SELECT COUNT(*) FROM todos WHERE parent_id=?",
                [parent_id],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        if id.is_none() && count >= 5 {
            return Err("一个顶级待办最多只能有 5 个子任务".into());
        }
        if parent.status == "open" && input.due_at > parent.due_at {
            return Err("普通场景下子任务截止时间不能晚于父待办".into());
        }
    }
    Ok(())
}

#[tauri::command]
fn save_todo(input: TodoInput, state: State<'_, AppState>) -> Result<Todo, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let old = store.todo(&id).ok();
    if old.as_ref().is_some_and(|x| x.status == "done") {
        return Err("已完成待办不可编辑".into());
    }
    validate_todo(&store, &input, Some(&id))?;
    let timestamp = now();
    let created_at = old
        .as_ref()
        .map(|x| x.created_at.clone())
        .unwrap_or_else(|| timestamp.clone());
    store.db.execute("INSERT INTO todos(id, content, due_at, remind_at, repeat_rule, priority, remark, parent_id, created_at, updated_at) VALUES(?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET content=excluded.content, due_at=excluded.due_at, remind_at=excluded.remind_at, repeat_rule=excluded.repeat_rule, priority=excluded.priority, remark=excluded.remark, parent_id=excluded.parent_id, updated_at=excluded.updated_at", params![id, input.content.trim(), input.due_at, input.remind_at, input.repeat_rule, input.priority, input.remark, input.parent_id, created_at, timestamp]).map_err(db_err)?;
    store.replace_tags(
        "todo_tags",
        "todo_id",
        &id,
        &normalized_todo_tags(&input.tags)?,
    )?;
    store.todo(&id)
}

#[tauri::command]
fn complete_todo(
    id: String,
    completed: bool,
    state: State<'_, AppState>,
) -> Result<Vec<Todo>, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let todo = store.todo(&id)?;
    if !completed && todo.status == "done" {
        return Err("已完成待办不可取消完成".into());
    }
    let completed_at = if completed { Some(now()) } else { None };
    store
        .db
        .execute(
            "UPDATE todos SET status=?, completed_at=?, updated_at=? WHERE id=?",
            params![
                if completed { "done" } else { "open" },
                completed_at,
                now(),
                id
            ],
        )
        .map_err(db_err)?;
    if completed {
        if let Some(parent_id) = todo.parent_id {
            let open_children: i64 = store
                .db
                .query_row(
                    "SELECT COUNT(*) FROM todos WHERE parent_id=? AND status='open'",
                    [&parent_id],
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            if open_children == 0 {
                store.db.execute("UPDATE todos SET status='done', completed_at=?, updated_at=? WHERE id=? AND status='open'", params![now(), now(), parent_id]).map_err(db_err)?;
            }
        }
    }
    let mut stmt = store.db.prepare("SELECT id FROM todos ORDER BY CASE status WHEN 'open' THEN 0 ELSE 1 END, due_at ASC, created_at ASC").map_err(db_err)?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err)?;
    ids.iter().map(|item| store.todo(item)).collect()
}

#[tauri::command]
fn create_child_todo(
    parent_id: String,
    content: String,
    due_at: String,
    priority: String,
    remark: String,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Todo, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let parent = store.todo(&parent_id)?;
    if content.trim().is_empty() {
        return Err("子任务内容不能为空".into());
    }
    if !is_valid_priority(&priority) {
        return Err("无效的优先级".into());
    }
    if remark.chars().count() > 200 {
        return Err("待办备注最多 200 个字".into());
    }
    let tags = normalized_todo_tags(&tags)?;
    let count: i64 = store
        .db
        .query_row(
            "SELECT COUNT(*) FROM todos WHERE parent_id=?",
            [&parent_id],
            |r| r.get(0),
        )
        .map_err(db_err)?;
    if count >= 5 {
        return Err("一个待办最多只能有 5 个子任务".into());
    }
    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    let due_at = if parent.status == "done" {
        due_at
    } else if due_at > parent.due_at {
        return Err("子任务截止时间不能晚于父待办".into());
    } else {
        due_at
    };
    let tx = store.db.unchecked_transaction().map_err(db_err)?;
    tx.execute("INSERT INTO todos(id, content, due_at, status, priority, remark, parent_id, created_at, updated_at) VALUES(?,?,?,?,?,?,?,?,?)", params![id, content.trim(), due_at, "open", priority, remark.trim(), parent_id, timestamp, timestamp]).map_err(db_err)?;
    tx.execute("UPDATE todos SET status='open', completed_at=NULL, due_at=CASE WHEN due_at < ? THEN ? ELSE due_at END, updated_at=? WHERE id=?", params![due_at, due_at, timestamp, parent_id]).map_err(db_err)?;
    let child_id = id.clone();
    tx.commit().map_err(db_err)?;
    store.replace_tags("todo_tags", "todo_id", &child_id, &tags)?;
    store.todo(&id)
}

#[tauri::command]
fn set_todo_priority(
    id: String,
    priority: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !is_valid_priority(&priority) {
        return Err("无效的优先级".into());
    }
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let todo = store.todo(&id)?;
    if todo.status == "done" {
        return Err("已完成待办不可变更优先级".into());
    }
    store
        .db
        .execute(
            "UPDATE todos SET priority=?, updated_at=? WHERE id=?",
            params![priority, now(), id],
        )
        .map_err(db_err)?;
    Ok(())
}

#[tauri::command]
fn delete_todo(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let todo = store.todo(&id)?;
    if todo.status == "done" {
        return Err("已完成待办不可删除".into());
    }
    store
        .db
        .execute("DELETE FROM todos WHERE id=?", [&id])
        .map_err(db_err)?;
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let mut values = std::collections::HashMap::new();
    let mut stmt = store
        .db
        .prepare("SELECT key, value FROM settings")
        .map_err(db_err)?;
    for row in stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db_err)?
    {
        let (key, value) = row.map_err(db_err)?;
        values.insert(key, value);
    }
    Ok(Settings {
        collapse_policy: values
            .get("collapse_policy")
            .cloned()
            .unwrap_or_else(|| "3s".into()),
        clipboard_retention_days: values
            .get("clipboard_retention_days")
            .and_then(|x| x.parse().ok())
            .unwrap_or(30),
        start_on_boot: values.get("start_on_boot").is_some_and(|x| x == "true"),
        shortcut: values
            .get("shortcut")
            .cloned()
            .unwrap_or_else(|| "Ctrl+Shift+Space".into()),
        remark_style: values
            .get("remark_style")
            .cloned()
            .unwrap_or_else(|| "mixed".into()),
        theme: values
            .get("theme")
            .cloned()
            .unwrap_or_else(|| "dark".into()),
    })
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    for (key, value) in [
        ("collapse_policy", settings.collapse_policy),
        (
            "clipboard_retention_days",
            settings.clipboard_retention_days.to_string(),
        ),
        ("start_on_boot", settings.start_on_boot.to_string()),
        ("shortcut", settings.shortcut),
        ("remark_style", settings.remark_style),
        ("theme", settings.theme),
    ] {
        store.db.execute("INSERT INTO settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value]).map_err(db_err)?;
    }
    Ok(())
}

#[tauri::command]
fn get_activity(state: State<'_, AppState>) -> Result<Vec<ActivityDay>, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let today = Utc::now().date_naive();
    let mut days = Vec::new();
    for offset in (0..180).rev() {
        let date = today - chrono::Days::new(offset);
        let date_str = date.to_string();
        let next = (date + chrono::Days::new(1)).to_string();
        let notes: i64 = store.db.query_row("SELECT COUNT(*) FROM notes WHERE is_draft=0 AND created_at >= ? AND created_at < ?", params![date_str, next], |r| r.get(0)).map_err(db_err)?;
        let clips: i64 = store
            .db
            .query_row(
                "SELECT COUNT(*) FROM clipboard_entries WHERE copied_at >= ? AND copied_at < ?",
                params![date_str, next],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        let todos: i64 = store
            .db
            .query_row(
                "SELECT COUNT(*) FROM todos WHERE due_at >= ? AND due_at < ?",
                params![date_str, next],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        let completed: i64 = store
            .db
            .query_row(
                "SELECT COUNT(*) FROM todos WHERE completed_at >= ? AND completed_at < ?",
                params![date_str, next],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        let overdue: i64 = store
            .db
            .query_row(
                "SELECT COUNT(*) FROM todos WHERE status='open' AND due_at >= ? AND due_at < ? AND due_at < ?",
                params![date_str, next, now()],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        days.push(ActivityDay {
            date: date_str,
            notes,
            clips,
            todos,
            completed,
            overdue,
        });
    }
    Ok(days)
}

#[tauri::command]
fn cleanup_clipboard(state: State<'_, AppState>) -> Result<usize, String> {
    let store = state.0.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let days: i64 = store
        .db
        .query_row(
            "SELECT value FROM settings WHERE key='clipboard_retention_days'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(db_err)?
        .and_then(|x: String| x.parse().ok())
        .unwrap_or(30);
    let threshold = (Utc::now() - Duration::days(days)).to_rfc3339();
    let count = store
        .db
        .execute(
            "DELETE FROM clipboard_entries WHERE pinned=0 AND modified_at < ?",
            [threshold],
        )
        .map_err(db_err)?;
    Ok(count)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("获取应用数据目录失败: {e}"))?;
            let store = Store::open(data_dir).map_err(std::io::Error::other)?;
            app.manage(AppState(Mutex::new(store)));

            let show = MenuItem::with_id(app, "show", "打开 Inkling", true, None::<&str>)?;
            let stats = MenuItem::with_id(app, "stats", "统计报表", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "偏好设置", true, None::<&str>)?;
            let quit = PredefinedMenuItem::quit(app, Some("退出 Inkling"))?;
            let menu = Menu::with_items(app, &[&show, &stats, &settings, &quit])?;
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or_else(|| std::io::Error::other("未找到默认应用图标"))?;
            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Inkling · 念头捕手")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let target = match event.id.as_ref() {
                        "show" => Some("notes"),
                        "stats" => Some("stats"),
                        "settings" => Some("settings"),
                        _ => None,
                    };
                    if let Some(target) = target {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = app.emit("navigate", target);
                        }
                    }
                })
                .build(app)?;

            app.handle()
                .global_shortcut()
                .on_shortcut("CommandOrControl+Shift+Space", |app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_notes,
            save_note,
            delete_note,
            list_clipboard,
            save_clipboard,
            update_clipboard,
            set_clipboard_pinned,
            delete_clipboard,
            list_todos,
            save_todo,
            complete_todo,
            create_child_todo,
            set_todo_priority,
            delete_todo,
            get_settings,
            save_settings,
            get_activity,
            cleanup_clipboard
        ])
        .run(tauri::generate_context!())
        .expect("启动 Inkling 失败");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_trimmed_deduplicated_and_limited() {
        let tags = vec!["  Rust ".into(), "rust".into(), "Vue".into(), "".into()];
        assert_eq!(normalized_tags(&tags), vec!["Rust", "Vue"]);
    }

    #[test]
    fn content_hash_is_stable() {
        assert_eq!(hash_content("Inkling"), hash_content("Inkling"));
        assert_ne!(hash_content("Inkling"), hash_content("inkling"));
    }

    #[test]
    fn priorities_only_accept_three_values() {
        assert!(is_valid_priority("high"));
        assert!(is_valid_priority("medium"));
        assert!(is_valid_priority("low"));
        assert!(!is_valid_priority("urgent"));
    }

    #[test]
    fn todo_tags_are_trimmed_deduplicated_and_limited() {
        let tags = vec!["  Rust ".into(), "rust".into(), "Vue".into()];
        assert_eq!(normalized_todo_tags(&tags).unwrap(), vec!["Rust", "Vue"]);
        assert!(normalized_todo_tags(&["a".into(), "b".into(), "c".into(), "d".into()]).is_err());
        assert!(normalized_todo_tags(&["12345678901".into()]).is_err());
    }
}
