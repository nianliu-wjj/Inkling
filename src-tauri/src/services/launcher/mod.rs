//! 启动器搜索（spec: docs/superpowers/specs/2026-09-08-launcher-search-design.md）。
//!
//! 模块划分：
//! - `model`：Candidate / Kind / Hit；
//! - `keyword`：关键字生成管道（纯函数）；
//! - `score`：标准检索引擎（纯函数）；
//! - `history`：启动历史与查询亲和度加权（纯函数 + JSON 落盘）；
//! - `snapshot`：索引快照读写；
//! - `scan`：数据源扫描（程序 / UWP / 文件根目录 / 内置命令）；
//! - `search`：并行打分 + 加权 + Top-K；
//! - `launch`：打开 / 管理员 / 打开所在文件夹。
//!
//! 本文件是服务入口：`LauncherState`（索引代际、历史、L1 缓存）与后台重建线程。

pub mod history;
pub mod keyword;
pub mod launch;
pub mod model;
pub mod scan;
pub mod score;
pub mod search;
pub mod snapshot;

mod drives;
mod exclude;
mod index_db;

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::app::state::AppState;
use history::History;
use model::{Candidate, Hit};
use scan::LauncherRoot;

/// 后台全量重扫间隔。
const REBUILD_INTERVAL: Duration = Duration::from_secs(30 * 60);
/// L1 查询结果缓存容量（同一索引代际内有效）。
const L1_CAPACITY: usize = 64;
/// 返回给前端的条数。
pub const TOP_K: usize = 9;

/// 一代索引：候选 + 代际号 + 生成时间。重扫后整体替换，旧代际的缓存随之作废。
pub struct Index {
    pub candidates: Vec<Candidate>,
    pub generation: u64,
    pub created_at: i64,
}

/// L1 查询缓存：查询 → (代际, 结果) + FIFO 淘汰序列。
type QueryCache = (HashMap<String, (u64, Vec<Hit>)>, VecDeque<String>);

/// 启动器共享状态（`app.manage`）。
pub struct LauncherState {
    index: RwLock<Arc<Index>>,
    history: Mutex<History>,
    /// L1：查询 → (代际, 结果)。FIFO 淘汰。
    cache: Mutex<QueryCache>,
    dir: PathBuf,
    rebuilding: AtomicBool,
}

/// 索引状态（设置页展示）。
#[derive(Debug, Clone, Serialize)]
pub struct LauncherStatus {
    pub count: usize,
    pub generation: u64,
    pub created_at: i64,
    pub rebuilding: bool,
}

impl LauncherState {
    /// `dir` = app_data_dir/launcher。同步加载历史与快照（快照通常 <200ms）。
    pub fn new(dir: PathBuf) -> Self {
        let history = History::load(&dir.join("history.json"));
        let (candidates, created_at) = snapshot::load(&dir.join("index.json")).unwrap_or_default();
        if !candidates.is_empty() {
            eprintln!(
                "[launcher] 已加载快照 {} 条（生成于 {created_at}）",
                candidates.len()
            );
        }
        Self {
            index: RwLock::new(Arc::new(Index {
                candidates,
                generation: 1,
                created_at,
            })),
            history: Mutex::new(history),
            cache: Mutex::new((HashMap::new(), VecDeque::new())),
            dir,
            rebuilding: AtomicBool::new(false),
        }
    }

    fn current(&self) -> Arc<Index> {
        self.index
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }

    pub fn status(&self) -> LauncherStatus {
        let index = self.current();
        LauncherStatus {
            count: index.candidates.len(),
            generation: index.generation,
            created_at: index.created_at,
            rebuilding: self.rebuilding.load(Ordering::Relaxed),
        }
    }

    /// 搜索：先查 L1，未命中则并行打分。
    pub fn search(&self, query: &str) -> Vec<Hit> {
        let normalized = search::normalize_query(query);
        let index = self.current();
        if let Ok(cache) = self.cache.lock() {
            if let Some((generation, hits)) = cache.0.get(&normalized) {
                if *generation == index.generation {
                    return hits.clone();
                }
            }
        }
        let now = now_secs();
        let mem_hits = match self.history.lock() {
            Ok(history) => search::search(&index.candidates, &history, &normalized, TOP_K, now),
            Err(_) => search::search(
                &index.candidates,
                &History::default(),
                &normalized,
                TOP_K,
                now,
            ),
        };
        // 文件/文件夹命中来自磁盘索引（全盘模式）；索引为空时返回空，不影响内存结果。
        let file_hits = self.search_file_index(&normalized, TOP_K);
        let hits = search::merge_and_rank(mem_hits, file_hits, TOP_K);
        if let Ok(mut cache) = self.cache.lock() {
            let (map, order) = &mut *cache;
            if !map.contains_key(&normalized) {
                order.push_back(normalized.clone());
                while order.len() > L1_CAPACITY {
                    if let Some(old) = order.pop_front() {
                        map.remove(&old);
                    }
                }
            }
            map.insert(normalized, (index.generation, hits.clone()));
        }
        hits
    }

    /// 文件索引库路径。
    fn file_index_path(&self) -> PathBuf {
        self.dir.join("launcher-index.db")
    }

    /// 从磁盘索引粗筛并用 score 精排，产出文件/文件夹命中。打不开或空则返回空。
    fn search_file_index(&self, query: &str, top_k: usize) -> Vec<Hit> {
        if query.is_empty() {
            return Vec::new();
        }
        let idx = match index_db::FileIndex::open(&self.file_index_path()) {
            Ok(i) => i,
            Err(error) => {
                eprintln!("[launcher] 打开文件索引失败: {error}");
                return Vec::new();
            }
        };
        let rows = idx.query(query, 800).unwrap_or_default();
        if rows.is_empty() {
            return Vec::new();
        }
        // 把命中行造成临时 Candidate 交现有打分（id 仅本次查询用；文件启动走 path）。
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
        let now = now_secs();
        match self.history.lock() {
            Ok(history) => search::search(&candidates, &history, query, top_k, now),
            Err(_) => search::search(&candidates, &History::default(), query, top_k, now),
        }
    }

    /// 按 id 取候选（启动用）。
    pub fn candidate(&self, id: u32) -> Option<Candidate> {
        self.current().candidates.get(id as usize).cloned()
    }

    /// 记录一次启动并落盘历史；清空 L1（排序依赖历史）。
    pub fn record_launch(&self, path: &str, query: &str) {
        let today = crate::data::local_date_key(chrono::Utc::now());
        if let Ok(mut history) = self.history.lock() {
            history.record(path, query, &today, now_secs());
            if let Err(error) = history.save(&self.dir.join("history.json")) {
                eprintln!("[launcher] 保存历史失败: {error}");
            }
        }
        if let Ok(mut cache) = self.cache.lock() {
            cache.0.clear();
            cache.1.clear();
        }
    }

    /// 全量重扫并替换索引（阻塞调用方线程；由后台线程或命令的 spawn 调用）。
    /// `full_disk` 为真：内存只收程序/UWP/命令（不扫根目录文件），文件/文件夹改扫全盘 SQLite 索引。
    pub fn rebuild(&self, roots: &[LauncherRoot], full_disk: bool, extra_excludes: &[String]) {
        if self
            .rebuilding
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            eprintln!("[launcher] 已有重建在进行，跳过");
            return;
        }
        // 全盘模式下内存不再扫根目录文件（文件走 SQLite），传空根目录给内存扫描。
        let mem_roots: &[LauncherRoot] = if full_disk { &[] } else { roots };
        let (candidates, report) = scan::scan_all(mem_roots);
        let created_at = now_secs();
        let generation = self.current().generation + 1;
        eprintln!(
            "[launcher] 扫描完成 程序={} UWP={} 文件={} 命令={} 用时={}ms → 第 {generation} 代",
            report.programs, report.uwp, report.files, report.commands, report.elapsed_ms
        );
        if let Err(error) = snapshot::save(&self.dir.join("index.json"), &candidates, created_at) {
            eprintln!("[launcher] 保存快照失败: {error}");
        }
        if let Ok(mut guard) = self.index.write() {
            *guard = Arc::new(Index {
                candidates,
                generation,
                created_at,
            });
        }
        if let Ok(mut cache) = self.cache.lock() {
            cache.0.clear();
            cache.1.clear();
        }
        // 全盘文件索引（写独立 SQLite 库；失败不影响内存候选可用）。
        if full_disk {
            match index_db::FileIndex::open(&self.file_index_path()) {
                Ok(mut idx) => {
                    let n = scan::scan_files_to_index(&mut idx, extra_excludes);
                    eprintln!("[launcher] 全盘索引就绪 {n} 条");
                }
                Err(error) => eprintln!("[launcher] 文件索引打开失败: {error}"),
            }
        }
        self.rebuilding.store(false, Ordering::Release);
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 从设置读出文件根目录配置（JSON 数组字符串）；为空或解析失败用默认四个用户目录。
pub fn roots_from_settings(app: &AppHandle) -> Vec<LauncherRoot> {
    let raw = app
        .state::<AppState>()
        .lock_store()
        .ok()
        .and_then(|store| store.get_settings().ok())
        .map(|settings| settings.launcher_roots().clone())
        .unwrap_or_default();
    scan::parse_roots(&raw)
}

/// 启动后台线程：立刻全量重扫一次（快照已在 new() 里加载，此刻已可搜），之后每 30 分钟重扫。
pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("launcher-indexer".into())
        .spawn(move || loop {
            let roots = roots_from_settings(&app);
            // 全盘开关与排除目录在 Task 7 接入设置；此处暂默认开启全盘、无额外排除。
            app.state::<LauncherState>().rebuild(&roots, true, &[]);
            std::thread::sleep(REBUILD_INTERVAL);
        })
        .expect("启动启动器索引线程失败");
}

/// 在后台线程触发一次重建（设置页「立即重建」）。
pub fn rebuild_async(app: AppHandle) {
    std::thread::spawn(move || {
        let roots = roots_from_settings(&app);
        app.state::<LauncherState>().rebuild(&roots, true, &[]);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_search_merges_file_index_hits() {
        let dir = std::env::temp_dir().join(format!("inkling-state-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let state = LauncherState::new(dir.clone());
        // 直接往索引库写一条文件，查询应命中并（无内存候选时）出现在结果里。
        {
            let mut idx = index_db::FileIndex::open(&dir.join("launcher-index.db")).unwrap();
            idx.begin_generation();
            idx.upsert_batch(&[index_db::FileRow {
                name: "预算表.xlsx".into(),
                path: "C:/x/预算表.xlsx".into(),
                kind: model::Kind::File,
                keywords: keyword::generate("预算表.xlsx"),
            }])
            .unwrap();
        }
        let hits = state.search("yusuan");
        assert!(hits.iter().any(|h| h.name.contains("预算表")));
        std::fs::remove_dir_all(&dir).ok();
    }
}
