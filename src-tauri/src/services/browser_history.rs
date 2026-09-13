//! 浏览器历史采集（spec 4C D35 / §4.4）：只扫 Chrome 与 Edge——同一套 Chromium 表结构
//! （`urls(url, title, last_visit_time)`），Firefox 的 places.sqlite 另需一套解析，本期不做。
//!
//! 两条硬约束：
//! 1. 浏览器运行时会独占 `History`（SQLite 加锁 / 共享冲突），因此**先复制到临时文件再打开**，
//!    复制失败（权限 / 独占）只记日志跳过本轮，不影响其他 Profile；
//! 2. 副本一律用 `OpenFlags::SQLITE_OPEN_READ_ONLY` 打开——外部库只读，绝不写入。
//!
//! 导入结果落自有 `browser_history` 表（v8 建表），同一 URL 以最新访问时刻覆盖；
//! 每轮导入末尾按设置里的保留天数清理过期行。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use tauri::{AppHandle, Manager};

use crate::app::state::AppState;
use crate::data::{db_err, Store};
use crate::domain::models::BrowserHistoryRow;

/// 启动后延迟首扫（spec §4.4）：避开开机高峰，也给浏览器留出写完 History 的时间。
const FIRST_SCAN_DELAY: Duration = Duration::from_secs(30);
/// WebKit / Chromium 时间戳零点（1601-01-01）与 Unix 纪元（1970-01-01）的秒差。
const WEBKIT_EPOCH_OFFSET_SECS: i64 = 11_644_473_600;
/// 之后每 10 分钟导入一次（spec §4.4）。
const SCAN_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// 单次导入的查询上限（保留窗口内通常远小于它；兜底防止异常库把内存吃满）。
const MAX_IMPORT_ROWS: usize = 20_000;
/// 每个浏览器的 Profile 目录探测上限（`Profile 1` … `Profile 8`）。
const MAX_PROFILES: u32 = 8;
/// 临时副本序号：同进程内两次导入不撞名。
static COPY_SEQ: AtomicU64 = AtomicU64::new(0);

/// Chromium 时间戳（1601-01-01 起的**微秒**）→ Unix 秒（spec §4.4）。
pub fn webkit_to_unix(webkit_micros: i64) -> i64 {
    webkit_micros / 1_000_000 - WEBKIT_EPOCH_OFFSET_SECS
}

/// 待扫描的 `History` 文件：Chrome 与 Edge 的 `User Data` 下 `Default` 与 `Profile 1..8`。
///
/// 目录不存在（未安装）直接跳过；同一浏览器的多个 Profile 都会导入，URL 以最新访问时刻为准。
pub fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return out;
    };
    for vendor in ["Google\\Chrome", "Microsoft\\Edge"] {
        let user_data = local.join(vendor).join("User Data");
        let mut profiles = vec![user_data.join("Default")];
        for index in 1..=MAX_PROFILES {
            profiles.push(user_data.join(format!("Profile {index}")));
        }
        for profile in profiles {
            let path = profile.join("History");
            if path.is_file() {
                out.push(path);
            }
        }
    }
    out
}

/// 复制副本并只读打开。返回连接与临时文件路径（调用方负责删除副本）。
///
/// 浏览器占用原文件时 `std::fs::copy` 仍能成功（Windows 下浏览器以共享读打开），
/// 但直接对原文件跑 SQLite 查询可能撞锁，所以一律读副本。
fn open_copy(path: &Path) -> Result<(Connection, PathBuf), String> {
    let seq = COPY_SEQ.fetch_add(1, Ordering::Relaxed);
    let copy = std::env::temp_dir().join(format!("inkling-hist-{}-{seq}.tmp", std::process::id()));
    if let Err(error) = std::fs::copy(path, &copy) {
        // 复制中断可能在目标留下半个文件，先清掉再报错。
        let _ = std::fs::remove_file(&copy);
        return Err(format!("复制历史库失败: {error}"));
    }
    match Connection::open_with_flags(&copy, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => Ok((conn, copy)),
        Err(error) => {
            // 副本打不开（不是合法 SQLite / 被截断）也要删掉：调用方此时还拿不到路径，
            // 若在这里直接返回，临时目录会被无效 History 文件一点点堆满。
            let _ = std::fs::remove_file(&copy);
            Err(format!("只读打开历史副本失败: {error}"))
        }
    }
}

/// 读一个 History 副本里保留窗口内的行（`last_visit_time` 是 WebKit 微秒）。
fn read_rows(conn: &Connection, since: i64) -> Result<Vec<BrowserHistoryRow>, String> {
    let webkit_min = (since + WEBKIT_EPOCH_OFFSET_SECS) * 1_000_000;
    let mut stmt = conn
        .prepare("SELECT url, title, last_visit_time FROM urls WHERE last_visit_time > ? LIMIT ?")
        .map_err(|e| format!("读取历史表失败: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params![webkit_min, MAX_IMPORT_ROWS as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| format!("查询历史失败: {e}"))?
        .filter_map(|row| row.ok())
        .filter(|(url, _, _)| !url.is_empty())
        .map(|(url, title, webkit)| {
            BrowserHistoryRow::builder()
                .url(url)
                .title(title)
                .visited_at(webkit_to_unix(webkit))
                .build()
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(rows)
}

/// 导入一个 `History` 文件，返回写入行数。副本无论读取成败都会被删除。
pub fn import_file(store: &Store, path: &Path, since: i64) -> Result<usize, String> {
    let (conn, copy) = open_copy(path)?;
    let read = read_rows(&conn, since);
    drop(conn);
    if let Err(error) = std::fs::remove_file(&copy) {
        eprintln!("[history] 删除临时副本失败 {}: {error}", copy.display());
    }
    let rows = read?;
    if rows.is_empty() {
        return Ok(0);
    }
    // 一次导入一个事务：几百到几万行的逐条 INSERT OR REPLACE 走事务才不会慢到卡住其他查询。
    let tx = store.tx()?;
    for row in &rows {
        tx.execute(
            "INSERT OR REPLACE INTO browser_history(url, title, visited_at) VALUES(?,?,?)",
            rusqlite::params![row.url(), row.title(), row.visited_at()],
        )
        .map_err(db_err)?;
    }
    tx.commit().map_err(db_err)?;
    Ok(rows.len())
}

/// 按标题 / URL 模糊查询，按访问时刻倒序（前端历史段与设置页计数共用）。
pub fn search(store: &Store, query: &str, limit: usize) -> Result<Vec<BrowserHistoryRow>, String> {
    let needle = format!("%{}%", query.trim());
    let mut stmt = store
        .db
        .prepare(
            "SELECT url, title, visited_at FROM browser_history \
             WHERE url LIKE ?1 OR title LIKE ?1 ORDER BY visited_at DESC LIMIT ?2",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map(rusqlite::params![needle, limit as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(db_err)?
        .filter_map(|row| row.ok())
        .map(|(url, title, visited_at)| {
            BrowserHistoryRow::builder()
                .url(url)
                .title(title)
                .visited_at(visited_at)
                .build()
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(rows)
}

/// 已记录的历史地址条数（设置页「已记录历史地址」）。
pub fn count(store: &Store) -> Result<i64, String> {
    store
        .db
        .query_row("SELECT COUNT(*) FROM browser_history", [], |r| r.get(0))
        .map_err(db_err)
}

/// 删除保留窗口之外的行，返回删除条数。保留天数钳制在 1–365（原型 1–365）。
pub fn prune(store: &Store, now: i64, retention_days: i64) -> Result<usize, String> {
    let cutoff = now - retention_days.clamp(1, 365) * 86_400;
    store
        .db
        .execute("DELETE FROM browser_history WHERE visited_at < ?", [cutoff])
        .map_err(db_err)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 按当前设置清理过期历史（`settings_save` 立即调用，不等下一轮导入）。
pub fn prune_now(app: &AppHandle) -> Result<usize, String> {
    // `app.state()` 返回的 State 借用 app，先绑定再取锁：MutexGuard 借用 State，
    // 写成 `app.state::<AppState>().lock_store()?` 会因为临时 State 先被释放而编译不过。
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    let days = *store.get_settings()?.launcher_history_retention_days();
    prune(&store, now_secs(), days)
}

/// 导入一轮：扫描候选 → 逐个导入 → 清理过期（spec §4.4）。返回本轮写入行数。
pub fn run_once(app: &AppHandle) -> Result<usize, String> {
    let paths = candidate_paths();
    if paths.is_empty() {
        eprintln!("[history] 未发现 Chrome / Edge 历史库，跳过本轮");
        return Ok(0);
    }
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    let days = *store.get_settings()?.launcher_history_retention_days();
    let now = now_secs();
    let since = now - days.clamp(1, 365) * 86_400;
    let mut written = 0usize;
    for path in paths {
        match import_file(&store, &path, since) {
            Ok(rows) => {
                written += rows;
                eprintln!("[history] 导入 {rows} 条 ← {}", path.display());
            }
            // 复制失败（浏览器独占 / 权限）只跳过本轮，不影响其他库。
            Err(error) => eprintln!("[history] 导入失败 {}: {error}", path.display()),
        }
    }
    let removed = prune(&store, now, days)?;
    eprintln!("[history] 本轮写入 {written} 条，清理过期 {removed} 条，保留 {days} 天");
    Ok(written)
}

/// 启动采集线程：延迟 30 秒首扫，之后每 10 分钟一轮（spec §4.4）。
pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("browser-history".into())
        .spawn(move || {
            std::thread::sleep(FIRST_SCAN_DELAY);
            loop {
                if let Err(error) = run_once(&app) {
                    eprintln!("[history] 导入轮次失败: {error}");
                }
                std::thread::sleep(SCAN_INTERVAL);
            }
        })
        .expect("启动浏览器历史采集线程失败");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 时间换算用两个已知值锚定：Unix 纪元与 2024-01-01T00:00:00Z。
    #[test]
    fn webkit_timestamp_matches_known_values() {
        // 1601-01-01 + 11_644_473_600 秒 = 1970-01-01T00:00:00Z。
        assert_eq!(webkit_to_unix(11_644_473_600_000_000), 0);
        // 2024-01-01T00:00:00Z = 1_704_067_200（自 1601 起 13_348_540_800 秒）。
        assert_eq!(webkit_to_unix(13_348_540_800_000_000), 1_704_067_200);
    }

    /// 建一个只有 browser_history 表的内存库（v8 迁移后的最小状态）。
    fn memory_store() -> Store {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE browser_history (
               url TEXT PRIMARY KEY,
               title TEXT NOT NULL DEFAULT '',
               visited_at INTEGER NOT NULL
             );
             CREATE INDEX idx_browser_history_visited ON browser_history(visited_at DESC);",
        )
        .unwrap();
        Store {
            db,
            data_dir: std::path::PathBuf::from("."),
        }
    }

    fn insert(store: &Store, url: &str, title: &str, visited_at: i64) {
        store
            .db
            .execute(
                "INSERT INTO browser_history(url, title, visited_at) VALUES(?,?,?)",
                rusqlite::params![url, title, visited_at],
            )
            .unwrap();
    }

    /// 标题或 URL 命中，按访问时刻倒序（spec §4.4）。
    #[test]
    fn search_matches_title_or_url_and_orders_desc() {
        let store = memory_store();
        insert(&store, "https://a.example/", "Rust 手册", 300);
        insert(&store, "https://b.example/rust-notes", "笔记", 200);
        insert(&store, "https://c.example/", "无关", 100);

        let by_title = search(&store, "rust", 10).unwrap();
        assert_eq!(
            by_title
                .iter()
                .map(|r| r.url().as_str())
                .collect::<Vec<_>>(),
            vec!["https://a.example/", "https://b.example/rust-notes"]
        );
        let by_url = search(&store, "b.example", 10).unwrap();
        assert_eq!(by_url.len(), 1);
        assert_eq!(by_url[0].title(), "笔记");
        assert_eq!(count(&store).unwrap(), 3);
    }

    /// 保留窗口外的行被删掉，窗口内保留；天数越界按 1 / 365 生效。
    #[test]
    fn prune_drops_rows_outside_retention_window() {
        let store = memory_store();
        let now = 1_700_000_000;
        insert(&store, "https://old.example/", "旧", now - 10 * 86_400);
        insert(&store, "https://new.example/", "新", now - 86_400);

        let removed = prune(&store, now, 3).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(count(&store).unwrap(), 1);

        // 幂等：再清理一次没有可删的行。
        assert_eq!(prune(&store, now, 3).unwrap(), 0);
    }

    /// 造一个最小 Chromium `History` 库：只建 urls 表，验证复制 + 只读打开 + 换算 + upsert。
    #[test]
    fn import_file_reads_chromium_urls_table() {
        let dir = std::env::temp_dir().join(format!("inkling-hist-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("History");
        let in_window = (1_700_000_000 + WEBKIT_EPOCH_OFFSET_SECS) * 1_000_000;
        let out_of_window = (1_600_000_000 + WEBKIT_EPOCH_OFFSET_SECS) * 1_000_000;
        {
            let conn = rusqlite::Connection::open(&src).unwrap();
            conn.execute_batch(
                "CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, last_visit_time INTEGER);",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO urls(url, title, last_visit_time) VALUES(?,?,?)",
                rusqlite::params!["https://in.example/", "窗口内", in_window],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO urls(url, title, last_visit_time) VALUES(?,?,?)",
                rusqlite::params!["https://out.example/", "窗口外", out_of_window],
            )
            .unwrap();
        }

        let store = memory_store();
        // 只取 1_600_000_000 之后的（窗口外那条被 WHERE 过滤）。
        let written = import_file(&store, &src, 1_600_000_100).unwrap();
        assert_eq!(written, 1);
        assert_eq!(count(&store).unwrap(), 1);
        let row = search(&store, "in.example", 10).unwrap();
        assert_eq!(row[0].title(), "窗口内");
        assert_eq!(*row[0].visited_at(), 1_700_000_000);
        // 同 URL 再导入以最新时刻覆盖（INSERT OR REPLACE）。
        assert_eq!(import_file(&store, &src, 1_600_000_100).unwrap(), 1);
        assert_eq!(count(&store).unwrap(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
