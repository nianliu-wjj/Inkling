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

/// 临时副本的 RAII 守卫：离开作用域（正常返回、`?` 提前返回、panic 展开）即删除副本文件。
///
/// 历史库副本可能有几十 MB，复制中断的半截文件 / 打开失败的无效库 / 读取途中的 panic
/// 都会留下残渣；用 Drop 兜底比在每个出口手写 `remove_file` 更不容易漏。
struct TempCopy(PathBuf);

impl TempCopy {
    /// 副本路径（交给 SQLite 打开）。
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempCopy {
    fn drop(&mut self) {
        // 删除失败只吞掉：临时目录的残渣不值得打断导入（复制文件名带序号，不会重名冲突）。
        let _ = std::fs::remove_file(&self.0);
    }
}

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

/// 复制副本并只读打开。返回连接与临时副本守卫（守卫离开作用域即删除副本）。
///
/// 浏览器占用原文件时 `std::fs::copy` 仍能成功（Windows 下浏览器以共享读打开），
/// 但直接对原文件跑 SQLite 查询可能撞锁，所以一律读副本。
fn open_copy(path: &Path) -> Result<(Connection, TempCopy), String> {
    let seq = COPY_SEQ.fetch_add(1, Ordering::Relaxed);
    // 先建守卫再复制：复制中断留下的半个文件由 Drop 清掉，不必等调用方拿到路径。
    let copy = TempCopy(
        std::env::temp_dir().join(format!("inkling-hist-{}-{seq}.tmp", std::process::id())),
    );
    if let Err(error) = std::fs::copy(path, copy.path()) {
        return Err(format!("复制历史库失败: {error}"));
    }
    match Connection::open_with_flags(copy.path(), OpenFlags::SQLITE_OPEN_READ_ONLY) {
        // 守卫随连接一起交给调用方：删副本的时机由它的作用域决定。
        Ok(conn) => Ok((conn, copy)),
        // 副本打不开（不是合法 SQLite / 被截断）：`copy` 在此处被 Drop 删除，
        // 临时目录不会被无效 History 文件一点点堆满。
        Err(error) => Err(format!("只读打开历史副本失败: {error}")),
    }
}

/// 读一个 History 副本里保留窗口内的行（`last_visit_time` 是 WebKit 微秒）。
fn read_rows(conn: &Connection, since: i64) -> Result<Vec<BrowserHistoryRow>, String> {
    let webkit_min = (since + WEBKIT_EPOCH_OFFSET_SECS) * 1_000_000;
    let mut stmt = conn
        .prepare("SELECT url, title, last_visit_time FROM urls WHERE last_visit_time > ? LIMIT ?")
        .map_err(|e| format!("读取历史表失败: {e}"))?;
    // 解码失败（URL 列不是文本 / 时间戳类型异常）的行只丢自己，最后按数量记一条日志，
    // 不像以前那样静默丢弃、事后无法分辨「库里没有」与「解析失败」。
    let mut skipped = 0usize;
    let rows = stmt
        .query_map(rusqlite::params![webkit_min, MAX_IMPORT_ROWS as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                // `urls.title` 可空（Chromium 允许 NULL）：必须读 `Option` 再降级成空串，
                // 与 DTO 约定一致（前端回退显示 URL）；直接 `get::<String>` 会整行报错，
                // 连 URL 一起被丢掉。
                r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| format!("查询历史失败: {e}"))?
        .filter_map(|row| match row {
            Ok(item) => Some(item),
            Err(_) => {
                skipped += 1;
                None
            }
        })
        .filter(|(url, _, _)| !url.is_empty())
        .map(|(url, title, webkit)| {
            BrowserHistoryRow::builder()
                .url(url)
                .title(title)
                .visited_at(webkit_to_unix(webkit))
                .build()
        })
        .collect::<Result<Vec<_>, String>>()?;
    if skipped > 0 {
        eprintln!("[history] 跳过 {skipped} 条无法解析的历史行");
    }
    Ok(rows)
}

/// 锁外的重活：复制副本 → 只读解析，返回待写入的行。
///
/// 与 [`import_rows`] 分开是为了锁纪律：调用方（[`run_once`]）要在**不持 store 锁**的
/// 状态下完成外部文件 IO（几十 MB 的复制可能几十毫秒到几秒），只把 INSERT 放回锁内。
fn read_file_rows(path: &Path, since: i64) -> Result<Vec<BrowserHistoryRow>, String> {
    let (conn, copy) = open_copy(path)?;
    let rows = read_rows(&conn, since);
    // 先显式关连接再丢副本：Windows 下 SQLite 仍持有句柄时删文件会失败。
    drop(conn);
    // 副本在这里（或上面的提前返回、panic 展开）由 TempCopy::drop 删除。
    drop(copy);
    rows
}

/// 锁内的写：一个事务写完一个历史库的全部行，返回写入条数。
fn import_rows(store: &Store, rows: &[BrowserHistoryRow]) -> Result<usize, String> {
    if rows.is_empty() {
        return Ok(0);
    }
    // 一次导入一个事务：几百到几万行的逐条 INSERT OR REPLACE 走事务才不会慢到卡住其他查询。
    let tx = store.tx()?;
    for row in rows {
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
///
/// 锁纪律：`Store` 是「单连接 + 单个 Mutex」，IPC 里每个 `lock_store` 都排在它后面，
/// 所以这里只在两处短暂持锁——开头读一次设置、每个库的写事务与末尾清理；
/// 复制 / 只读打开外部 History（几十 MB，耗时不可控）全部放在锁外，
/// 否则整轮导入期间设置页与启动台查询都会被堵住（对照 `services::launcher` 读设置的做法）。
pub fn run_once(app: &AppHandle) -> Result<usize, String> {
    let paths = candidate_paths();
    if paths.is_empty() {
        eprintln!("[history] 未发现 Chrome / Edge 历史库，跳过本轮");
        return Ok(0);
    }
    let state = app.state::<AppState>();
    // 第一段临界区：只读一次设置（保留天数 + 当前时刻），拿到值立即释放锁。
    let (days, now) = {
        let store = state.lock_store()?;
        (
            *store.get_settings()?.launcher_history_retention_days(),
            now_secs(),
        )
    };
    let since = now - days.clamp(1, 365) * 86_400;
    let mut written = 0usize;
    for path in paths {
        // 锁外：复制副本 + 只读解析，通常是本轮最慢的一步。
        let rows = match read_file_rows(&path, since) {
            Ok(rows) => rows,
            // 复制失败（浏览器独占 / 权限）或库不合法只跳过本轮，不影响其他库。
            Err(error) => {
                eprintln!("[history] 导入失败 {}: {error}", path.display());
                continue;
            }
        };
        // 第二段临界区：只包含这个库的 INSERT，几毫秒级，写完（或出错）立刻释放。
        let imported = {
            let store = state.lock_store()?;
            import_rows(&store, &rows)
        };
        match imported {
            Ok(count) => {
                written += count;
                eprintln!("[history] 导入 {count} 条 ← {}", path.display());
            }
            // 单个库写失败（磁盘 / 库损坏）也只跳过它，其余库继续导入。
            Err(error) => eprintln!("[history] 写入历史失败 {}: {error}", path.display()),
        }
    }
    // 第三段临界区：按保留天数清理过期行。
    let removed = {
        let store = state.lock_store()?;
        prune(&store, now, days)?
    };
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

    /// 生产代码里「锁外读 + 锁内写」是分开调用的（见 `run_once` 的锁纪律）；
    /// 测试不涉及并发，这里合成一个入口，等价于原先的 `import_file`。
    fn import_file(store: &Store, path: &Path, since: i64) -> Result<usize, String> {
        let rows = read_file_rows(path, since)?;
        import_rows(store, &rows)
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

    /// `urls.title` 可空（Chromium 允许 NULL）：NULL 标题降级成空串照常导入，不能整行（含 URL）被丢；
    /// 顺带确认临时副本由 RAII 删除，不留残渣。
    #[test]
    fn import_file_tolerates_null_title_and_removes_copy() {
        let dir = std::env::temp_dir().join(format!("inkling-hist-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("History");
        let visited = (1_700_000_000 + WEBKIT_EPOCH_OFFSET_SECS) * 1_000_000;
        {
            let conn = rusqlite::Connection::open(&src).unwrap();
            conn.execute_batch(
                "CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, last_visit_time INTEGER);",
            )
            .unwrap();
            // 一条正常标题 + 一条 NULL 标题：两条都要导入。
            conn.execute(
                "INSERT INTO urls(url, title, last_visit_time) VALUES(?,?,?)",
                rusqlite::params!["https://titled.example/", "有标题", visited],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO urls(url, title, last_visit_time) VALUES(?, NULL, ?)",
                rusqlite::params!["https://null-title.example/", visited],
            )
            .unwrap();
        }

        let store = memory_store();
        assert_eq!(import_file(&store, &src, 1_600_000_100).unwrap(), 2);
        assert_eq!(count(&store).unwrap(), 2);
        let rows = search(&store, "null-title", 10).unwrap();
        assert_eq!(rows.len(), 1, "NULL 标题的行不能被整行丢弃");
        assert_eq!(rows[0].title(), "", "NULL 标题降级为空串（DTO 约定）");
        assert_eq!(*rows[0].visited_at(), 1_700_000_000);

        // 副本文件由 TempCopy::drop 删除：守卫一离开作用域就不该再有这个文件。
        let (conn, copy) = open_copy(&src).unwrap();
        let copy_path = copy.path().to_path_buf();
        assert!(
            copy_path.is_file(),
            "复制后副本应存在: {}",
            copy_path.display()
        );
        // 先关连接再丢守卫：Windows 下 SQLite 仍持有句柄时删文件会失败。
        drop(conn);
        drop(copy);
        assert!(
            !copy_path.exists(),
            "TempCopy::drop 应删除临时副本: {}",
            copy_path.display()
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
