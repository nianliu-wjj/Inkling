//! 数据层：SQLite 连接、迁移与共享辅助。渲染进程不直接碰 SQL。

pub mod clipboard;
pub mod notes;
pub mod settings;
pub mod stats;
pub mod todos;

use chrono::{DateTime, Local, TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension};
use std::path::PathBuf;

/// 本地日期键（yyyy-MM-dd，用户当前时区）。
pub fn local_date_key(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%Y-%m-%d").to_string()
}

/// 本地月份键（yyyy-MM）。
pub fn local_month_key(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%Y-%m").to_string()
}

/// 本地某日起点的 UTC RFC3339 字符串（用于与存储值做字典序比较）。
pub fn local_day_start(date: &str) -> String {
    let naive = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
        .unwrap_or_else(|_| Local::now().date_naive().and_hms_opt(0, 0, 0).unwrap());
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339())
}

/// 本地某日结束（次日零点）的 UTC RFC3339 字符串。
pub fn local_day_end(date: &str) -> String {
    let naive_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());
    let next = naive_date + chrono::Days::new(1);
    local_day_start(&next.to_string())
}

/// 当前时刻（UTC RFC3339）。
pub fn now() -> String {
    Utc::now().to_rfc3339()
}

pub fn db_err(error: rusqlite::Error) -> String {
    format!("数据库操作失败: {error}")
}

/// 持久化存储：单连接 + WAL。
pub struct Store {
    pub db: Connection,
    pub data_dir: PathBuf,
}

impl Store {
    pub fn open(data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir.join("notes")).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
        std::fs::create_dir_all(data_dir.join("clipboard")).map_err(|e| format!("创建剪贴板附件目录失败: {e}"))?;
        let db = Connection::open(data_dir.join("inkling.sqlite3")).map_err(|e| format!("打开数据库失败: {e}"))?;
        db.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(|e| format!("初始化数据库参数失败: {e}"))?;
        let mut store = Self { db, data_dir };
        store.migrate()?;
        Ok(store)
    }

    /// 版本化迁移：v0（初版）→ v1（archived_at / 附件路径 / 统计事件 / 提醒实例 / 提醒抑制标记）。
    fn migrate(&mut self) -> Result<(), String> {
        let version: i64 = self
            .db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db_err)?;
        self.db
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS notes (
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
                   content_type TEXT NOT NULL CHECK (content_type IN ('text','link','code','image','richtext')),
                   content TEXT NOT NULL DEFAULT '',
                   preview TEXT NOT NULL DEFAULT '',
                   file_path TEXT,
                   content_hash TEXT NOT NULL UNIQUE,
                   pinned INTEGER NOT NULL DEFAULT 0,
                   copied_at TEXT NOT NULL,
                   modified_at TEXT NOT NULL,
                   created_at TEXT
                 );
                 CREATE INDEX IF NOT EXISTS idx_clipboard_hash ON clipboard_entries(content_hash);
                 CREATE TABLE IF NOT EXISTS todos (
                   id TEXT PRIMARY KEY,
                   content TEXT NOT NULL,
                   due_at TEXT NOT NULL,
                   completed_at TEXT,
                   status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','done')),
                   remind_at TEXT,
                   repeat_rule TEXT CHECK (repeat_rule IN ('daily','weekly') OR repeat_rule IS NULL),
                   priority TEXT NOT NULL DEFAULT 'medium' CHECK (priority IN ('high','medium','low')),
                   remark TEXT NOT NULL DEFAULT '',
                   parent_id TEXT REFERENCES todos(id) ON DELETE CASCADE,
                   created_at TEXT NOT NULL,
                   updated_at TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_todos_due ON todos(due_at, status);
                 CREATE INDEX IF NOT EXISTS idx_todos_parent ON todos(parent_id);
                 CREATE TABLE IF NOT EXISTS todo_tags (
                   todo_id TEXT NOT NULL REFERENCES todos(id) ON DELETE CASCADE,
                   tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                   PRIMARY KEY(todo_id, tag_id)
                 );
                 CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .map_err(|e| format!("初始化数据库失败: {e}"))?;

        if version < 1 {
            self.with_v1().map_err(|e| format!("数据库迁移到 v1 失败: {e}"))?;
            self.db.pragma_update(None, "user_version", 1).map_err(db_err)?;
        }
        Ok(())
    }

    /// v1 增量：notes.archived_at、todo remind_off、提醒实例表、活动事件与每日聚合表。
    fn with_v1(&self) -> Result<(), String> {
        self.add_column_if_missing("notes", "archived_at", "TEXT")?;
        self.add_column_if_missing("todos", "remind_off", "INTEGER NOT NULL DEFAULT 0")?;
        self.db
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS activity_events (
                   id TEXT PRIMARY KEY,
                   event_type TEXT NOT NULL,
                   entity_id TEXT NOT NULL,
                   occurred_at TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS stats_daily (
                   date TEXT PRIMARY KEY,
                   note_archived_count INTEGER NOT NULL DEFAULT 0,
                   clipboard_captured_count INTEGER NOT NULL DEFAULT 0,
                   todo_created_count INTEGER NOT NULL DEFAULT 0,
                   todo_completed_count INTEGER NOT NULL DEFAULT 0,
                   updated_at TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS reminder_log (
                   id TEXT PRIMARY KEY,
                   todo_id TEXT NOT NULL,
                   fired_at TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_reminder_log_todo ON reminder_log(todo_id);",
            )
            .map_err(db_err)?;
        // 回填历史归档时间：已归档笔记按创建时间补记，保证统计连续。
        self.db
            .execute(
                "UPDATE notes SET archived_at = created_at WHERE is_draft = 0 AND archived_at IS NULL",
                [],
            )
            .map_err(db_err)?;
        self.db
            .execute(
                "UPDATE clipboard_entries SET created_at = copied_at WHERE created_at IS NULL",
                [],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn add_column_if_missing(&self, table: &str, column: &str, decl: &str) -> Result<(), String> {
        let existing: Vec<String> = {
            let mut stmt = self
                .db
                .prepare(&format!("PRAGMA table_info({table})"))
                .map_err(db_err)?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(1))
                .map_err(db_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db_err)?;
            rows
        };
        if !existing.iter().any(|c| c == column) {
            self.db
                .execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"), [])
                .map_err(db_err)?;
        }
        Ok(())
    }

    /// 统一替换实体标签（内部调用方已负责约束校验）。
    pub fn replace_tags(
        &self,
        table: &str,
        id_column: &str,
        id: &str,
        tags: &[String],
    ) -> Result<(), String> {
        self.db
            .execute(&format!("DELETE FROM {table} WHERE {id_column}=?"), [id])
            .map_err(db_err)?;
        let insert_sql = format!("INSERT OR IGNORE INTO {table} ({id_column}, tag_id) VALUES (?, ?)");
        for raw in tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            let normalized = raw.to_lowercase();
            self.db
                .execute(
                    "INSERT OR IGNORE INTO tags(name, normalized) VALUES(?, ?)",
                    rusqlite::params![raw, normalized],
                )
                .map_err(db_err)?;
            let tag_id: i64 = self
                .db
                .query_row("SELECT id FROM tags WHERE normalized=?", [normalized], |r| r.get(0))
                .map_err(db_err)?;
            self.db
                .execute(&insert_sql, rusqlite::params![id, tag_id])
                .map_err(db_err)?;
        }
        Ok(())
    }

    /// 读取实体标签。
    pub fn tags(&self, table: &str, id_column: &str, id: &str) -> Result<Vec<String>, String> {
        let sql = format!(
            "SELECT t.name FROM tags t JOIN {table} x ON x.tag_id=t.id WHERE x.{id_column}=? ORDER BY t.name"
        );
        let mut stmt = self.db.prepare(&sql).map_err(db_err)?;
        stmt.query_map([id], |row| row.get(0))
            .map_err(db_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_err)
    }

    /// 打开事务（unchecked_transaction 允许借用 &self）。
    pub fn tx(&self) -> Result<rusqlite::Transaction<'_>, String> {
        self.db.unchecked_transaction().map_err(db_err)
    }

    /// 幂等写入活动事件并聚合到 stats_daily。
    pub fn record_event(&self, event_type: &str, entity_id: &str, occurred_at: &str) -> Result<(), String> {
        let date = local_date_key(
            DateTime::parse_from_rfc3339(occurred_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        );
        let event_id = format!("{event_type}:{entity_id}:{date}");
        let inserted = self
            .db
            .execute(
                "INSERT OR IGNORE INTO activity_events(id, event_type, entity_id, occurred_at) VALUES(?,?,?,?)",
                rusqlite::params![event_id, event_type, entity_id, occurred_at],
            )
            .map_err(db_err)?;
        if inserted == 0 {
            return Ok(());
        }
        let column = match event_type {
            "note_archived" => "note_archived_count",
            "clipboard_captured" => "clipboard_captured_count",
            "todo_created" => "todo_created_count",
            "todo_completed" => "todo_completed_count",
            _ => return Ok(()),
        };
        self.db
            .execute(
                &format!(
                    "INSERT INTO stats_daily(date, {column}, updated_at) VALUES(?, 1, ?)
                     ON CONFLICT(date) DO UPDATE SET {column} = {column} + 1, updated_at = excluded.updated_at"
                ),
                rusqlite::params![date, now()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// 查询待办（含标签）。
    pub fn todo(&self, id: &str) -> Result<crate::domain::models::Todo, String> {
        self.db
            .query_row(
                "SELECT id, content, due_at, completed_at, status, remind_at, repeat_rule, remind_off, \
                 priority, remark, parent_id, created_at, updated_at FROM todos WHERE id=?",
                [id],
                |r| {
                    Ok(crate::domain::models::Todo {
                        id: r.get(0)?,
                        content: r.get(1)?,
                        due_at: r.get(2)?,
                        completed_at: r.get(3)?,
                        status: r.get(4)?,
                        remind_at: r.get(5)?,
                        repeat_rule: r.get(6)?,
                        remind_off: r.get::<_, i64>(7)? != 0,
                        priority: r.get(8)?,
                        remark: r.get(9)?,
                        parent_id: r.get(10)?,
                        tags: Vec::new(),
                        created_at: r.get(11)?,
                        updated_at: r.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(db_err)?
            .map(|mut todo| {
                todo.tags = self.tags("todo_tags", "todo_id", &todo.id)?;
                Ok(todo)
            })
            .unwrap_or_else(|| Err("待办不存在".into()))
    }

    /// 查询笔记（含标签，外置文件时回读内容）。
    pub fn note(&self, id: &str) -> Result<crate::domain::models::Note, String> {
        let row = self
            .db
            .query_row(
                "SELECT id, content, file_path, is_draft, pinned, archived_at, created_at, updated_at \
                 FROM notes WHERE id=?",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, Option<String>>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(db_err)?;
        let Some((id, content, file_path, draft, pinned, archived_at, created_at, updated_at)) = row else {
            return Err("笔记不存在".into());
        };
        let mut content = content.unwrap_or_default();
        if content.is_empty() {
            if let Some(path) = file_path {
                content = std::fs::read_to_string(self.data_dir.join(&path)).unwrap_or_default();
            }
        }
        Ok(crate::domain::models::Note {
            tags: self.tags("note_tags", "note_id", &id)?,
            id,
            content,
            is_draft: draft != 0,
            pinned: pinned != 0,
            archived_at,
            created_at,
            updated_at,
        })
    }
}
