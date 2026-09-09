//! 全盘文件索引：SQLite（rusqlite bundled，含 FTS5）。
//!
//! `file_entry` 存元数据 + 代际；`file_fts`（trigram 分词）对关键字做子串/错字粗筛。
//! 查询只返回命中的 ≤limit 行交给 score.rs 精排——索引常驻磁盘，内存不承载全表，
//! 守住需求 3.1 的 120MB 空闲内存预算。代际（gen）实现增量：每轮全盘重扫写新代，
//! 扫描结束删除非当前代的行（即本轮未再出现 = 已消失的文件）。

use std::path::Path;

use rusqlite::{params, Connection};

use super::model::Kind;

/// 一条文件索引行。
#[derive(Debug, Clone)]
pub struct FileRow {
    pub name: String,
    pub path: String,
    pub kind: Kind,
    pub keywords: Vec<String>,
}

/// 文件索引句柄。单连接单线程使用：后台重建线程与查询各自 `open` 一个连接。
pub struct FileIndex {
    conn: Connection,
    generation: i64,
}

fn kind_to_i64(k: Kind) -> i64 {
    match k {
        Kind::App => 0,
        Kind::Uwp => 1,
        Kind::File => 2,
        Kind::Folder => 3,
        Kind::Command => 4,
    }
}

fn kind_from_i64(v: i64) -> Kind {
    match v {
        0 => Kind::App,
        1 => Kind::Uwp,
        3 => Kind::Folder,
        4 => Kind::Command,
        _ => Kind::File,
    }
}

impl FileIndex {
    /// 打开/创建索引库并建表。generation 从库中已有最大代起，`begin_generation` 再 +1。
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS file_entry(
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kw   TEXT NOT NULL,
                kind INTEGER NOT NULL,
                gen  INTEGER NOT NULL
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS file_fts USING fts5(
                kw, content='file_entry', content_rowid='rowid', tokenize='trigram'
             );
             CREATE TRIGGER IF NOT EXISTS file_ai AFTER INSERT ON file_entry BEGIN
                INSERT INTO file_fts(rowid, kw) VALUES (new.rowid, new.kw);
             END;
             CREATE TRIGGER IF NOT EXISTS file_ad AFTER DELETE ON file_entry BEGIN
                INSERT INTO file_fts(file_fts, rowid, kw) VALUES('delete', old.rowid, old.kw);
             END;
             CREATE TRIGGER IF NOT EXISTS file_au AFTER UPDATE ON file_entry BEGIN
                INSERT INTO file_fts(file_fts, rowid, kw) VALUES('delete', old.rowid, old.kw);
                INSERT INTO file_fts(rowid, kw) VALUES (new.rowid, new.kw);
             END;",
        )?;
        let max_gen: i64 = conn
            .query_row("SELECT COALESCE(MAX(gen), 0) FROM file_entry", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        Ok(Self {
            conn,
            generation: max_gen,
        })
    }

    /// 开新一代（每次全盘重扫前调用）。
    pub fn begin_generation(&mut self) {
        self.generation += 1;
    }

    /// 批量 upsert 当前代。kw = 小写名 + 关键字（含拼音）空格连接，供 trigram 分词。
    pub fn upsert_batch(&self, rows: &[FileRow]) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO file_entry(path, name, kw, kind, gen) VALUES(?1,?2,?3,?4,?5)
                 ON CONFLICT(path) DO UPDATE SET name=?2, kw=?3, kind=?4, gen=?5",
            )?;
            for r in rows {
                let kw = format!("{} {}", r.name.to_lowercase(), r.keywords.join(" "));
                stmt.execute(params![
                    r.path,
                    r.name,
                    kw,
                    kind_to_i64(r.kind),
                    self.generation
                ])?;
            }
        }
        tx.commit()
    }

    /// 删除不属于当前代的行（本轮全盘扫描未再出现 = 已消失）。返回删除条数。
    pub fn prune_stale(&self) -> rusqlite::Result<usize> {
        self.conn.execute(
            "DELETE FROM file_entry WHERE gen <> ?1",
            params![self.generation],
        )
    }

    /// 粗筛：输入 ≥3 字符走 FTS trigram MATCH，否则退化为 kw LIKE 前缀；返回 ≤limit 行。
    pub fn query(&self, input: &str, limit: usize) -> rusqlite::Result<Vec<FileRow>> {
        let q = input.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        if q.chars().count() >= 3 {
            let mut stmt = self.conn.prepare_cached(
                "SELECT e.name, e.path, e.kind FROM file_fts f
                 JOIN file_entry e ON e.rowid = f.rowid
                 WHERE file_fts MATCH ?1 LIMIT ?2",
            )?;
            // 用双引号包成 FTS5 字符串字面量，规避 q 中的特殊语法字符。
            let pat = format!("\"{}\"", q.replace('"', ""));
            collect_rows(&mut stmt, params![pat, limit as i64])
        } else {
            let mut stmt = self.conn.prepare_cached(
                "SELECT name, path, kind FROM file_entry WHERE kw LIKE ?1 LIMIT ?2",
            )?;
            collect_rows(&mut stmt, params![format!("%{q}%"), limit as i64])
        }
    }

    /// 索引总条目数（设置页展示）。
    pub fn count(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM file_entry", [], |r| r.get(0))
            .unwrap_or(0)
    }
}

fn collect_rows(
    stmt: &mut rusqlite::CachedStatement<'_>,
    p: impl rusqlite::Params,
) -> rusqlite::Result<Vec<FileRow>> {
    stmt.query_map(p, |row| {
        Ok(FileRow {
            name: row.get::<_, String>(0)?,
            path: row.get::<_, String>(1)?,
            kind: kind_from_i64(row.get::<_, i64>(2)?),
            keywords: Vec::new(),
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::launcher::keyword;
    use crate::services::launcher::model::Kind;

    fn row(name: &str, path: &str, kind: Kind) -> FileRow {
        FileRow {
            name: name.into(),
            path: path.into(),
            kind,
            keywords: keyword::generate(name),
        }
    }

    #[test]
    fn upsert_query_and_prune_by_generation() {
        let dir = std::env::temp_dir().join(format!("inkling-idx-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.db");
        let _ = std::fs::remove_file(&file);

        let mut idx = FileIndex::open(&file).unwrap();
        idx.begin_generation();
        idx.upsert_batch(&[
            row("报告.docx", "C:/a/报告.docx", Kind::File),
            row("Projects", "D:/Projects", Kind::Folder),
        ])
        .unwrap();

        // 拼音命中文件，英文命中文件夹
        assert!(idx
            .query("baogao", 50)
            .unwrap()
            .iter()
            .any(|r| r.path == "C:/a/报告.docx"));
        assert!(idx
            .query("proj", 50)
            .unwrap()
            .iter()
            .any(|r| r.path == "D:/Projects"));

        // 新一代只写其中一条 → prune 删除未再出现的另一条
        idx.begin_generation();
        idx.upsert_batch(&[row("Projects", "D:/Projects", Kind::Folder)])
            .unwrap();
        let removed = idx.prune_stale().unwrap();
        assert_eq!(removed, 1);
        assert!(idx.query("baogao", 50).unwrap().is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
