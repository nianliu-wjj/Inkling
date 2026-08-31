//! 笔记数据访问：CRUD、草稿提升与 >1MB 落盘（临时文件 + 原子替换）。

use super::{db_err, now, Store};
use crate::domain::models::Note;
use rusqlite::OptionalExtension;
use std::fs;

const MAX_NOTE_BYTES: usize = 1_048_576;

pub struct NoteInput {
    pub id: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub draft: bool,
}

impl Store {
    fn note_row_ids(&self, where_clause: &str) -> Result<Vec<String>, String> {
        let sql = format!("SELECT id FROM notes {where_clause}");
        let mut stmt = self.db.prepare(&sql).map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_err)?;
        Ok(rows)
    }

    pub fn list_notes(&self) -> Result<Vec<Note>, String> {
        self.note_row_ids("WHERE is_draft=0 ORDER BY pinned DESC, updated_at DESC")?
            .iter()
            .map(|id| self.note(id))
            .collect()
    }

    /// 当前活跃草稿（面板 500ms 自动保存的暂存内容）。
    pub fn active_draft(&self) -> Result<Option<Note>, String> {
        let ids = self.note_row_ids("WHERE is_draft=1 ORDER BY updated_at DESC LIMIT 1")?;
        match ids.first() {
            Some(id) => Ok(Some(self.note(id)?)),
            None => Ok(None),
        }
    }

    fn write_large_note(&self, id: &str, content: &str) -> Result<String, String> {
        let relative = format!("notes/{id}.md");
        let target = self.data_dir.join(&relative);
        let temp = target.with_extension("md.tmp");
        fs::write(&temp, content.as_bytes()).map_err(|e| format!("写入大笔记失败: {e}"))?;
        if let Err(error) = fs::rename(&temp, &target) {
            if target.exists() {
                fs::remove_file(&target).map_err(|e| format!("替换大笔记失败: {e}"))?;
                fs::rename(&temp, &target)
                    .map_err(|e| format!("提交大笔记失败: {e}; 原始错误: {error}"))?;
            } else {
                return Err(format!("提交大笔记失败: {error}"));
            }
        }
        Ok(relative)
    }

    /// 保存笔记。草稿→归档的迁移点写入 archived_at 并记录幂等统计事件；
    /// 数据库与文件写入遵循“先文件后库”，失败时保留旧状态可恢复。
    pub fn save_note(&self, input: &NoteInput) -> Result<Note, String> {
        let tags = crate::domain::clipboard::validate_note_tags(&input.tags)?;
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let timestamp = now();
        let existing: Option<(String, String, Option<String>, i64)> = self
            .db
            .query_row(
                "SELECT created_at, COALESCE(file_path, ''), archived_at, is_draft FROM notes WHERE id=?",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(db_err)?;
        let was_draft = existing.as_ref().is_some_and(|x| x.3 == 1);
        let created_at = existing
            .as_ref()
            .map(|x| x.0.clone())
            .unwrap_or_else(|| timestamp.clone());
        let old_file_path = existing
            .as_ref()
            .and_then(|x| (!x.1.is_empty()).then_some(x.1.clone()));
        let archived_at = existing.as_ref().map(|x| x.2.clone()).unwrap_or(None);

        let just_archived = !input.draft && (was_draft || archived_at.is_none());
        let archived_at = if input.draft {
            None
        } else {
            Some(archived_at.unwrap_or_else(|| timestamp.clone()))
        };

        let (content, file_path) = if input.content.len() > MAX_NOTE_BYTES && !input.draft {
            (
                String::new(),
                Some(self.write_large_note(&id, &input.content)?),
            )
        } else {
            (input.content.clone(), None)
        };

        self.db
            .execute(
                "INSERT INTO notes(id, content, plain_text, file_path, is_draft, archived_at, created_at, updated_at) \
                 VALUES(?,?,?,?,?,?,?,?) \
                 ON CONFLICT(id) DO UPDATE SET content=excluded.content, plain_text=excluded.plain_text, \
                   file_path=excluded.file_path, is_draft=excluded.is_draft, archived_at=excluded.archived_at, \
                   updated_at=excluded.updated_at",
                rusqlite::params![
                    id,
                    content,
                    input.content,
                    file_path,
                    input.draft as i64,
                    archived_at,
                    created_at,
                    timestamp
                ],
            )
            .map_err(db_err)?;
        self.replace_tags("note_tags", "note_id", &id, &tags)?;

        if let Some(old_path) = old_file_path {
            if file_path.as_deref() != Some(old_path.as_str()) {
                let _ = fs::remove_file(self.data_dir.join(old_path));
            }
        }
        if just_archived {
            self.record_event("note_archived", &id, &timestamp)?;
        }
        self.note(&id)
    }

    pub fn delete_note(&self, id: &str) -> Result<(), String> {
        if let Some(path) = self
            .db
            .query_row("SELECT file_path FROM notes WHERE id=?", [id], |r| {
                r.get::<_, Option<String>>(0)
            })
            .optional()
            .map_err(db_err)?
            .flatten()
        {
            let _ = fs::remove_file(self.data_dir.join(path));
        }
        self.db
            .execute("DELETE FROM notes WHERE id=?", [id])
            .map_err(db_err)?;
        Ok(())
    }

    /// 读取笔记正文（含外置文件回读），导出使用。
    pub fn note_content(&self, id: &str) -> Result<String, String> {
        Ok(self.note(id)?.content)
    }
}
