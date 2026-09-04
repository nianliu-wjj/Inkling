//! 剪贴板数据访问：去重入库、编辑、置顶、删除与保留期清理。

use super::{db_err, now, Store};
use crate::domain::models::ClipboardEntry;
use chrono::{Duration, Utc};
use rusqlite::OptionalExtension;

const ROW_COLUMNS: &str =
    "id, content_type, content, preview, file_path, pinned, copied_at, modified_at";

fn row_entry(r: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    Ok(ClipboardEntry {
        id: r.get(0)?,
        content_type: r.get(1)?,
        content: r.get(2)?,
        preview: r.get(3)?,
        file_path: r.get(4)?,
        pinned: r.get::<_, i64>(5)? != 0,
        copied_at: r.get(6)?,
        modified_at: r.get(7)?,
    })
}

/// 一次剪贴板捕获快照（由轮询线程或手动捕获命令产生）。
pub struct Capture {
    pub content: String,
    pub content_type: &'static str,
    pub preview: String,
    /// 图片等附件在应用数据目录内的相对路径。
    pub file_path: Option<String>,
    pub hash: String,
}

impl Store {
    fn entry_by(&self, where_clause: &str, param: &str) -> Result<Option<ClipboardEntry>, String> {
        let sql = format!("SELECT {ROW_COLUMNS} FROM clipboard_entries {where_clause}");
        self.db
            .query_row(&sql, [param], row_entry)
            .optional()
            .map_err(db_err)
    }

    pub fn clipboard_entry(&self, id: &str) -> Result<Option<ClipboardEntry>, String> {
        self.entry_by("WHERE id=?", id)
    }

    pub fn find_clipboard_by_hash(&self, hash: &str) -> Result<Option<ClipboardEntry>, String> {
        self.entry_by("WHERE content_hash=?", hash)
    }

    pub fn list_clipboard(&self) -> Result<Vec<ClipboardEntry>, String> {
        let sql = format!(
            "SELECT {ROW_COLUMNS} FROM clipboard_entries ORDER BY pinned DESC, modified_at DESC"
        );
        let mut stmt = self.db.prepare(&sql).map_err(db_err)?;
        let rows = stmt
            .query_map([], row_entry)
            .map_err(db_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_err)?;
        Ok(rows)
    }

    /// 事务内按哈希去重入库；重复内容直接忽略并返回既有条目。
    pub fn insert_capture(&self, capture: &Capture) -> Result<Option<ClipboardEntry>, String> {
        if let Some(existing) = self.find_clipboard_by_hash(&capture.hash)? {
            return Ok(Some(existing));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = now();
        self.db
            .execute(
                "INSERT INTO clipboard_entries(id, content_type, content, preview, file_path, content_hash, copied_at, modified_at, created_at) \
                 VALUES(?,?,?,?,?,?,?,?,?)",
                rusqlite::params![
                    id,
                    capture.content_type,
                    capture.content,
                    capture.preview,
                    capture.file_path,
                    capture.hash,
                    timestamp,
                    timestamp,
                    timestamp
                ],
            )
            .map_err(db_err)?;
        self.record_event("clipboard_captured", &id, &timestamp)?;
        self.clipboard_entry(&id)
    }

    /// 编辑文本类条目：替换内容并更新最后修改时间；命中重复内容时拒绝。
    pub fn update_clipboard(&self, id: &str, content: &str) -> Result<ClipboardEntry, String> {
        let existing = self.clipboard_entry(id)?.ok_or("剪贴板条目不存在")?;
        if existing.content_type == "image" {
            return Err("图片条目不可编辑".into());
        }
        let hash = crate::domain::clipboard::hash_content(content.as_bytes());
        if let Some(other) = self.find_clipboard_by_hash(&hash)? {
            if other.id != id {
                return Err("内容与已有条目重复".into());
            }
        }
        let timestamp = now();
        self.db
            .execute(
                "UPDATE clipboard_entries SET content=?, preview=?, content_hash=?, modified_at=? WHERE id=?",
                rusqlite::params![
                    content,
                    crate::domain::clipboard::build_preview(content, 240),
                    hash,
                    timestamp,
                    id
                ],
            )
            .map_err(db_err)?;
        self.clipboard_entry(id)?
            .ok_or_else(|| "剪贴板条目不存在".into())
    }

    pub fn set_clipboard_pinned(&self, id: &str, pinned: bool) -> Result<(), String> {
        self.db
            .execute(
                "UPDATE clipboard_entries SET pinned=? WHERE id=?",
                rusqlite::params![pinned as i64, id],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn delete_clipboard(&self, id: &str) -> Result<(), String> {
        if let Some(path) = self.clipboard_entry(id)?.and_then(|e| e.file_path) {
            let _ = std::fs::remove_file(self.data_dir.join(path));
        }
        self.db
            .execute("DELETE FROM clipboard_entries WHERE id=?", [id])
            .map_err(db_err)?;
        Ok(())
    }

    /// 按保留天数清理未置顶的过期条目（含附件文件），返回清理数量。
    pub fn cleanup_clipboard(&self, retention_days: i64) -> Result<usize, String> {
        let threshold = (Utc::now() - Duration::days(retention_days)).to_rfc3339();
        let expired: Vec<(String, Option<String>)> = {
            let mut stmt = self
                .db
                .prepare(
                    "SELECT id, file_path FROM clipboard_entries WHERE pinned=0 AND modified_at < ?",
                )
                .map_err(db_err)?;
            let rows = stmt
                .query_map([&threshold], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(db_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db_err)?;
            rows
        };
        for (id, file) in &expired {
            if let Some(path) = file {
                let _ = std::fs::remove_file(self.data_dir.join(path));
            }
            self.db
                .execute("DELETE FROM clipboard_entries WHERE id=?", [id])
                .map_err(db_err)?;
        }
        Ok(expired.len())
    }

    /// 将条目内容写回系统剪贴板时所需的数据（文本或图片文件路径）。
    pub fn paste_payload(
        &self,
        id: &str,
    ) -> Result<(ClipboardEntry, Option<std::path::PathBuf>), String> {
        let entry = self.clipboard_entry(id)?.ok_or("剪贴板条目不存在")?;
        let file = entry.file_path.as_ref().map(|p| self.data_dir.join(p));
        Ok((entry, file))
    }

    /// 统计某日期键下的捕获数量（由真实业务数据派生）。
    pub fn clips_captured_on(&self, date: &str) -> Result<i64, String> {
        let start = super::local_day_start(date);
        let end = super::local_day_end(date);
        self.db
            .query_row(
                "SELECT COUNT(*) FROM clipboard_entries WHERE copied_at >= ? AND copied_at < ?",
                rusqlite::params![start, end],
                |r| r.get(0),
            )
            .map_err(db_err)
    }
}
