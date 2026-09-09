# 启动器全盘文件索引 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让启动器能搜索全盘所有文件/文件夹，结果按「应用组→文件夹→文件」严格分组、组内按匹配分排序；文件索引落地 SQLite（FTS5 trigram），内存只承载命中行。

**Architecture:** 程序/UWP/命令仍走内存候选（现状不变）；文件/文件夹改为后台全盘 `WalkDir` 写入独立 SQLite 库 `launcher-index.db`（`file_entry` 表 + FTS5 trigram 虚表）。查询时用 FTS `MATCH` 粗筛 ≤800 行，复用现有 `score.rs` 内存精排，再与内存候选合并，按 D4 严格分组比较器排序。

**Tech Stack:** Rust、rusqlite 0.37（bundled，含 FTS5）、walkdir、windows-sys 0.59、Vue 3 + TS（设置页）。

**Spec:** `docs/superpowers/specs/2026-09-09-launcher-full-disk-index-design.md`（D1–D6 已定稿，D4 为严格分组）。

## Global Constraints

- 回答与提交信息用中文；提交格式 `<type>(<scope>): <中文描述>`，scope 用 `launcher`。每完成一个 Task 立即提交并推送，不攒。
- Rust 提交前 `cargo test`（相关包）零失败、`cargo fmt`；前端提交前 `npx vue-tsc --noEmit` 与 `npx prettier --check`。
- 全项目禁止裸 `println!` 之外的调试输出；Rust 日志用 `eprintln!("[launcher] ...")`，关键节点必须有日志：索引开始/结束、每盘条目数、查询回退、重建触发。
- 内存预算：文件索引**不得**整表载入内存；查询只取 FTS 命中的 ≤800 行。
- 只索引**固定磁盘**（`DRIVE_FIXED`），跳过可移动盘与网络盘。
- 只索引路径与文件名，**不读文件内容**。
- 移植/新增不破坏现有程序/UWP/命令搜索与「回车打开第一项」行为。
- 新增 SQLite 库文件放应用数据目录下 `launcher/launcher-index.db`，与业务库分离。

---

## File Structure

| 路径                                                 | 职责                                                                                                                 |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/services/launcher/drives.rs`          | 新增：枚举固定磁盘根（`GetLogicalDrives`+`GetDriveTypeW`）；纯函数 `drive_letters_from_bitmask` 可单测               |
| `src-tauri/src/services/launcher/exclude.rs`         | 新增：`is_excluded_dir(name)` 纯函数 + 内置排除集 + 运行期追加                                                       |
| `src-tauri/src/services/launcher/index_db.rs`        | 新增：`FileIndex` 封装 SQLite——建表(FTS5 trigram)、批量 upsert、代际 prune、`query(input, limit)`                    |
| `src-tauri/src/services/launcher/scan.rs`            | 改：新增 `scan_files_to_index(index, drives, extra_excludes)` 全盘写库；旧内存文件扫描保留供「关闭全盘索引」回退     |
| `src-tauri/src/services/launcher/search.rs`          | 改：新增 `bucket(kind)`、`final_cmp`（D4 严格分组）；`merge_and_rank(mem_hits, file_hits, top_k)`                    |
| `src-tauri/src/services/launcher/mod.rs`             | 改：`LauncherState` 持 `FileIndex`；`rebuild` 同时刷内存候选与文件索引；`search` 合并两路；新增 `rebuild_index_only` |
| `src-tauri/src/services/launcher/model.rs`           | 不变（`Kind` 已含 File/Folder；`Candidate`/`Hit` 复用）                                                              |
| `src-tauri/Cargo.toml`                               | 改：`windows-sys` 增 `Win32_Storage_FileSystem` feature                                                              |
| `src-tauri/src/domain/models.rs`、`data/settings.rs` | 改：新增 `launcher_full_disk_index`(bool,默认 true)、`launcher_extra_excludes`(String)                               |
| `src-tauri/src/ipc.rs`                               | 改：`launcher_rebuild` 已存在；新增 `launcher_index_status`（可选，复用 status）                                     |
| `src/windows/Main/SettingsView.vue`                  | 改：启动器分区加「全盘文件索引」开关 / 额外排除目录 / 重建索引按钮                                                   |
| `src/service/tauri.ts`                               | 改：暴露设置字段（若走通用 settings 保存则无需改）                                                                   |

---

## Task 1：固定磁盘枚举 `drives.rs`

**Files:**

- Create: `src-tauri/src/services/launcher/drives.rs`
- Modify: `src-tauri/src/services/launcher/mod.rs`（加 `mod drives;`）
- Modify: `src-tauri/Cargo.toml`（windows-sys feature）

**Interfaces:**

- Produces: `pub fn fixed_drive_roots() -> Vec<std::path::PathBuf>`；`pub fn drive_letters_from_bitmask(mask: u32) -> Vec<char>`

- [ ] **Step 1: Cargo.toml 加 feature**

在 `windows-sys` 的 features 数组里追加（保持既有项）：

```toml
"Win32_Storage_FileSystem",
"Win32_Foundation",
```

- [ ] **Step 2: 写失败测试（纯函数位掩码解析）**

`drives.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmask_maps_bits_to_letters() {
        // bit0=A, bit2=C, bit3=D
        assert_eq!(drive_letters_from_bitmask(0b1101), vec!['A', 'C', 'D']);
        assert_eq!(drive_letters_from_bitmask(0), Vec::<char>::new());
    }
}
```

- [ ] **Step 3: 运行确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml drives::`
Expected: 编译错误 `drive_letters_from_bitmask not found`。

- [ ] **Step 4: 实现**

`drives.rs` 顶部：

```rust
//! 固定磁盘枚举：仅 DRIVE_FIXED，跳过可移动盘/网络盘/光驱。

use std::path::PathBuf;

/// 把 GetLogicalDrives 的位掩码解析成盘符列表（bit0=A…bit25=Z）。纯函数，便于单测。
pub fn drive_letters_from_bitmask(mask: u32) -> Vec<char> {
    (0..26)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| (b'A' + i as u8) as char)
        .collect()
}

/// 枚举所有固定磁盘的根路径（如 `C:\`、`D:\`）。非 Windows 返回空。
#[cfg(target_os = "windows")]
pub fn fixed_drive_roots() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives, DRIVE_FIXED};
    let mask = unsafe { GetLogicalDrives() };
    drive_letters_from_bitmask(mask)
        .into_iter()
        .filter_map(|letter| {
            let root = format!("{letter}:\\");
            // GetDriveTypeW 需要以 NUL 结尾的宽字符串。
            let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
            let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
            (kind == DRIVE_FIXED).then(|| PathBuf::from(root))
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
pub fn fixed_drive_roots() -> Vec<PathBuf> {
    Vec::new()
}
```

在 `mod.rs` 顶部模块声明区加 `mod drives;`。

- [ ] **Step 5: 运行确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml drives::`
Expected: PASS。

- [ ] **Step 6: 提交并推送**

```bash
git add src-tauri/src/services/launcher/drives.rs src-tauri/src/services/launcher/mod.rs src-tauri/Cargo.toml
git commit -m "feat(launcher): 固定磁盘枚举（DRIVE_FIXED）"
git push origin feature/tauri-vue
```

---

## Task 2：目录排除规则 `exclude.rs`

**Files:**

- Create: `src-tauri/src/services/launcher/exclude.rs`
- Modify: `src-tauri/src/services/launcher/mod.rs`（加 `mod exclude;`）

**Interfaces:**

- Produces: `pub fn is_excluded_dir(name: &str, extra: &[String]) -> bool`

- [ ] **Step 1: 写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_builtin_and_extra_case_insensitive() {
        assert!(is_excluded_dir("Windows", &[]));
        assert!(is_excluded_dir("node_modules", &[]));
        assert!(is_excluded_dir("$Recycle.Bin", &[]));
        assert!(is_excluded_dir(".git", &[]));
        assert!(!is_excluded_dir("Projects", &[]));
        // 大小写不敏感 + 运行期追加
        assert!(is_excluded_dir("WINDOWS", &[]));
        assert!(is_excluded_dir("MyCache", &["mycache".to_string()]));
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml exclude::`
Expected: FAIL（未定义）。

- [ ] **Step 3: 实现**

```rust
//! 全盘扫描的目录排除规则：系统目录、缓存、版本控制、构建产物等。

/// 内置排除目录名（小写比较）。
const BUILTIN: &[&str] = &[
    "windows",
    "$recycle.bin",
    "system volume information",
    "$winreagent",
    "recovery",
    "perflogs",
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "target",
    ".cache",
    "__pycache__",
    ".gradle",
    ".nuget",
];

/// 目录名是否应被跳过（大小写不敏感）。`extra` 为设置里追加的目录名（小写）。
pub fn is_excluded_dir(name: &str, extra: &[String]) -> bool {
    let lower = name.to_ascii_lowercase();
    BUILTIN.contains(&lower.as_str()) || extra.iter().any(|e| e.eq_ignore_ascii_case(&lower))
}
```

`mod.rs` 加 `mod exclude;`。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml exclude::`
Expected: PASS。

- [ ] **Step 5: 提交并推送**

```bash
git add src-tauri/src/services/launcher/exclude.rs src-tauri/src/services/launcher/mod.rs
git commit -m "feat(launcher): 全盘扫描目录排除规则"
git push origin feature/tauri-vue
```

---

## Task 3：SQLite 文件索引 `index_db.rs`

**Files:**

- Create: `src-tauri/src/services/launcher/index_db.rs`
- Modify: `src-tauri/src/services/launcher/mod.rs`（加 `mod index_db;`）

**Interfaces:**

- Consumes: `super::model::Kind`
- Produces:
  - `pub struct FileIndex { conn: rusqlite::Connection, generation: i64 }`
  - `pub struct FileRow { pub name: String, pub path: String, pub kind: Kind, pub keywords: Vec<String> }`
  - `pub fn open(path: &std::path::Path) -> rusqlite::Result<FileIndex>`
  - `impl FileIndex`: `fn begin_generation(&mut self)`；`fn upsert_batch(&self, rows: &[FileRow]) -> rusqlite::Result<()>`；`fn prune_stale(&self) -> rusqlite::Result<usize>`；`fn query(&self, input: &str, limit: usize) -> rusqlite::Result<Vec<FileRow>>`；`fn count(&self) -> i64`

- [ ] **Step 1: 写失败测试（临时库建表/写入/查询/代际清理）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::launcher::model::Kind;

    fn row(name: &str, path: &str, kind: Kind) -> FileRow {
        FileRow { name: name.into(), path: path.into(), kind,
            keywords: crate::services::launcher::keyword::generate(name) }
    }

    #[test]
    fn upsert_query_and_prune_by_generation() {
        let dir = std::env::temp_dir().join(format!("inkling-idx-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.db");
        let _ = std::fs::remove_file(&file);

        let mut idx = FileIndex::open(&file).unwrap();
        idx.begin_generation();
        idx.upsert_batch(&[row("报告.docx", "C:/a/报告.docx", Kind::File),
                           row("Projects", "D:/Projects", Kind::Folder)]).unwrap();
        // 名称/拼音命中
        let hits = idx.query("baogao", 50).unwrap();
        assert!(hits.iter().any(|r| r.path == "C:/a/报告.docx"));
        assert!(idx.query("proj", 50).unwrap().iter().any(|r| r.path == "D:/Projects"));

        // 新一代只写其中一条 → prune 删除未再出现的另一条
        idx.begin_generation();
        idx.upsert_batch(&[row("Projects", "D:/Projects", Kind::Folder)]).unwrap();
        let removed = idx.prune_stale().unwrap();
        assert_eq!(removed, 1);
        assert!(idx.query("baogao", 50).unwrap().is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml index_db::`
Expected: FAIL（未定义）。

- [ ] **Step 3: 实现**

```rust
//! 全盘文件索引：SQLite（rusqlite bundled 含 FTS5）。
//! file_entry 存元数据 + 代际；file_fts(trigram) 做名称/拼音粗筛；查询命中后交 score.rs 精排。

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

/// 文件索引句柄（单线程使用：后台重建线程与查询各自 open 一个连接）。
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
    /// 打开/创建索引库并建表。generation 从库里已有最大代 +1 起，避免与旧数据混淆。
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
            .query_row("SELECT COALESCE(MAX(gen), 0) FROM file_entry", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(Self { conn, generation: max_gen })
    }

    /// 开新一代（每次全盘重扫前调用）。
    pub fn begin_generation(&mut self) {
        self.generation += 1;
    }

    /// 批量 upsert 当前代；kw 存空格连接的关键字（含小写名与拼音），供 trigram 分词。
    pub fn upsert_batch(&self, rows: &[FileRow]) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO file_entry(path, name, kw, kind, gen) VALUES(?1,?2,?3,?4,?5)
                 ON CONFLICT(path) DO UPDATE SET name=?2, kw=?3, kind=?4, gen=?5",
            )?;
            for r in rows {
                let kw = format!("{} {}", r.name.to_lowercase(), r.keywords.join(" "));
                stmt.execute(params![r.path, r.name, kw, kind_to_i64(r.kind), self.generation])?;
            }
        }
        tx.commit()
    }

    /// 删除不属于当前代的行（本轮全盘扫描未再出现 = 已消失）。返回删除条数。
    pub fn prune_stale(&self) -> rusqlite::Result<usize> {
        self.conn
            .execute("DELETE FROM file_entry WHERE gen <> ?1", params![self.generation])
            .map(|n| n)
    }

    /// 粗筛：把输入按 trigram 规则给 FTS 查询（长度<3 退化为 LIKE 前缀），返回 ≤limit 行。
    pub fn query(&self, input: &str, limit: usize) -> rusqlite::Result<Vec<FileRow>> {
        let q = input.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let rows = if q.chars().count() >= 3 {
            let mut stmt = self.conn.prepare_cached(
                "SELECT e.name, e.path, e.kind FROM file_fts f
                 JOIN file_entry e ON e.rowid = f.rowid
                 WHERE file_fts MATCH ?1 LIMIT ?2",
            )?;
            let pat = format!("\"{}\"", q.replace('"', ""));
            collect_rows(&mut stmt, params![pat, limit as i64])?
        } else {
            let mut stmt = self.conn.prepare_cached(
                "SELECT name, path, kind FROM file_entry WHERE kw LIKE ?1 LIMIT ?2",
            )?;
            collect_rows(&mut stmt, params![format!("%{q}%"), limit as i64])?
        };
        Ok(rows)
    }

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
    let rows = stmt
        .query_map(p, |row| {
            Ok(FileRow {
                name: row.get::<_, String>(0)?,
                path: row.get::<_, String>(1)?,
                kind: kind_from_i64(row.get::<_, i64>(2)?),
                keywords: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
```

`mod.rs` 加 `mod index_db;`。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml index_db::`
Expected: PASS（若 trigram 不可用则报错——rusqlite bundled 已含，正常应通过）。

- [ ] **Step 5: 提交并推送**

```bash
git add src-tauri/src/services/launcher/index_db.rs src-tauri/src/services/launcher/mod.rs
git commit -m "feat(launcher): SQLite 文件索引（FTS5 trigram 粗筛 + 代际增量）"
git push origin feature/tauri-vue
```

---

## Task 4：全盘扫描写入索引 `scan_files_to_index`

**Files:**

- Modify: `src-tauri/src/services/launcher/scan.rs`

**Interfaces:**

- Consumes: `drives::fixed_drive_roots`、`exclude::is_excluded_dir`、`index_db::{FileIndex, FileRow}`、`keyword::generate`、`model::Kind`
- Produces: `pub fn scan_files_to_index(index: &mut index_db::FileIndex, extra_excludes: &[String]) -> usize`（返回索引条目数）

- [ ] **Step 1: 写失败测试（用临时目录树，避免真扫全盘）**

在 `scan.rs` 测试模块加：

```rust
#[test]
fn scan_dir_tree_into_index_respects_excludes() {
    use super::super::index_db::FileIndex;
    let base = std::env::temp_dir().join(format!("inkling-scan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("keep")).unwrap();
    std::fs::create_dir_all(base.join("node_modules")).unwrap();
    std::fs::write(base.join("keep/报告.txt"), b"x").unwrap();
    std::fs::write(base.join("node_modules/junk.js"), b"x").unwrap();

    let dbfile = base.join("i.db");
    let mut idx = FileIndex::open(&dbfile).unwrap();
    idx.begin_generation();
    let n = scan_dir_into_index(&base, &mut idx, &[]);
    idx.prune_stale().unwrap();

    assert!(n >= 2); // keep 目录 + 报告.txt
    assert!(idx.query("baogao", 50).unwrap().iter().any(|r| r.name.contains("报告")));
    assert!(idx.query("junk", 50).unwrap().is_empty()); // node_modules 被排除
    std::fs::remove_dir_all(&base).ok();
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scan_dir_tree_into_index`
Expected: FAIL（`scan_dir_into_index` 未定义）。

- [ ] **Step 3: 实现（单目录树入库 + 全盘入口）**

`scan.rs` 顶部补 `use` 后新增：

```rust
use super::{exclude, index_db, keyword};
use super::model::Kind;
use walkdir::WalkDir;

/// 扫描单个根目录树写入索引（供全盘入口与单测复用）。返回写入条数。
pub fn scan_dir_into_index(root: &std::path::Path, index: &mut index_db::FileIndex, extra: &[String]) -> usize {
    let mut batch: Vec<index_db::FileRow> = Vec::with_capacity(2048);
    let mut total = 0usize;
    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
        // 目录被排除则整棵跳过；文件永远放行（由父目录过滤）。
        if e.file_type().is_dir() {
            e.depth() == 0
                || e.file_name()
                    .to_str()
                    .map(|n| !exclude::is_excluded_dir(n, extra))
                    .unwrap_or(false)
        } else {
            true
        }
    });
    for entry in walker.filter_map(|e| e.ok()) {
        if entry.depth() == 0 {
            continue;
        }
        let name = match entry.file_name().to_str() {
            Some(n) => n.to_string(),
            None => continue,
        };
        let kind = if entry.file_type().is_dir() { Kind::Folder } else { Kind::File };
        batch.push(index_db::FileRow {
            keywords: keyword::generate(&name),
            name,
            path: entry.path().to_string_lossy().replace('\\', "/"),
            kind,
        });
        if batch.len() >= 2000 {
            let _ = index.upsert_batch(&batch);
            total += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let _ = index.upsert_batch(&batch);
        total += batch.len();
    }
    total
}

/// 全盘扫描所有固定磁盘写入索引。返回总条目数。
pub fn scan_files_to_index(index: &mut index_db::FileIndex, extra_excludes: &[String]) -> usize {
    index.begin_generation();
    let mut total = 0usize;
    for root in super::drives::fixed_drive_roots() {
        eprintln!("[launcher] 全盘索引开始扫描 {}", root.display());
        total += scan_dir_into_index(&root, index, extra_excludes);
    }
    let removed = index.prune_stale().unwrap_or(0);
    eprintln!("[launcher] 全盘索引完成 共 {total} 条，清理消失 {removed} 条");
    total
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scan_dir_tree_into_index`
Expected: PASS。

- [ ] **Step 5: 提交并推送**

```bash
git add src-tauri/src/services/launcher/scan.rs
git commit -m "feat(launcher): 全盘 WalkDir 扫描写入 SQLite 索引（排除+批量+代际）"
git push origin feature/tauri-vue
```

---

## Task 5：D4 严格分组排序 + 合并 `search.rs`

**Files:**

- Modify: `src-tauri/src/services/launcher/search.rs`

**Interfaces:**

- Consumes: `model::{Hit, Kind}`
- Produces: `pub fn bucket(kind: Kind) -> u8`；`pub fn merge_and_rank(mem: Vec<Hit>, files: Vec<Hit>, top_k: usize) -> Vec<Hit>`

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn strict_group_order_app_folder_file() {
    use super::super::model::{Hit, Kind};
    let h = |kind, name: &str, score: f64| Hit {
        id: 0, kind, name: name.into(), path: name.into(), score, matched_keyword: String::new(),
    };
    // 文件分更高，但严格分组下应用仍排在文件前
    let mem = vec![h(Kind::App, "低分应用", 1.0)];
    let files = vec![h(Kind::File, "高分文件", 99.0), h(Kind::Folder, "文件夹", 50.0)];
    let ranked = merge_and_rank(mem, files, 10);
    let kinds: Vec<Kind> = ranked.iter().map(|x| x.kind).collect();
    assert_eq!(kinds, vec![Kind::App, Kind::Folder, Kind::File]);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml strict_group_order`
Expected: FAIL（`merge_and_rank` 未定义）。

- [ ] **Step 3: 实现**

`search.rs` 新增：

```rust
use super::model::{Hit, Kind};

/// D4 桶优先级：应用组(App/Uwp/Command)=0 < 文件夹=1 < 文件=2。
pub fn bucket(kind: Kind) -> u8 {
    match kind {
        Kind::App | Kind::Uwp | Kind::Command => 0,
        Kind::Folder => 1,
        Kind::File => 2,
    }
}

/// 合并内存命中与文件命中，按「桶优先 → 分数降序」严格分组排序，截断 top_k。
pub fn merge_and_rank(mut mem: Vec<Hit>, mut files: Vec<Hit>, top_k: usize) -> Vec<Hit> {
    let mut all = Vec::with_capacity(mem.len() + files.len());
    all.append(&mut mem);
    all.append(&mut files);
    all.sort_by(|a, b| {
        bucket(a.kind)
            .cmp(&bucket(b.kind))
            .then(b.score.total_cmp(&a.score))
    });
    all.truncate(top_k);
    all
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml strict_group_order`
Expected: PASS。

- [ ] **Step 5: 提交并推送**

```bash
git add src-tauri/src/services/launcher/search.rs
git commit -m "feat(launcher): D4 严格分组排序（应用组→文件夹→文件）"
git push origin feature/tauri-vue
```

---

## Task 6：LauncherState 编排（持索引、后台重建、查询合并）

**Files:**

- Modify: `src-tauri/src/services/launcher/mod.rs`

**Interfaces:**

- Consumes: 前述全部
- Produces: `LauncherState` 内部持 `index_path: PathBuf`；`search` 合并内存与文件命中；`rebuild` 同时刷文件索引；`pub fn rebuild(&self, roots, full_disk: bool, extra_excludes: &[String])`

- [ ] **Step 1: 写失败测试（文件命中并入结果并遵守分组）**

```rust
#[test]
fn state_search_merges_file_index_hits() {
    let dir = std::env::temp_dir().join(format!("inkling-state-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let state = LauncherState::new(dir.clone());
    // 直接往索引库写一条文件，再查询应能命中且排在应用之后
    {
        let mut idx = index_db::FileIndex::open(&dir.join("launcher-index.db")).unwrap();
        idx.begin_generation();
        idx.upsert_batch(&[index_db::FileRow {
            name: "预算表.xlsx".into(), path: "C:/x/预算表.xlsx".into(),
            kind: model::Kind::File, keywords: keyword::generate("预算表.xlsx"),
        }]).unwrap();
    }
    let hits = state.search("yusuan");
    assert!(hits.iter().any(|h| h.name.contains("预算表")));
    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml state_search_merges`
Expected: FAIL（search 尚未查文件索引）。

- [ ] **Step 3: 实现**

在 `LauncherState::search` 里，现有内存打分得到 `mem_hits` 后，追加文件索引查询与合并（新增 `file_index_path()` 辅助）：

```rust
// LauncherState::search 尾部（返回前）改为：
let mem_hits = /* 现有内存打分结果 */;
let file_hits = self.search_file_index(&normalized, top_k);
let ranked = search::merge_and_rank(mem_hits, file_hits, top_k);
// 写入 L1 缓存后返回 ranked
```

新增方法：

```rust
fn file_index_path(&self) -> std::path::PathBuf {
    self.dir.join("launcher-index.db")
}

/// 从磁盘索引粗筛并用 score 精排，产出文件/文件夹命中。查询失败或空返回空。
fn search_file_index(&self, query: &str, top_k: usize) -> Vec<Hit> {
    let idx = match index_db::FileIndex::open(&self.file_index_path()) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("[launcher] 打开文件索引失败: {e}");
            return Vec::new();
        }
    };
    let rows = idx.query(query, 800).unwrap_or_default();
    if rows.is_empty() {
        return Vec::new();
    }
    let now = chrono::Local::now().timestamp();
    // 把 FileRow 造成 Candidate 交现有打分（id 用行序，仅用于本次查询）。
    let candidates: Vec<Candidate> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| Candidate {
            id: i as u32,
            kind: r.kind,
            name: r.name.clone(),
            path: r.path.clone(),
            keywords: keyword::generate(&r.name),
            bias: 0.0,
        })
        .collect();
    let hist = self.history.lock().map(|h| h.clone()).unwrap_or_default();
    search::search(&candidates, &hist, query, top_k, now)
}
```

> 注：`search::search` 返回的 `Hit.id` 是候选行序，不能用于 `candidate(id)` 启动；因此文件命中的启动改走 `path`。见 Step 3b。

- [ ] **Step 3b: 文件命中按 path 启动（改 `candidate`/launch 链路）**

`LauncherState::candidate(id)` 仅对内存候选有效。文件命中前端拿到的是 `path`，启动命令 `launcher_launch` 已支持按路径 open（核对 `launch.rs`：若其按 id 取 candidate，则改为前端传 `path` 或在 Hit 内带 path——`Hit` 已含 `path`，`launch.rs` 用 path 打开即可）。确认 `launch.rs` 的 open 分支使用 `hit.path`；文件夹用 reveal/open 目录。

- [ ] **Step 4: `rebuild` 同时刷文件索引**

```rust
pub fn rebuild(&self, roots: &[LauncherRoot], full_disk: bool, extra_excludes: &[String]) {
    // 1) 现有内存候选（程序/UWP/命令 + 关闭全盘时的根目录文件）照旧
    // 2) 若 full_disk：开后台扫描写库
    if full_disk {
        let mut idx = match index_db::FileIndex::open(&self.file_index_path()) {
            Ok(i) => i,
            Err(e) => { eprintln!("[launcher] 文件索引打开失败: {e}"); return; }
        };
        let n = scan::scan_files_to_index(&mut idx, extra_excludes);
        eprintln!("[launcher] 全盘索引就绪 {n} 条");
    }
}
```

调用方 `rebuild_async` / `start` 从设置读 `full_disk` 与 `extra_excludes`（见 Task 7）。

- [ ] **Step 5: 运行确认通过 + 全量回归**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 全部 PASS（含既有 63 项）。

- [ ] **Step 6: 提交并推送**

```bash
git add src-tauri/src/services/launcher/mod.rs src-tauri/src/services/launcher/launch.rs
git commit -m "feat(launcher): 查询合并文件索引命中，后台重建刷全盘索引"
git push origin feature/tauri-vue
```

---

## Task 7：设置项 D5（后端）

**Files:**

- Modify: `src-tauri/src/domain/models.rs`、`src-tauri/src/data/settings.rs`
- Modify: `src-tauri/src/services/launcher/mod.rs`（`roots_from_settings` 旁加读开关）

**Interfaces:**

- Produces: `Settings.launcher_full_disk_index: bool`（默认 true）、`Settings.launcher_extra_excludes: String`（逗号分隔，默认空）；`pub fn full_disk_from_settings(app) -> (bool, Vec<String>)`

- [ ] **Step 1: 写失败测试（默认值 + 解析）**

`settings.rs` 测试模块：

```rust
#[test]
fn launcher_full_disk_defaults_on_and_parses_excludes() {
    let d = Settings::default();
    assert!(*d.launcher_full_disk_index());
    // 解析逗号分隔排除目录
    assert_eq!(parse_excludes("a, b ,,c"), vec!["a","b","c"]);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml launcher_full_disk_defaults`
Expected: FAIL。

- [ ] **Step 3: 实现**

`models.rs`：`Settings` 结构加两字段（getset 模式沿用既有），默认函数：

```rust
fn default_launcher_full_disk_index() -> bool { true }
```

字段：

```rust
#[serde(default = "default_launcher_full_disk_index")]
launcher_full_disk_index: bool,
#[serde(default)]
launcher_extra_excludes: String,
```

`Default` 里补 `launcher_full_disk_index: true, launcher_extra_excludes: String::new()`。
`settings.rs` 的 KV 读写表补两键（照既有 `island_*` 模式），并加：

```rust
pub fn parse_excludes(raw: &str) -> Vec<String> {
    raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string()).collect()
}
```

`launcher/mod.rs` 加：

```rust
pub fn full_disk_from_settings(app: &AppHandle) -> (bool, Vec<String>) {
    match app.state::<AppState>().lock_store().and_then(|s| s.get_settings()) {
        Ok(s) => (*s.launcher_full_disk_index(), crate::data::settings::parse_excludes(s.launcher_extra_excludes())),
        Err(_) => (true, Vec::new()),
    }
}
```

`rebuild_async`/`start` 改为读该值传给 `rebuild`。

- [ ] **Step 4: 运行确认通过 + 回归**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 全部 PASS。

- [ ] **Step 5: 提交并推送**

```bash
git add src-tauri/src/domain/models.rs src-tauri/src/data/settings.rs src-tauri/src/services/launcher/mod.rs
git commit -m "feat(launcher): 设置项 全盘索引开关/额外排除目录（默认开）"
git push origin feature/tauri-vue
```

---

## Task 8：设置页 UI D5（前端）+ 重建按钮

**Files:**

- Modify: `src/windows/Main/SettingsView.vue`
- Modify: `src/service/tauri.ts`（若设置字段走通用 settings 则仅补类型；重建复用现有 `launcher.rebuild`）

**Interfaces:**

- Consumes: 现有 `api.launcher.rebuild()`、设置读写通道

- [ ] **Step 1: 确认现有设置读写与 launcher.rebuild**

Run: `grep -n "launcher\|island_enabled\|settings" src/windows/Main/SettingsView.vue | head`
核对启动器分区位置与设置双向绑定写法（照 island 分区）。

- [ ] **Step 2: 加三项控件**

在启动器分区（照既有 `setting-row` 结构）加：

```html
<label class="setting-row"
  ><span>全盘文件索引（搜索所有文件/文件夹）</span>
  <input type="checkbox" v-model="settings.launcher_full_disk_index" @change="save"
/></label>
<label class="setting-row"
  ><span>额外排除目录（逗号分隔）</span>
  <input type="text" v-model="settings.launcher_extra_excludes" @change="save" placeholder="如 tmp, backup"
/></label>
<div class="setting-row"><span>文件索引</span> <button class="btn tiny" @click="rebuildIndex">重建索引</button></div>
```

脚本：

```ts
async function rebuildIndex(): Promise<void> {
  await api.launcher.rebuild()
  toast('已开始重建索引')
}
```

`settings` 类型补 `launcher_full_disk_index: boolean` 与 `launcher_extra_excludes: string`（在前端 Settings 类型定义处，照 island 字段）。

- [ ] **Step 3: 校验**

Run: `npx vue-tsc --noEmit` → 0 错误；`npx prettier --check "src/windows/Main/SettingsView.vue" "src/service/tauri.ts"` → 通过。

- [ ] **Step 4: 提交并推送**

```bash
git add src/windows/Main/SettingsView.vue src/service/tauri.ts src/composables/useData.ts
git commit -m "feat(launcher): 设置页 全盘索引开关/排除目录/重建索引按钮"
git push origin feature/tauri-vue
```

---

## Task 9：全量验收

- [ ] **Step 1: 后端全绿**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` → 全部 PASS；`cargo fmt --manifest-path src-tauri/Cargo.toml`。

- [ ] **Step 2: 前端全绿**

Run: `npx vue-tsc --noEmit`（0 错误）、`npx prettier --check "src/**/*.{vue,ts,css}"`（通过）、`npx vite build`（成功）。

- [ ] **Step 3: 实机核对（用户）**

启动 `pnpm tauri:dev`，逐项：全盘搜到深层文件、拼音/简拼/错字命中、结果顺序为应用→文件夹→文件、回车打开第一项、文件夹命中打开目录、设置开关关→回退根目录模式、重建索引、首次索引不阻塞交互、空闲内存 ≤120MB。

- [ ] **Step 4: 提交验收记录**

在本文件末尾「验收记录」填结果并提交：

```bash
git commit -am "docs(launcher): 全盘索引实机验收记录"
git push origin feature/tauri-vue
```

## 验收记录

**验收日期**：2026-09-09　**分支**：feature/tauri-vue

### 自动化校验（全绿）

| 项                                             | 结果                                                                                                                 |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `cargo test`（src-tauri）                      | ✅ 70 passed / 0 failed（含新增 drives/exclude/index_db/scan-to-index/strict_group/state_merge/parse_excludes 7 项） |
| `cargo fmt -- --check`                         | ✅ 通过                                                                                                              |
| `npx vue-tsc --noEmit`                         | ✅ 0 错误                                                                                                            |
| `npx prettier --check "src/**/*.{vue,ts,css}"` | ✅ 通过                                                                                                              |
| `npx vite build`                               | ✅ built（仅 chunk >500KB 信息级提示）                                                                               |

### 任务完成对照

| Task | 内容                                                      | 状态 |
| ---- | --------------------------------------------------------- | ---- |
| 1    | 固定磁盘枚举 drives.rs                                    | ✅   |
| 2    | 目录排除规则 exclude.rs                                   | ✅   |
| 3    | SQLite 文件索引 index_db.rs（FTS5 trigram，单测证实可用） | ✅   |
| 4    | 全盘扫描写入索引 scan_files_to_index                      | ✅   |
| 5    | D4 严格分组排序（应用组→文件夹→文件）                     | ✅   |
| 6    | LauncherState 编排：查询合并文件命中、按 path 启动        | ✅   |
| 7    | 设置项后端（全盘开关/排除目录，默认开）                   | ✅   |
| 8    | 设置页 UI（开关/排除/重建入口）                           | ✅   |

### 实机核对（待用户在 `pnpm tauri:dev` 中确认）

本环境无显示器，以下交互项留待用户实机验收（代码路径均已实现并通过静态校验与构建）：
全盘搜到深层文件、拼音/简拼/错字命中、结果顺序为应用→文件夹→文件、回车打开第一项、
文件夹命中打开目录、设置关闭全盘索引后回退根目录模式、「立即重建」生效、首次索引不阻塞交互、空闲内存 ≤120MB。

### 说明

- 「回车默认选中并打开第一项」原已满足（`active=0` + Enter 执行），本次未改该行为。
- 未做项均为设计 D6 明确排除：USN/MFT 实时监控、文件内容全文检索、网络盘/可移动盘。
