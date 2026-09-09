//! 数据源扫描：传统程序（开始菜单 / 桌面）、UWP（Get-StartApps）、用户配置的文件根目录、内置命令。
//!
//! 与参考项目一致：程序源固定内置（`*.exe / *.lnk / *.url`，深度 5，排除 uninstall / 卸载 / help / 帮助）；
//! 文件源按 `LauncherRoot { path, depth, excludes }` 配置；总条数上限 200,000。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::keyword;
use super::model::{Candidate, Kind};
use super::{drives, exclude, index_db};

/// 索引条目上限：防止把整盘加进来拖垮内存（单条约 250B → 上限约 50MB）。
pub const MAX_CANDIDATES: usize = 200_000;
/// 程序名里含这些词的条目不入索引。
const PROGRAM_EXCLUDES: [&str; 4] = ["uninstall", "卸载", "help", "帮助"];
const PROGRAM_DEPTH: usize = 5;
/// UWP 枚举最长等待。
const UWP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// 用户配置的一个文件扫描根目录。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LauncherRoot {
    pub path: String,
    #[serde(default = "default_depth")]
    pub depth: u32,
    #[serde(default)]
    pub excludes: Vec<String>,
}

fn default_depth() -> u32 {
    4
}

/// 解析设置里的根目录 JSON；空或损坏时给默认值（用户主目录下四个常用文件夹）。
pub fn parse_roots(raw: &str) -> Vec<LauncherRoot> {
    if !raw.trim().is_empty() {
        match serde_json::from_str::<Vec<LauncherRoot>>(raw) {
            Ok(roots) => return roots,
            Err(error) => eprintln!("[launcher] 根目录配置损坏，使用默认: {error}"),
        }
    }
    default_roots()
}

/// 默认根目录：Documents / Downloads / Desktop / Pictures，深度 4。
pub fn default_roots() -> Vec<LauncherRoot> {
    let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) else {
        return Vec::new();
    };
    ["Documents", "Downloads", "Desktop", "Pictures"]
        .iter()
        .map(|name| LauncherRoot {
            path: home.join(name).to_string_lossy().to_string(),
            depth: 4,
            excludes: vec!["node_modules".into(), ".git".into()],
        })
        .collect()
}

/// 一次扫描的统计。
#[derive(Debug, Default)]
pub struct ScanReport {
    pub programs: usize,
    pub uwp: usize,
    pub files: usize,
    pub commands: usize,
    pub elapsed_ms: u128,
}

/// 扫描全部数据源，返回编号后的候选与统计。按路径去重。
pub fn scan_all(roots: &[LauncherRoot]) -> (Vec<Candidate>, ScanReport) {
    let started = Instant::now();
    let mut report = ScanReport::default();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<Candidate> = Vec::new();

    report.commands = builtin_commands(&mut out, &mut seen);
    report.programs = scan_programs(&mut out, &mut seen);
    report.uwp = scan_uwp(&mut out, &mut seen);
    report.files = scan_files(roots, &mut out, &mut seen);

    report.elapsed_ms = started.elapsed().as_millis();
    (out, report)
}

fn push(
    out: &mut Vec<Candidate>,
    seen: &mut HashSet<String>,
    kind: Kind,
    name: &str,
    path: &str,
) -> bool {
    if out.len() >= MAX_CANDIDATES {
        return false;
    }
    let key = path.to_lowercase();
    if !seen.insert(key) {
        return false;
    }
    let keywords = keyword::generate(name);
    if keywords.is_empty() {
        return false;
    }
    out.push(Candidate {
        id: out.len() as u32,
        kind,
        name: name.to_string(),
        path: path.to_string(),
        keywords,
        bias: 0.0,
    });
    true
}

/// 内置命令。
fn builtin_commands(out: &mut Vec<Candidate>, seen: &mut HashSet<String>) -> usize {
    let mut count = 0;
    for (id, name) in [
        ("cmd:settings", "Inkling 设置"),
        ("cmd:rebuild", "重建启动器索引"),
        ("cmd:quit", "退出 Inkling"),
    ] {
        if push(out, seen, Kind::Command, name, id) {
            count += 1;
        }
    }
    count
}

/// 程序源目录：公共开始菜单、用户开始菜单、用户桌面。
fn program_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(program_data) = std::env::var_os("ProgramData") {
        dirs.push(PathBuf::from(program_data).join("Microsoft\\Windows\\Start Menu"));
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(app_data).join("Microsoft\\Windows\\Start Menu"));
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        dirs.push(PathBuf::from(home).join("Desktop"));
    }
    dirs.into_iter().filter(|d| d.is_dir()).collect()
}

fn is_program_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("exe" | "lnk" | "url")
    )
}

fn name_excluded(name: &str, excludes: &[&str]) -> bool {
    let lower = name.to_lowercase();
    excludes.iter().any(|kw| lower.contains(&kw.to_lowercase()))
}

fn scan_programs(out: &mut Vec<Candidate>, seen: &mut HashSet<String>) -> usize {
    let mut count = 0;
    for dir in program_dirs() {
        for entry in WalkDir::new(&dir)
            .max_depth(PROGRAM_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_program_file(path) {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if name_excluded(stem, &PROGRAM_EXCLUDES) {
                continue;
            }
            if push(out, seen, Kind::App, stem, &path.to_string_lossy()) {
                count += 1;
            }
        }
    }
    count
}

/// UWP：`Get-StartApps` 一次拿到系统「所有应用」（已本地化）；失败或超时跳过。
fn scan_uwp(out: &mut Vec<Candidate>, seen: &mut HashSet<String>) -> usize {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (out, seen);
        0
    }
    #[cfg(target_os = "windows")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let output = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "Get-StartApps | Select-Object Name, AppID | ConvertTo-Json -Compress",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            let _ = tx.send(output);
        });
        let output = match rx.recv_timeout(UWP_TIMEOUT) {
            Ok(Ok(output)) => output,
            Ok(Err(error)) => {
                eprintln!("[launcher] 枚举 UWP 失败（PowerShell 不可用？）: {error}");
                return 0;
            }
            Err(_) => {
                eprintln!("[launcher] 枚举 UWP 超时 {}s，跳过", UWP_TIMEOUT.as_secs());
                return 0;
            }
        };
        let text = String::from_utf8_lossy(&output.stdout);
        parse_start_apps(&text)
            .into_iter()
            .filter(|(_, app_id)| {
                // 传统程序会以 .exe/.lnk 路径出现在 Get-StartApps 里，程序源已覆盖，这里只收 UWP 形态的 AppID
                !app_id.to_lowercase().ends_with(".exe") && !app_id.contains('\\')
            })
            .filter(|(name, _)| !name_excluded(name, &PROGRAM_EXCLUDES))
            .filter(|(name, app_id)| {
                push(
                    out,
                    seen,
                    Kind::Uwp,
                    name,
                    &format!("shell:AppsFolder\\{app_id}"),
                )
            })
            .count()
    }
}

/// 解析 `Get-StartApps | ConvertTo-Json` 输出（数组或单个对象）。
pub fn parse_start_apps(json: &str) -> Vec<(String, String)> {
    #[derive(Deserialize)]
    struct Row {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "AppID")]
        app_id: String,
    }
    let text = json.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let rows: Vec<Row> = match serde_json::from_str::<Vec<Row>>(text) {
        Ok(rows) => rows,
        Err(_) => match serde_json::from_str::<Row>(text) {
            Ok(row) => vec![row],
            Err(error) => {
                eprintln!("[launcher] 解析 Get-StartApps 输出失败: {error}");
                Vec::new()
            }
        },
    };
    rows.into_iter().map(|r| (r.name, r.app_id)).collect()
}

/// 文件根目录：文件与文件夹都入索引；名字含排除词的目录整棵跳过。
fn scan_files(
    roots: &[LauncherRoot],
    out: &mut Vec<Candidate>,
    seen: &mut HashSet<String>,
) -> usize {
    let mut count = 0;
    for root in roots {
        let base = Path::new(&root.path);
        if !base.is_dir() {
            eprintln!("[launcher] 根目录不存在，跳过: {}", root.path);
            continue;
        }
        let excludes: Vec<&str> = root.excludes.iter().map(|s| s.as_str()).collect();
        let walker = WalkDir::new(base)
            .max_depth(root.depth.max(1) as usize)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                e.depth() == 0
                    || e.file_name()
                        .to_str()
                        .map(|n| !n.starts_with('.') && !name_excluded(n, &excludes))
                        .unwrap_or(false)
            });
        for entry in walker.filter_map(|e| e.ok()) {
            if entry.depth() == 0 {
                continue;
            }
            let Some(name) = entry.file_name().to_str() else {
                continue;
            };
            let kind = if entry.file_type().is_dir() {
                Kind::Folder
            } else {
                Kind::File
            };
            if push(out, seen, kind, name, &entry.path().to_string_lossy()) {
                count += 1;
            }
            if out.len() >= MAX_CANDIDATES {
                eprintln!("[launcher] 达到索引上限 {MAX_CANDIDATES}，停止扫描");
                return count;
            }
        }
    }
    count
}

/// 扫描单个根目录树写入文件索引（供全盘入口与单测复用）。返回写入条数。
/// 目录名命中排除集则整棵跳过；文件不单独判断（由父目录过滤）。批量 2000 行一提交。
pub fn scan_dir_into_index(
    root: &Path,
    index: &mut index_db::FileIndex,
    extra: &[String],
) -> usize {
    let mut batch: Vec<index_db::FileRow> = Vec::with_capacity(2048);
    let mut total = 0usize;
    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
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
        let kind = if entry.file_type().is_dir() {
            Kind::Folder
        } else {
            Kind::File
        };
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

/// 全盘扫描所有固定磁盘写入索引（开新代 → 逐盘扫 → 清理消失项）。返回总条目数。
pub fn scan_files_to_index(index: &mut index_db::FileIndex, extra_excludes: &[String]) -> usize {
    index.begin_generation();
    let mut total = 0usize;
    for root in drives::fixed_drive_roots() {
        eprintln!("[launcher] 全盘索引开始扫描 {}", root.display());
        total += scan_dir_into_index(&root, index, extra_excludes);
    }
    let removed = index.prune_stale().unwrap_or(0);
    eprintln!("[launcher] 全盘索引完成 共 {total} 条，清理消失 {removed} 条");
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_dir_tree_into_index_respects_excludes() {
        use super::super::index_db::FileIndex;
        let base = std::env::temp_dir().join(format!("inkling-scanidx-{}", std::process::id()));
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
        assert!(idx
            .query("baogao", 50)
            .unwrap()
            .iter()
            .any(|r| r.name.contains("报告")));
        // node_modules 被排除
        assert!(idx.query("junk", 50).unwrap().is_empty());
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn parse_roots_defaults_and_json() {
        let roots =
            parse_roots(r#"[{"path":"C:/x","depth":2,"excludes":["tmp"]},{"path":"D:/y"}]"#);
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].depth, 2);
        assert_eq!(roots[1].depth, 4);
        assert!(roots[1].excludes.is_empty());
        // 损坏 / 空 → 默认（在有 USERPROFILE 的环境里非空）
        assert_eq!(parse_roots("not json"), default_roots());
        assert_eq!(parse_roots(""), default_roots());
    }

    #[test]
    fn parse_start_apps_handles_array_and_single_object() {
        let rows = parse_start_apps(
            r#"[{"Name":"计算器","AppID":"Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"}]"#,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "计算器");
        let single = parse_start_apps(r#"{"Name":"A","AppID":"B"}"#);
        assert_eq!(single, vec![("A".to_string(), "B".to_string())]);
        assert!(parse_start_apps("").is_empty());
    }

    #[test]
    fn scan_files_respects_depth_and_excludes() {
        let dir = std::env::temp_dir().join(format!("inkling-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("keep/deep/deeper")).unwrap();
        std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        std::fs::write(dir.join("keep/微信记录.txt"), "x").unwrap();
        std::fs::write(dir.join("keep/deep/deeper/too-deep.txt"), "x").unwrap();
        std::fs::write(dir.join("node_modules/pkg/index.js"), "x").unwrap();

        let roots = vec![LauncherRoot {
            path: dir.to_string_lossy().to_string(),
            depth: 2,
            excludes: vec!["node_modules".into()],
        }];
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let count = scan_files(&roots, &mut out, &mut seen);
        let names: Vec<&str> = out.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"keep"));
        assert!(names.contains(&"微信记录.txt"));
        assert!(!names.contains(&"too-deep.txt"), "{names:?}");
        assert!(!names.contains(&"index.js"), "{names:?}");
        assert!(!names.contains(&"node_modules"));
        assert_eq!(count, out.len());
        // 中文文件名生成了拼音关键字
        let wechat = out.iter().find(|c| c.name == "微信记录.txt").unwrap();
        assert!(wechat.keywords.iter().any(|k| k.starts_with("weixinjilu")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn program_filter_and_dedupe() {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        assert!(push(
            &mut out,
            &mut seen,
            Kind::App,
            "WeChat",
            "C:/A/WeChat.exe"
        ));
        assert!(
            !push(&mut out, &mut seen, Kind::App, "WeChat", "c:/a/wechat.exe"),
            "同路径大小写不同也应去重"
        );
        assert!(is_program_file(Path::new("x.LNK")));
        assert!(!is_program_file(Path::new("x.txt")));
        assert!(name_excluded("Uninstall WeChat", &PROGRAM_EXCLUDES));
    }
}
