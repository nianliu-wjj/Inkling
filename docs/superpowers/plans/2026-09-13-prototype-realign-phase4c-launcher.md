# 原型对齐 · 阶段四 C「浮窗启动台」实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把浮窗启动台对齐原型并补全检索能力：浮窗 DOM 改原型 `#launcherWindow.glass` 三段式（`.launcher-input-row` + `.launcher-list` + `.launcher-footer`）与两态页脚，统一结果源（应用 / 命令 / 笔记 / 待办 / 计算器 / 浏览器历史，受五个检索范围开关控制），内置命令扩到 9 条，浏览器历史采集（Chrome / Edge，复制副本后只读导入）与查询 / 计数命令，启动台页补七行设置（五开关 + 保留天数 + 已记录地址数），浮窗尺寸改 560×420，呼出时收起面板。

**Architecture:** 后端只加「能力」：六个 KV 设置键 + v8 迁移（**本次有真实 DDL**：建 `browser_history` 表与 `visited_at` 索引）、`services/browser_history.rs` 采集线程（30 秒首扫 / 之后每 10 分钟；复制到临时文件后 `OpenFlags::SQLITE_OPEN_READ_ONLY` 打开，WebKit 微秒时间戳换算后 `INSERT OR REPLACE`）、`browser_history_search` / `browser_history_count` 两个命令、`builtin_commands` 扩到 9 条并在 `launcher_launch` 按 id 分发（该命令由同步改 **async**，因为 `cmd:mindmap` 会建窗）。前端一个纯函数 `mergeLauncherResults()` 决定「结果集与顺序」，一个组合式 `useLauncherResults()` 负责取数 / 去抖 / 动作分派，浮窗与启动台页共用；浮窗整体重写为原型 DOM，删除 `src/styles/launcher.css`，自有样式只补 `window-fit.css`（窗口适配）与 `extensions.css` §11（页脚两栏）。

**Tech Stack:** Tauri 2 / Rust（rusqlite 0.37 `bundled`，`OpenFlags::SQLITE_OPEN_READ_ONLY`；`std::thread` 采集线程）· Vue 3.5 `<script setup lang="ts">` · TypeScript 5.9 · Vite 7 · animejs 4.5（`enter` 预设）· Node 24（`node:test`）

**设计文档：** `docs/superpowers/specs/2026-09-13-prototype-realign-phase4c-launcher-design.md`（下文简称 spec；决策 §2 D35–D40、差异表 §3 #1–#15、设计 §4.1–§4.7、测试 §5、验收 §6、提交批次 §7、风险 §8）

## Global Constraints

- 唯一参考原型 `docs/index.html`（`#launcherWindow` 438–447、启动台页 185–208）+ `docs/styles.css` + `docs/app.js`（`LAUNCHER_APPS` / `collectLauncherResults` / `renderLauncherResults` / `openLauncher` 3174–3347、键盘与页脚 3316–3355、设置桥接与 `pruneBrowserHistory` 3360–3434）；生成层样式（`tokens.css` / `base.css` / `components.css` / `themes.css`）**不改**，`pnpm sync:styles --check` 必须始终通过。项目自有样式只进 `src/styles/extensions.css`（本阶段：**追加 §11 浮窗启动台页脚**）与 `src/styles/window-fit.css`（**追加启动台窗口块**）；`src/styles/launcher.css` 整文件删除。
- 组件模板只用原型类名（`#launcherWindow` / `.glass` / `.launcher-input-row` / `.launcher-ico` / `#launcherInput` / `.launcher-list` / `.launcher-item(.active)` / `.li-ico` / `.li-name` / `.li-cat` / `.launcher-empty` / `.launcher-footer` / `.launcher-page-hero` / `.setting-row` / `.setting-col` / `.clip-editor-hint`）；`.launcher-keys` 是本阶段唯一新增类名（D40 的动作提示位，规则在 `extensions.css` §11）；颜色只用令牌。
- 已拍定决策（spec §2）：D35 只扫 Chrome / Edge 的 Chromium `History`（`User Data/{Default,Profile *}`），**复制到临时文件后只读打开**；D36 内置命令 9 条；D37 浮窗 560×420；D38 不内置表达式求值，`=` 开头或「计算器 / calc」→ 执行 `calc.exe`；D39 呼出时只 `panel_hide`，**不隐藏主窗口**；D40 动作栏 / `Alt+N` / `Ctrl+Enter` 并入 `.launcher-footer` 右侧。
- 编码规范：Vue 一律 `<script setup lang="ts">`，禁止 `any`；关键节点走 `src/service/logger.ts`（Rust 侧 `eprintln!`，前缀 `[launcher]` / `[history]` / `[data]` / `[settings]`）；每个文件头部有职责说明注释；Rust 新增 `pub` 项一律带 `///` 文档注释。
- **所有可能建窗或做 show / hide 序列的 Tauri 命令一律 `pub async fn`**：本阶段 `ipc::launcher_launch` 由**同步改 async**（`cmd:mindmap` 走 `windows::mindmap_open` 建窗，任务书误记为「已是 async」，代码实际是 `pub fn`——见自检记录 1）；`ipc::launcher_show` 已是 async；新增 `browser_history_search` / `browser_history_count` 只读库、不碰窗口，保持同步。
- rusqlite 只读打开外部库必须带 `OpenFlags::SQLITE_OPEN_READ_ONLY`（`Connection::open_with_flags`）；仓库此前没有 `OpenFlags` 用法，本阶段首次引入。
- 动效硬约束沿用阶段一 / 四 B：只动 transform / opacity；`.glass` 静止态无 transform（入场由 `src/motion` 的 `enter()` 驱动，瞬时内联 style，播完自动清）。
- 验证：每任务结束 `pnpm typecheck` 零错误；改 Rust 的任务 `cargo fmt --check && cargo check && cargo test`（在 `src-tauri/` 下）；改纯函数（Task 4）`pnpm test:unit`；改样式（Task 5）`pnpm sync:styles --check`；`pnpm format:check` 每任务；最后 `pnpm exec vite build`。
- 提交：中文提交信息，按 spec §7 的 7 个批次，每任务一次提交（Task 7 为验收记录提交）。

---

## File Structure

```
src-tauri/
├─ src/
│  ├─ domain/models.rs                Settings 六个新键（五开关 + 保留天数）；新增 BrowserHistoryRow DTO
│  ├─ data/settings.rs                读写六键 + 往返单测
│  ├─ data/mod.rs                     migrate 链加 v8（建 browser_history 表与 visited_at 索引）+ 单测
│  ├─ services/mod.rs                 注册 browser_history 模块
│  ├─ services/browser_history.rs     新：WebKit 换算 / 候选路径 / 复制只读导入 / search / count / prune / 线程
│  ├─ services/launcher/scan.rs       builtin_commands 3 → 9 条 + 单测
│  ├─ services/launcher/launch.rs     open_system_calculator()
│  ├─ ipc.rs                          两个浏览器历史命令；settings_save 补 prune；launcher_launch 改 async 并按 id 分发
│  ├─ app/windows.rs                  LAUNCHER_WIDTH 640→560、LAUNCHER_HEIGHT 464→420；launcher_show 先 panel_hide
│  └─ main.rs                         注册两个新命令 + 启动 browser_history 线程
src/
├─ typings/domain.ts                  Settings 六键
├─ composables/
│  ├─ useData.ts                      DEFAULT_SETTINGS 六键
│  ├─ useLauncherResults.ts           新：共享结果源（取数 / 去抖 / 动作分派 / 浮窗键位）
│  └─ useLauncherSearch.ts            删除（Task 6：最后一个调用方 LauncherPageView 改掉后整文件删）
├─ utils/
│  ├─ launcherResults.ts              新：mergeLauncherResults 纯函数 + 类型
│  └─ launcherResults.test.ts         新：六个用例
├─ service/tauri.ts                   BrowserHistoryRow 类型 + api.browserHistory.{search,count}
├─ windows/
│  ├─ launcher.ts                     删 launcher.css 引入
│  ├─ Launcher/LauncherApp.vue        整体重写（原型 DOM / 两态页脚 / 键盘 / 入场 / 清空聚焦）
│  └─ Main/LauncherPageView.vue       换结果源（useLauncherResults）+ 补七行设置
└─ styles/
   ├─ launcher.css                    删除
   ├─ window-fit.css                  启动台窗口铺满 + 结果列表撑满
   └─ extensions.css                  §11 浮窗启动台页脚两栏
docs/superpowers/plans/2026-09-13-prototype-realign-phase4c-acceptance.md   验收记录（Task 7）
```

---

## Task 1: 启动台六个新设置项与 v8 迁移（建 `browser_history` 表）

**Files:**
- Modify: `src-tauri/src/domain/models.rs:228-231,234-271`（`launcher_extra_excludes` 之后 + `Default` 实现）
- Modify: `src-tauri/src/data/mod.rs:85,195-202,327-334,834-835`
- Modify: `src-tauri/src/data/settings.rs:141-147,205-212,276-281`
- Modify: `src/typings/domain.ts:149-154`
- Modify: `src/composables/useData.ts:48-52`

**Interfaces:**
- Produces（Rust）：`Settings` 新增 `launcher_scope_apps / launcher_scope_notes / launcher_scope_todos / launcher_scope_calc / launcher_scope_history: bool`（默认全 `true`）、`launcher_history_retention_days: i64`（默认 100）；`dto!` 自动生成 `launcher_scope_apps()` / `set_launcher_history_retention_days()` 等访问器；新默认函数 `default_launcher_history_retention_days() -> i64`；`Store::with_v8() -> Result<(), String>`。
- Produces（TS）：`Settings` 增同名六键（`launcher_scope_*: boolean`、`launcher_history_retention_days: number`）。

- [ ] **Step 1: 先写失败的 v8 迁移单测**

`src-tauri/src/data/mod.rs` 的 `mod tests` 末尾（`v7_migration_bumps_version_and_is_idempotent` 的收尾 `}` 之后、模块收尾 `}` 之前，即第 834 行与第 835 行之间）追加：

```rust
    /// v8 推进版本号并建 browser_history 表（spec 4C §4.3，本次有真实 DDL）：6 项断言。
    #[test]
    fn v8_migration_creates_browser_history_and_is_idempotent() {
        let db = settings_db(None);
        db.pragma_update(None, "user_version", 7).unwrap();
        let mut store = Store {
            db,
            data_dir: std::path::PathBuf::from("."),
        };
        store.migrate().unwrap();
        let version: i64 = store
            .db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 8);

        // 表与索引都建出来了（索引名由 sqlite_master 查，避免依赖 PRAGMA 顺序）。
        let tables: i64 = store
            .db
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='browser_history'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 1);
        let indexes: i64 = store
            .db
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_browser_history_visited'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(indexes, 1);

        // 幂等：再跑一次不报错，版本仍是 8，表没有被重建（主键仍是 url）。
        store.migrate().unwrap();
        let again: i64 = store
            .db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(again, 8);
        let url_pk: i64 = store
            .db
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('browser_history') WHERE name='url' AND pk=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(url_pk, 1);

        // 设置键仍靠默认值兜底：迁移不凭空插入设置行。
        let rows: i64 = store
            .db
            .query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }
```

- [ ] **Step 2: 运行，确认失败**

Run: `cd src-tauri && cargo test v8_migration 2>&1 | grep -E "assert|panicked|test result" | head`
Expected: `assertion `left == right` failed`（left 7 right 8）——`migrate()` 尚无 v8 分支。

- [ ] **Step 3: `data/mod.rs` 加 v8 链与 `with_v8`**

第 85 行注释末尾追加一段：

```rust
    /// 版本化迁移：v0（初版）→ v1（archived_at / 附件路径 / 统计事件 / 提醒实例 / 提醒抑制标记）→ v6（clipboard_entries.source_app）→ v7（灵动岛四个新设置键，仅推进版本号）→ v8（browser_history 表与 visited_at 索引；六个启动台设置键仅推进版本号）。
```

第 195–201 行 `if version < 7 {…}` 之后、`Ok(())`（第 202 行）之前插入：

```rust
        if version < 8 {
            self.with_v8()
                .map_err(|e| format!("数据库迁移到 v8 失败: {e}"))?;
            self.db
                .pragma_update(None, "user_version", 8)
                .map_err(db_err)?;
        }
```

第 331–334 行 `with_v7` 之后（`add_column_if_missing` 之前）插入：

```rust
    /// v8 增量：浏览器历史表（spec 4C D35 / §4.3）。
    ///
    /// 与 v5 / v7 不同，这里**有真实 DDL**：`browser_history` 由
    /// `services/browser_history.rs` 从 Chrome / Edge 的 Chromium `History` 库导入后查询。
    /// 六个新设置键（五个检索范围开关 + 历史保留天数）仍靠 `Settings::default()` 兜底，不写行。
    fn with_v8(&self) -> Result<(), String> {
        self.db
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS browser_history (
                   url        TEXT PRIMARY KEY,
                   title      TEXT NOT NULL DEFAULT '',
                   visited_at INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_browser_history_visited ON browser_history(visited_at DESC);",
            )
            .map_err(db_err)?;
        eprintln!("[data] v8 迁移：新建 browser_history 表与 idx_browser_history_visited 索引");
        Ok(())
    }
```

Run: `cd src-tauri && cargo test v8_migration 2>&1 | grep "test result"`
Expected: `test result: ok. 1 passed`。

- [ ] **Step 4: `domain/models.rs` 六字段与新默认**

第 131–134 行 `default_launcher_shortcut` 之后插入：

```rust
/// 浏览器历史默认保留天数；原型 prefs.lpHistoryRetention = 100（spec 4C §4.3）。
fn default_launcher_history_retention_days() -> i64 {
    100
}
```

`Settings` 结构体第 230 行 `launcher_extra_excludes: String,` 之后插入：

```rust
        /// 启动台检索范围：应用与命令（原型 prefs.lpApps，默认开）。
        #[serde(default = "default_true")]
        launcher_scope_apps: bool,
        /// 启动台检索范围：笔记（原型 prefs.lpNotes，默认开）。
        #[serde(default = "default_true")]
        launcher_scope_notes: bool,
        /// 启动台检索范围：待办（原型 prefs.lpTodos，默认开）。
        #[serde(default = "default_true")]
        launcher_scope_todos: bool,
        /// 启动台计算器：`=` 开头或匹配「计算器 / calc」时提示打开系统计算器（原型 prefs.lpCalc，默认开）。
        #[serde(default = "default_true")]
        launcher_scope_calc: bool,
        /// 启动台检索范围：浏览器历史（原型 prefs.lpHistory，默认开）。
        #[serde(default = "default_true")]
        launcher_scope_history: bool,
        /// 浏览器历史保留天数（1–365；原型 1–365、默认 100）。
        #[serde(default = "default_launcher_history_retention_days")]
        launcher_history_retention_days: i64,
```

`impl Default for Settings` 第 268 行 `launcher_extra_excludes: String::new(),` 之后插入：

```rust
            launcher_scope_apps: true,
            launcher_scope_notes: true,
            launcher_scope_todos: true,
            launcher_scope_calc: true,
            launcher_scope_history: true,
            launcher_history_retention_days: default_launcher_history_retention_days(),
```

- [ ] **Step 5: `data/settings.rs` 读写六键**

第 141–146 行 `.launcher_extra_excludes(…)` 之后、第 147 行 `.build()` 之前插入：

```rust
            .launcher_scope_apps(flag(
                &values,
                "launcher_scope_apps",
                *defaults.launcher_scope_apps(),
            ))
            .launcher_scope_notes(flag(
                &values,
                "launcher_scope_notes",
                *defaults.launcher_scope_notes(),
            ))
            .launcher_scope_todos(flag(
                &values,
                "launcher_scope_todos",
                *defaults.launcher_scope_todos(),
            ))
            .launcher_scope_calc(flag(
                &values,
                "launcher_scope_calc",
                *defaults.launcher_scope_calc(),
            ))
            .launcher_scope_history(flag(
                &values,
                "launcher_scope_history",
                *defaults.launcher_scope_history(),
            ))
            .launcher_history_retention_days(
                values
                    .get("launcher_history_retention_days")
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(*defaults.launcher_history_retention_days()),
            )
```

`save_settings` 第 209–212 行 `("launcher_extra_excludes", …)` 之后插入：

```rust
            (
                "launcher_scope_apps",
                settings.launcher_scope_apps().to_string(),
            ),
            (
                "launcher_scope_notes",
                settings.launcher_scope_notes().to_string(),
            ),
            (
                "launcher_scope_todos",
                settings.launcher_scope_todos().to_string(),
            ),
            (
                "launcher_scope_calc",
                settings.launcher_scope_calc().to_string(),
            ),
            (
                "launcher_scope_history",
                settings.launcher_scope_history().to_string(),
            ),
            (
                "launcher_history_retention_days",
                settings.launcher_history_retention_days().to_string(),
            ),
```

`mod tests` 末尾（第 276–281 行 `empty_password_is_not_masked` 之后、模块收尾 `}` 之前）追加：

```rust
    /// 六个启动台新键：默认值 + 读写往返（复用上面的临时目录 store 样板）。
    #[test]
    fn launcher_scope_defaults_and_history_retention_round_trip() {
        let store = store();
        let defaults = store.get_settings().unwrap();
        assert!(defaults.launcher_scope_apps());
        assert!(defaults.launcher_scope_notes());
        assert!(defaults.launcher_scope_todos());
        assert!(defaults.launcher_scope_calc());
        assert!(defaults.launcher_scope_history());
        assert_eq!(*defaults.launcher_history_retention_days(), 100);

        let mut settings = defaults;
        settings.set_launcher_scope_notes(false);
        settings.set_launcher_scope_history(false);
        settings.set_launcher_history_retention_days(7);
        store.save_settings(&settings).unwrap();

        let read = store.get_settings().unwrap();
        assert!(!read.launcher_scope_notes());
        assert!(!read.launcher_scope_history());
        assert!(read.launcher_scope_apps(), "未改的开关应保持开启");
        assert_eq!(*read.launcher_history_retention_days(), 7);
    }
```

- [ ] **Step 6: 前端类型与默认值**

`src/typings/domain.ts` 第 153 行 `launcher_extra_excludes: string` 之后插入：

```ts
  /** 启动台检索范围：应用与命令。 */
  launcher_scope_apps: boolean
  /** 启动台检索范围：笔记。 */
  launcher_scope_notes: boolean
  /** 启动台检索范围：待办。 */
  launcher_scope_todos: boolean
  /** 启动台计算器（`=` 开头或「计算器 / calc」→ 打开系统计算器）。 */
  launcher_scope_calc: boolean
  /** 启动台检索范围：浏览器历史。 */
  launcher_scope_history: boolean
  /** 浏览器历史保留天数（1–365，默认 100）。 */
  launcher_history_retention_days: number
```

`src/composables/useData.ts` 第 51 行 `launcher_extra_excludes: '',` 之后插入：

```ts
  launcher_scope_apps: true,
  launcher_scope_notes: true,
  launcher_scope_todos: true,
  launcher_scope_calc: true,
  launcher_scope_history: true,
  launcher_history_retention_days: 100,
```

- [ ] **Step 7: 校验并提交**

```bash
cd src-tauri && cargo fmt --check && cargo check && cargo test 2>&1 | grep -E "test result|FAILED"
cd .. && pnpm typecheck && pnpm format:check
git add src-tauri/src/domain/models.rs src-tauri/src/data/settings.rs src-tauri/src/data/mod.rs src/typings/domain.ts src/composables/useData.ts
git commit -m "feat(settings): 启动台五个检索范围开关与历史保留天数（v8 迁移含 browser_history 建表）"
```

---

## Task 2: 浏览器历史采集服务（Chrome / Edge，复制后只读导入）与查询 / 计数命令

**Files:**
- Create: `src-tauri/src/services/browser_history.rs`
- Modify: `src-tauri/src/domain/models.rs:310-324`（`DayDetailItem` 之前或文件末尾加 `BrowserHistoryRow`）
- Modify: `src-tauri/src/services/mod.rs:1-8`
- Modify: `src-tauri/src/ipc.rs:611-632,753-756`
- Modify: `src-tauri/src/main.rs:57-61,148-154`
- Modify: `src/service/tauri.ts:27-33,84-97`

**Interfaces:**
- Consumes：Task 1 `Settings.launcher_history_retention_days`、`launcher_history_retention_days` 键、v8 建好的 `browser_history` 表。
- Produces（Rust）：
  - `domain::models::BrowserHistoryRow { url: String, title: String, visited_at: i64 }`（`dto!`，`visited_at` 为 Unix 秒）。
  - `services::browser_history::webkit_to_unix(webkit_micros: i64) -> i64`。
  - `services::browser_history::candidate_paths() -> Vec<PathBuf>`（`%LOCALAPPDATA%\Google\Chrome\User Data\{Default,Profile 1..8}\History` 与 `Microsoft\Edge` 同构；不存在的跳过）。
  - `services::browser_history::import_file(store: &Store, path: &Path, since: i64) -> Result<usize, String>`。
  - `services::browser_history::run_once(app: &AppHandle) -> Result<usize, String>`、`prune_now(app: &AppHandle) -> Result<usize, String>`、`start(app: AppHandle)`。
  - `services::browser_history::search(store: &Store, query: &str, limit: usize) -> Result<Vec<BrowserHistoryRow>, String>`、`count(store: &Store) -> Result<i64, String>`、`prune(store: &Store, now: i64, retention_days: i64) -> Result<usize, String>`。
  - `ipc::browser_history_search(state, query: String, limit: usize)`、`ipc::browser_history_count(state) -> i64`。
- Produces（TS）：`BrowserHistoryRow { url: string; title: string; visited_at: number }`；`api.browserHistory.search(query)`、`api.browserHistory.count()`。

- [ ] **Step 1: 新建服务文件，先只写时间换算与其单测**

`src-tauri/src/services/browser_history.rs`：

```rust
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

use std::time::Duration;

/// 启动后延迟首扫（spec §4.4）：避开开机高峰，也给浏览器留出写完 History 的时间。
const FIRST_SCAN_DELAY: Duration = Duration::from_secs(30);
/// WebKit / Chromium 时间戳零点（1601-01-01）与 Unix 纪元（1970-01-01）的秒差。
const WEBKIT_EPOCH_OFFSET_SECS: i64 = 11_644_473_600;

/// Chromium 时间戳（1601-01-01 起的**微秒**）→ Unix 秒（spec §4.4）。
pub fn webkit_to_unix(webkit_micros: i64) -> i64 {
    webkit_micros / 1_000_000 - WEBKIT_EPOCH_OFFSET_SECS
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
}
```

Run: `cd src-tauri && cargo test webkit_timestamp 2>&1 | grep "test result"`
Expected: `test result: ok. 1 passed`。

> 本步**只写** `use std::time::Duration;` 这一条导入与两个常量。其余导入（`PathBuf` / `OpenFlags` / `AppState` / `Store` / `BrowserHistoryRow`）与常量在本步都用不到，先写会触发 `unused_imports` 告警；它们随 Step 3 / Step 4 一起补。

- [ ] **Step 2: 运行，确认失败（TDD 红灯）**

先临时把断言写错一次（把 `assert_eq!(webkit_to_unix(11_644_473_600_000_000), 0);` 改成 `1`），Run: `cd src-tauri && cargo test webkit_timestamp 2>&1 | grep -E "assert|test result"`
Expected: `assertion `left == right` failed`（left 1 right 0）——确认测试真的在跑这条断言，然后把断言改回 `0`。

- [ ] **Step 3: 补 `BrowserHistoryRow` DTO（先做，Step 4 的代码要引用它）**

`src-tauri/src/domain/models.rs` 文件末尾（`DayDetailItem` 的 `dto!` 块之后，第 324 行 `}` 之后）追加：

```rust
crate::dto! {
    /// 一条浏览器历史（Chrome / Edge 导入，spec 4C §4.4）。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct BrowserHistoryRow {
        url: String,
        /// 页面标题；浏览器未记录时为空串（前端回退显示 URL）。
        title: String,
        /// 访问时刻（Unix 秒）。
        visited_at: i64,
    }
}
```

- [ ] **Step 4: 补齐导入 / 常量 / 导入查询清理线程**

（a）文件头第 1 条 `use` 段（当前只有 `use std::time::Duration;`）整体替换为：

```rust
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use tauri::{AppHandle, Manager};

use crate::app::state::AppState;
use crate::data::{db_err, Store};
use crate::domain::models::BrowserHistoryRow;
```

（b）常量段（`const WEBKIT_EPOCH_OFFSET_SECS: i64 = 11_644_473_600;` 这一行之后）追加：

```rust
/// 之后每 10 分钟导入一次（spec §4.4）。
const SCAN_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// 单次导入的查询上限（保留窗口内通常远小于它；兜底防止异常库把内存吃满）。
const MAX_IMPORT_ROWS: usize = 20_000;
/// 每个浏览器的 Profile 目录探测上限（`Profile 1` … `Profile 8`）。
const MAX_PROFILES: u32 = 8;
/// 临时副本序号：同进程内两次导入不撞名。
static COPY_SEQ: AtomicU64 = AtomicU64::new(0);
```

（c）`webkit_to_unix` 函数之后（`#[cfg(test)] mod tests` 之前）插入：

```rust
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
    std::fs::copy(path, &copy).map_err(|e| format!("复制历史库失败: {e}"))?;
    let conn = Connection::open_with_flags(&copy, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("只读打开历史副本失败: {e}"))?;
    Ok((conn, copy))
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
        .execute(
            "DELETE FROM browser_history WHERE visited_at < ?",
            [cutoff],
        )
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
    let store = app.state::<AppState>().lock_store()?;
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
    let store = app.state::<AppState>().lock_store()?;
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
```

- [ ] **Step 5: 补 `search` / `prune` / `import_file` 的内存库单测**

在同文件 `mod tests` 里追加（`import_file` 的用例还需 `use crate::data::Store;`——已在文件头导入）：

```rust
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
            by_title.iter().map(|r| r.url().as_str()).collect::<Vec<_>>(),
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
```

Run: `cd src-tauri && cargo test browser_history 2>&1 | grep -E "test result|FAILED"`
Expected: `test result: ok. 4 passed`。

- [ ] **Step 6: `services/mod.rs` 注册模块**

第 3 行 `pub mod clipboard_watcher;` 之前插入：

```rust
pub mod browser_history;
```

- [ ] **Step 7: `ipc.rs` 两个命令 + `settings_save` 补 prune**

`launcher_status` 之后（第 756 行 `}` 之后、`launcher_hide` 之前）插入：

```rust
/// 按标题 / URL 模糊查询浏览器历史，按访问时刻倒序。
#[tauri::command]
pub fn browser_history_search(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<crate::domain::models::BrowserHistoryRow>, String> {
    crate::services::browser_history::search(&state.lock_store()?, &query, limit)
}

/// 已记录的历史地址条数（设置页展示）。
#[tauri::command]
pub fn browser_history_count(state: State<'_, AppState>) -> Result<i64, String> {
    crate::services::browser_history::count(&state.lock_store()?)
}
```

`settings_save` 里 `state.set_island_flags(...)` 之后（第 616 行 `});` 之后、`if *settings.start_on_boot()` 之前）插入：

```rust
    // 历史保留天数改小后立即生效：不等下一轮 10 分钟导入，否则设置页的「已记录历史地址」要等很久才变。
    if let Err(error) = crate::services::browser_history::prune_now(&app) {
        eprintln!("[settings] 清理过期浏览器历史失败: {error}");
    }
```

- [ ] **Step 8: `main.rs` 注册命令与启动线程**

命令注册：第 154 行 `ipc::rebind_launcher_shortcut` 之后追加（前一行末加逗号）：

```rust
            ipc::rebind_launcher_shortcut,
            ipc::browser_history_search,
            ipc::browser_history_count
```

服务启动：第 61 行 `services::launcher::start(app.clone());` 之后插入：

```rust
            services::browser_history::start(app.clone());
```

- [ ] **Step 9: `tauri.ts` 类型与 `api.browserHistory`**

`LauncherStatus` 接口之后（第 33 行 `}` 之后）插入：

```ts
/** 浏览器历史条目（对应 Rust domain::models::BrowserHistoryRow）。 */
export interface BrowserHistoryRow {
  url: string
  /** 页面标题；浏览器未记录时为空串。 */
  title: string
  /** 访问时刻（Unix 秒）。 */
  visited_at: number
}
```

`api.launcher` 之后（第 103 行 `},` 之后）插入：

```ts
  /** 浏览器历史（Chrome / Edge 定时导入，spec 4C D35）。 */
  browserHistory: {
    /** 按标题 / URL 模糊查询，按访问时刻倒序。 */
    search: (query: string, limit = 20) => invoke<BrowserHistoryRow[]>('browser_history_search', { query, limit }),
    /** 已记录的历史地址条数（设置页展示）。 */
    count: () => invoke<number>('browser_history_count'),
  },
```

- [ ] **Step 10: 校验并提交**

```bash
cd src-tauri && cargo fmt --check && cargo check && cargo test 2>&1 | grep -E "test result|FAILED"
cd .. && pnpm typecheck && pnpm format:check
git add src-tauri/src/services/browser_history.rs src-tauri/src/domain/models.rs src-tauri/src/services/mod.rs src-tauri/src/ipc.rs src-tauri/src/main.rs src/service/tauri.ts
git commit -m "feat(launcher): 浏览器历史采集服务（Chrome / Edge，复制后只读导入）与查询 / 计数命令"
```

---

## Task 3: 内置命令扩到九条并按 id 分发（含系统计算器）

**Files:**
- Modify: `src-tauri/src/services/launcher/scan.rs:120-133,388-494`
- Modify: `src-tauri/src/services/launcher/launch.rs:60-99`
- Modify: `src-tauri/src/ipc.rs:691-742`

**Interfaces:**
- Produces（Rust）：`scan::builtin_commands` 输出 9 条 `cmd:*`（名称自带 emoji）；`launch::open_system_calculator() -> Result<(), String>`；`ipc::launcher_launch` 改 `pub async fn`（签名其余不变）。
- Consumes：Task 4 前端按 `kind === 'command'` 剥名称里的 emoji（`LauncherResult` 的 `ico`）。

- [ ] **Step 1: 先写失败的内置命令单测**

`src-tauri/src/services/launcher/scan.rs` 的 `mod tests` 末尾（`program_filter_and_dedupe` 之后、模块收尾 `}` 之前）追加：

```rust
    /// 内置命令 9 条、顺序与 id 固定（spec 4C D36 / §4.5），且名称里的 emoji 不阻碍拼音关键字生成。
    #[test]
    fn builtin_commands_cover_nine_ids() {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        assert_eq!(builtin_commands(&mut out, &mut seen), 9);
        let ids: Vec<&str> = out.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "cmd:notes",
                "cmd:todos",
                "cmd:clips",
                "cmd:mindmap",
                "cmd:stats",
                "cmd:settings",
                "cmd:calc",
                "cmd:rebuild",
                "cmd:quit",
            ]
        );
        assert!(out.iter().all(|c| c.kind == Kind::Command));
        // 名称自带 emoji，关键字仍要能生成（否则拼音 / 首字母检索会漏掉命令）。
        assert!(
            out.iter().all(|c| !c.keywords.is_empty()),
            "{:?}",
            out.iter().map(|c| (&c.name, &c.keywords)).collect::<Vec<_>>()
        );
        // 同一批候选再次扫描时按 path 去重，不重复入表。
        assert_eq!(builtin_commands(&mut out, &mut seen), 0);
    }
```

Run: `cd src-tauri && cargo test builtin_commands 2>&1 | grep -E "assert|left|test result" | head`
Expected: `assertion `left == right` failed`（left 3 right 9）。

- [ ] **Step 2: `scan.rs` 改 `builtin_commands`**

第 120–133 行整体替换为：

```rust
/// 内置命令（spec 4C D36 共 9 条）：原型 `LAUNCHER_APPS` 的 6 条 Inkling 功能入口
/// ＋现有运维命令（重建索引 / 退出）＋计算器（D38，执行 `calc.exe`）。
///
/// 名称自带 emoji：前端对 `kind === 'command'` 直接显示名称（`cmd:spec` 这类 path 不作为副文案）。
fn builtin_commands(out: &mut Vec<Candidate>, seen: &mut HashSet<String>) -> usize {
    let mut count = 0;
    for (id, name) in [
        ("cmd:notes", "📚 历史归档"),
        ("cmd:todos", "✅ 待办清单"),
        ("cmd:clips", "📋 剪贴板历史"),
        ("cmd:mindmap", "🧠 新建思维导图"),
        ("cmd:stats", "📊 统计报表"),
        ("cmd:settings", "⚙️ 偏好设置"),
        ("cmd:calc", "🧮 计算器"),
        ("cmd:rebuild", "🔄 重建启动器索引"),
        ("cmd:quit", "⏻ 退出 Inkling"),
    ] {
        if push(out, seen, Kind::Command, name, id) {
            count += 1;
        }
    }
    count
}
```

Run: `cd src-tauri && cargo test builtin_commands 2>&1 | grep "test result"`
Expected: `test result: ok. 1 passed`。

- [ ] **Step 3: `launch.rs` 加系统计算器**

第 60 行 `launch` 函数收尾 `}` 之后、第 62 行 `/// 分离启动：…` 之前插入：

```rust
/// 打开系统计算器（D38：不内置表达式求值，直接把 `=` / 「计算器」的查询交给 `calc.exe`）。
pub fn open_system_calculator() -> Result<(), String> {
    eprintln!("[launcher] 执行内置命令 cmd:calc → calc.exe");
    spawn_detached("calc.exe", &[])
}
```

- [ ] **Step 4: `ipc.rs` `launcher_launch` 改 async 并按 id 分发**

第 691–742 行整体替换为：

```rust
/// 启动一个命中项。`mode`：open / admin / reveal。文件命中按 path 启动，故直接收 path/kind
/// （命中项自带，无需按 id 回查——文件索引命中不在内存候选表里）。`query` 用于记录查询亲和度。
///
/// 必须是 **async** 命令：`cmd:mindmap` 会走 `windows::mindmap_open` 建窗，而建窗需要主线程的
/// 事件循环；同步命令跑在主线程上会与事件循环互等（与 `editor_open` / `pin_create` 同款）。
#[tauri::command]
pub async fn launcher_launch(
    app: AppHandle,
    state: State<'_, crate::services::launcher::LauncherState>,
    path: String,
    kind: String,
    mode: String,
    query: String,
) -> Result<(), String> {
    use crate::services::launcher::launch::{open_system_calculator, launch, LaunchMode};
    use crate::services::launcher::model::{Candidate, Kind};

    let parsed_kind = match kind.as_str() {
        "app" => Kind::App,
        "uwp" => Kind::Uwp,
        "file" => Kind::File,
        "folder" => Kind::Folder,
        "command" => Kind::Command,
        other => return Err(format!("未知类型 {other}")),
    };

    // 内置命令直接在此执行（spec 4C §4.5 的九条）。
    if parsed_kind == Kind::Command {
        match path.as_str() {
            // 主窗口各页（原型 LAUNCHER_APPS 的 6 条 Inkling 功能）。
            "cmd:notes" => windows::show_main(&app, "notes")?,
            "cmd:todos" => windows::show_main(&app, "todos")?,
            "cmd:clips" => windows::show_main(&app, "clips")?,
            "cmd:stats" => windows::show_main(&app, "stats")?,
            "cmd:settings" => windows::show_main(&app, "settings")?,
            // 新建思维导图：会建窗，本命令已是 async，直接调同步实现（同 mindmap_open 命令的做法）。
            "cmd:mindmap" => windows::mindmap_open(&app, None)?,
            // 计算器：拉起系统 calc.exe（D38 不内置求值）。
            "cmd:calc" => open_system_calculator()?,
            "cmd:rebuild" => crate::services::launcher::rebuild_async(app.clone()),
            "cmd:quit" => windows::quit_app(&app),
            other => return Err(format!("未知命令 {other}")),
        }
        let _ = windows::launcher_hide(&app);
        return Ok(());
    }

    let launch_mode = LaunchMode::parse(&mode).ok_or("未知启动方式")?;
    let candidate = Candidate {
        id: 0,
        kind: parsed_kind,
        name: String::new(),
        path: path.clone(),
        keywords: Vec::new(),
        bias: 0.0,
    };
    launch(&candidate, launch_mode)?;
    state.record_launch(&path, &query);
    // 普通启动后收起启动器；打开所在文件夹保留窗口方便继续操作。
    if launch_mode != LaunchMode::Reveal {
        let _ = windows::launcher_hide(&app);
    }
    Ok(())
}
```

- [ ] **Step 5: 校验并提交**

```bash
cd src-tauri && cargo fmt --check && cargo check && cargo test 2>&1 | grep -E "test result|FAILED"
cd .. && pnpm typecheck && pnpm format:check
git add src-tauri/src/services/launcher/scan.rs src-tauri/src/services/launcher/launch.rs src-tauri/src/ipc.rs
git commit -m "feat(launcher): 内置命令扩到九条并按 id 分发（含系统计算器）"
```

---

## Task 4: 共享结果源（`mergeLauncherResults` 纯函数 + `useLauncherResults` 组合式）

**Files:**
- Create: `src/utils/launcherResults.ts`
- Create: `src/utils/launcherResults.test.ts`
- Create: `src/composables/useLauncherResults.ts`

**Interfaces:**
- Consumes：`api.launcher.search` / `api.launcher.launch` / `api.launcher.hide`（既有）、Task 2 `api.browserHistory.search`、`api.windows.mindmapOpen` / `api.windows.panelOpenNote` / `api.windows.showMain`、`api.system.openUrl`、`useNotes` / `useTodos` / `useSettings`、`mindmapAllText` / `mindmapRootText`（`@/utils/search`）、`dateKeyOf` / `formatStamp`（`@/utils/datetime`）。
- Produces：
  - `src/utils/launcherResults.ts`：`LauncherAction`、`LauncherResult`、`LauncherScopes`、`MergeInputs`、`mergeLauncherResults(input: MergeInputs): LauncherResult[]`。
  - `src/composables/useLauncherResults.ts`：`LauncherActionKey`、`LauncherActionItem`、`useLauncherResults(options?: { scope?: string; floating?: boolean })` → `{ query, results, active, actions, actionIndex, reset, move, cycleAction, runActive, onKeydown }`。

> 说明：`mergeLauncherResults` 用 `import type` 引 `LauncherHit`（`@/service/tauri`），类型只在编译期存在、运行时被 Node 的类型剥离删掉，`tsconfig.test.json` 下可独立编译（已实测）。

- [ ] **Step 1: 先写失败的纯函数单测**

`src/utils/launcherResults.test.ts`：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import type { BrowserHistoryRow, LauncherHit } from '@/service/tauri'
import type { Note, Todo } from '@/typings/domain'
import { mergeLauncherResults, type LauncherScopes } from './launcherResults'

/** 五个范围开关默认全开，用例按需覆盖。 */
const ALL: LauncherScopes = { apps: true, notes: true, todos: true, calc: true, history: true }

function hit(partial: Partial<LauncherHit> & { name: string; path: string }): LauncherHit {
  return { id: 1, kind: 'app', score: 1, matched_keyword: '', ...partial }
}

function note(partial: Partial<Note> & { id: string; content: string }): Note {
  return {
    editor_mode: 'text',
    mindmap_data: null,
    tags: [],
    is_draft: false,
    pinned: false,
    archived_at: null,
    created_at: '2026-09-13T00:00:00+00:00',
    updated_at: '2026-09-13T00:00:00+00:00',
    ...partial,
  }
}

function todo(partial: Partial<Todo> & { id: string; content: string; due_at: string }): Todo {
  return {
    completed_at: null,
    status: 'open',
    remind_at: null,
    remind_offset_minutes: null,
    remind_desktop: true,
    remind_email: false,
    repeat_rule: null,
    remind_off: false,
    priority: 'medium',
    remark: '',
    parent_id: null,
    tags: [],
    created_at: partial.due_at,
    updated_at: partial.due_at,
    ...partial,
  }
}

function history(url: string, title: string): BrowserHistoryRow {
  return { url, title, visited_at: 1_700_000_000 }
}

/** 本地时区构造 RFC3339（与 island.test.ts 同款：避免断言依赖机器时区）。 */
function at(year: number, month: number, day: number, hour: number): string {
  return new Date(year, month - 1, day, hour, 0, 0, 0).toISOString()
}

test('mergeLauncherResults：空查询只保留后端常用项（笔记 / 待办 / 历史都不参与）', () => {
  const rows = mergeLauncherResults({
    apps: [hit({ name: 'Visual Studio Code', path: 'C:/vscode.exe' })],
    notes: [note({ id: 'n1', content: '会议纪要' })],
    todos: [todo({ id: 't1', content: '写周报', due_at: at(2026, 9, 13, 10) })],
    history: [history('https://a.example/', 'A')],
    query: '',
    scopes: ALL,
  })
  assert.deepEqual(
    rows.map((r) => r.id),
    ['app:C:/vscode.exe'],
  )
  assert.equal(rows[0].cat, '应用')
  assert.equal(rows[0].sub, 'C:/vscode.exe')
})

test('mergeLauncherResults：`=` 与「计算器 / calc」都产出首条计算器条目，普通查询不产出', () => {
  const base = { apps: [], notes: [], todos: [], history: [], scopes: ALL }
  for (const query of ['=', '= 1+2', '计算器', 'calc']) {
    const rows = mergeLauncherResults({ ...base, query })
    assert.equal(rows[0]?.id, 'calc', query)
    assert.deepEqual(rows[0].action, { type: 'calc' })
    assert.equal(rows[0].ico, '🧮')
    assert.equal(rows[0].cat, '计算器')
    assert.equal(rows[0].sub, '打开系统计算器')
  }
  assert.equal(mergeLauncherResults({ ...base, query: '记事本' }).length, 0)
})

test('mergeLauncherResults：顺序为 计算器 → 应用 / 命令 → 笔记 → 待办 → 历史', () => {
  const rows = mergeLauncherResults({
    apps: [
      hit({ id: 1, kind: 'app', name: 'Visual Studio Code', path: 'C:/vscode.exe' }),
      hit({ id: 2, kind: 'command', name: '🧮 计算器', path: 'cmd:calc' }),
    ],
    notes: [note({ id: 'n1', content: 'calc 笔记' })],
    todos: [todo({ id: 't1', content: 'calc 待办', due_at: at(2026, 9, 13, 10), priority: 'high' })],
    history: [history('https://calc.example/', 'calc 页面')],
    query: 'calc',
    scopes: ALL,
  })
  assert.deepEqual(
    rows.map((r) => r.cat),
    ['计算器', '应用', '命令', '笔记', '待办', '历史'],
  )
  // 命令名里的 emoji 拆进 .li-ico，名称不含 emoji、也没有 path 副文案。
  const command = rows[2]
  assert.equal(command.ico, '🧮')
  assert.equal(command.name, '计算器')
  assert.equal(command.sub, undefined)
  // 待办副文案 = 归属日 · 优先级。
  assert.equal(rows[4].sub, '2026-09-13 · 优先级 高')
  // 历史：标题为空时回退 URL。
  assert.equal(rows[5].name, 'calc 页面')
})

test('mergeLauncherResults：范围开关逐项关闭后对应分类消失', () => {
  const input = {
    apps: [hit({ kind: 'app' as const, name: 'Code', path: 'C:/code.exe' })],
    notes: [note({ id: 'n1', content: 'code 笔记' })],
    todos: [todo({ id: 't1', content: 'code 待办', due_at: at(2026, 9, 13, 10) })],
    history: [history('https://code.example/', 'code')],
    query: 'code',
  }
  const cases: { scopes: LauncherScopes; gone: string }[] = [
    { scopes: { ...ALL, apps: false }, gone: '应用' },
    { scopes: { ...ALL, notes: false }, gone: '笔记' },
    { scopes: { ...ALL, todos: false }, gone: '待办' },
    { scopes: { ...ALL, calc: false }, gone: '计算器' },
    { scopes: { ...ALL, history: false }, gone: '历史' },
  ]
  for (const { scopes, gone } of cases) {
    const rows = mergeLauncherResults({ ...input, scopes })
    const cats = rows.map((r) => r.cat)
    assert.ok(!cats.includes(gone), `关闭「${gone}」后仍出现：${cats.join(',')}`)
    // 其他分类仍在。
    assert.ok(cats.length > 0, `关闭「${gone}」后什么都不剩`)
  }
  // 计算器开关关掉后 `=` 查询不再有计算器条目。
  assert.deepEqual(
    mergeLauncherResults({
      apps: [],
      notes: [],
      todos: [],
      history: [],
      query: '=',
      scopes: { ...ALL, calc: false },
    }),
    [],
  )
})

test('mergeLauncherResults：笔记按正文 / 标签 / 导图节点文本命中，导图用 🧠 且取前 8 条', () => {
  const notes = [
    ...Array.from({ length: 9 }, (_, i) => note({ id: `n${i}`, content: `项目会议 ${i}` })),
    note({ id: 'tag', content: '无关正文', tags: ['项目'] }),
    note({
      id: 'map',
      content: '',
      editor_mode: 'mindmap',
      mindmap_data: JSON.stringify({
        data: { text: '项目规划' },
        children: [{ data: { text: '里程碑' } }],
      }),
    }),
  ]
  const rows = mergeLauncherResults({
    apps: [],
    notes,
    todos: [],
    history: [],
    query: '项目',
    scopes: ALL,
  })
  const noteRows = rows.filter((r) => r.cat === '笔记')
  // 9 条正文命中被限流到 8 条，标签与导图不算进这 8 条（它们排在其后）。
  assert.equal(noteRows.filter((r) => r.id.startsWith('note:n')).length, 8)
  assert.ok(noteRows.some((r) => r.id === 'note:tag'))
  const map = noteRows.find((r) => r.id === 'note:map')
  assert.ok(map, '导图节点文本应命中')
  assert.equal(map.ico, '🧠')
  assert.equal(map.name, '思维导图: 项目规划')
  assert.deepEqual(map.action, { type: 'note', id: 'map', mindmap: true })
})
```

Run: `pnpm test:unit 2>&1 | grep -E "launcherResults|Cannot find" | head`
Expected: `tsc -p tsconfig.test.json` 报 `Cannot find module './launcherResults'`（或该模块无导出）。

- [ ] **Step 2: 实现 `src/utils/launcherResults.ts`**

```ts
import type { BrowserHistoryRow, LauncherHit } from '@/service/tauri'
import type { Note, Priority, Todo } from '@/typings/domain'
import { dateKeyOf, formatStamp } from './datetime'
import { mindmapAllText, mindmapRootText } from './search'

/**
 * 启动台结果源纯函数（原型 app.js `collectLauncherResults`，spec 4C §4.1）。
 *
 * 顺序与原型一致：计算器 → 应用 / 命令 / 文件 → 笔记 → 待办 → 历史；空查询只保留
 * 后端返回的应用 / 命令段（常用项）。五个检索范围开关逐段短路。
 *
 * 不碰 DOM、不读全局状态——笔记 / 待办 / 历史都由调用方取好传进来，
 * 浮窗启动台与主窗口启动台页共用同一份实现，保证两处结果完全一致。
 *
 * 只引 `@/service/tauri` 的**类型**（`import type`，运行时被剥掉），
 * 因此本模块可以直接跑 `node:test`（tsconfig.test.json 不含 tauri 前端实现）。
 */

/** 一条结果要执行的动作；由 `useLauncherResults` 解释成实际调用。 */
export type LauncherAction =
  | { type: 'launch'; path: string; kind: LauncherHit['kind'] }
  | { type: 'note'; id: string; mindmap: boolean }
  | { type: 'todos' }
  | { type: 'calc' }
  | { type: 'url'; url: string }

/** 一条可展示、可执行的结果（原型 `{ ico, name, sub, cat, run }`）。 */
export interface LauncherResult {
  /** 稳定 key（`分类:标识`），模板 v-for 用。 */
  id: string
  /** emoji 图标（`.li-ico`）。 */
  ico: string
  name: string
  /** `.li-name > small` 副文案；空则不渲染。 */
  sub?: string
  /** `.li-cat` 分类文案（应用 / 文件 / 文件夹 / 命令 / 笔记 / 待办 / 计算器 / 历史）。 */
  cat: string
  action: LauncherAction
}

/** 检索范围开关（对应设置里的五个 `launcher_scope_*` 键）。 */
export interface LauncherScopes {
  apps: boolean
  notes: boolean
  todos: boolean
  calc: boolean
  history: boolean
}

export interface MergeInputs {
  /** 后端命中（应用 / UWP / 文件 / 文件夹 / 命令）。 */
  apps: LauncherHit[]
  /** 全量笔记（调用方已取；命中判定在前端做）。 */
  notes: Note[]
  todos: Todo[]
  /** 后端按当前查询取回的历史（范围关时为空数组）。 */
  history: BrowserHistoryRow[]
  query: string
  scopes: LauncherScopes
}

/** 每档命中的条数上限：原型全量拼接，这里限流避免长列表把小分类挤出视野。 */
const SECTION_LIMIT = 8
/** 名称截断长度（原型 `n.text.slice(0, 36)`）。 */
const NAME_LIMIT = 36

/** 笔记的命中原因，同时决定展示顺序（控制者订正 2026-09-13，见笔记段注释）。 */
type NoteHitReason = 'content' | 'tag' | 'mindmap'
/** 笔记分段顺序（与 `NoteHitReason` 声明顺序一致）。 */
const NOTE_HIT_ORDER: NoteHitReason[] = ['content', 'tag', 'mindmap']

/** 笔记命中判定：正文 → 标签 → 导图节点文本，取首个命中的原因；都不中返回 null。 */
function noteHitReason(note: Note, needle: string): NoteHitReason | null {
  if (note.content.toLowerCase().includes(needle)) return 'content'
  if (note.tags.some((tag) => tag.toLowerCase().includes(needle))) return 'tag'
  return mindmapAllText(note.mindmap_data).toLowerCase().includes(needle) ? 'mindmap' : null
}

/** 命中类型 → 展示分类（`.li-cat`）。 */
const KIND_CAT: Record<LauncherHit['kind'], string> = {
  app: '应用',
  uwp: '应用',
  file: '文件',
  folder: '文件夹',
  command: '命令',
}

/** 命中类型 → 图标；命令的名称自带 emoji（`scan.rs::builtin_commands`），不套这一格。 */
const KIND_ICON: Record<LauncherHit['kind'], string> = {
  app: '🖥',
  uwp: '🧩',
  file: '📄',
  folder: '📁',
  command: '⚡',
}

/** 优先级文案（原型待办副文案 `优先级 ${t.priority}`）。 */
const PRIORITY_LABEL: Record<Priority, string> = { high: '高', medium: '中', low: '低' }

/** 单行化并截断（原型 `n.text.slice(0, 36)`；带换行的正文会让条目高度跳动）。 */
function clip(text: string): string {
  const flat = text.replace(/\s+/g, ' ').trim()
  return flat.length > NAME_LIMIT ? `${flat.slice(0, NAME_LIMIT)}…` : flat
}

/**
 * 拆出内置命令名里的 emoji：后端按 spec §4.5 把 emoji 写进名称（`🧮 计算器`），
 * 而 `.li-ico` 是独立的图标位——不拆会出现「图标位空白 + 名称里再带一个 emoji」。
 * 名称不含空格时整串当作文案，图标退回 ⚡。
 */
function splitCommandName(raw: string): { ico: string; name: string } {
  const index = raw.indexOf(' ')
  if (index <= 0) return { ico: KIND_ICON.command, name: raw }
  return { ico: raw.slice(0, index), name: raw.slice(index + 1) }
}

/** 后端命中 → 结果条目。 */
function fromHit(hit: LauncherHit): LauncherResult {
  if (hit.kind === 'command') {
    // 命令的 path 是 `cmd:x` 这类标识，不作为副文案展示（原型命令条目的 sub 为空）。
    const { ico, name } = splitCommandName(hit.name)
    return {
      id: `command:${hit.path}`,
      ico,
      name,
      cat: KIND_CAT.command,
      action: { type: 'launch', path: hit.path, kind: hit.kind },
    }
  }
  return {
    id: `${hit.kind}:${hit.path}`,
    ico: KIND_ICON[hit.kind],
    name: hit.name,
    sub: hit.path,
    cat: KIND_CAT[hit.kind],
    action: { type: 'launch', path: hit.path, kind: hit.kind },
  }
}

/** 合并各数据源（原型 `collectLauncherResults`）。 */
export function mergeLauncherResults(input: MergeInputs): LauncherResult[] {
  const query = input.query.trim()
  const needle = query.toLowerCase()
  const out: LauncherResult[] = []

  // 计算器（D38）：`=` 开头或命中「计算器 / calc」时置顶，执行后端 `cmd:calc`（打开系统计算器）。
  if (input.scopes.calc && (query.startsWith('=') || /计算器|calc/i.test(query))) {
    out.push({
      id: 'calc',
      ico: '🧮',
      name: '计算器',
      sub: '打开系统计算器',
      cat: '计算器',
      action: { type: 'calc' },
    })
  }

  if (input.scopes.apps) out.push(...input.apps.map(fromHit))

  // 空查询到此为止：原型只在有查询词时才搜笔记 / 待办 / 历史。
  if (!query) return out

  if (input.scopes.notes) {
    // 按命中原因分档后各自限流（控制者订正 2026-09-13：原示例把三种命中合成一个集合再
    // slice(0, 8)，与同一任务用例 5 的断言冲突——标签 / 导图命中会被正文命中整段挤掉；
    // 实际实现见 513fd53，口径已回填 spec §4.1：正文 → 标签 → 导图，每档 ≤8）。
    const buckets: Record<NoteHitReason, Note[]> = { content: [], tag: [], mindmap: [] }
    for (const note of input.notes) {
      const reason = noteHitReason(note, needle)
      if (reason) buckets[reason].push(note)
    }
    for (const reason of NOTE_HIT_ORDER) {
      out.push(...buckets[reason].slice(0, SECTION_LIMIT).map((note): LauncherResult => {
        const mindmap = note.editor_mode === 'mindmap'
        const label = mindmap ? mindmapRootText(note.mindmap_data) : clip(note.content)
        return {
          id: `note:${note.id}`,
          ico: mindmap ? '🧠' : '📝',
          name: `${mindmap ? '思维导图' : '笔记'}: ${label}`,
          // 原型：有标签显示标签，否则显示时间（formatStamp 口径）。
          sub: note.tags.length ? `[${note.tags.join(', ')}]` : formatStamp(note.updated_at),
          cat: '笔记',
          action: { type: 'note', id: note.id, mindmap },
        }
      }))
    }
  }

  if (input.scopes.todos) {
    const matched = input.todos.filter((todo) => todo.content.toLowerCase().includes(needle))
    out.push(
      ...matched.slice(0, SECTION_LIMIT).map(
        (todo): LauncherResult => ({
          id: `todo:${todo.id}`,
          ico: '✅',
          name: `待办: ${clip(todo.content)}`,
          sub: `${dateKeyOf(todo.due_at)} · 优先级 ${PRIORITY_LABEL[todo.priority]}`,
          cat: '待办',
          action: { type: 'todos' },
        }),
      ),
    )
  }

  if (input.scopes.history) {
    out.push(
      ...input.history.map(
        (row): LauncherResult => ({
          id: `history:${row.url}`,
          ico: '🌐',
          name: row.title || row.url,
          sub: row.url,
          cat: '历史',
          action: { type: 'url', url: row.url },
        }),
      ),
    )
  }

  return out
}
```

Run: `pnpm test:unit 2>&1 | grep -E "mergeLauncherResults|^# (pass|fail)|ℹ (pass|fail)"`
Expected: 五个用例 `ok`，`fail 0`。

- [ ] **Step 3: 实现 `src/composables/useLauncherResults.ts`**

```ts
import { computed, onBeforeUnmount, ref, watch, type ComputedRef, type Ref } from 'vue'
import { useNotes, useSettings, useTodos } from '@/composables/useData'
import { logger } from '@/service/logger'
import { api, type BrowserHistoryRow, type LauncherHit } from '@/service/tauri'
import { mergeLauncherResults, type LauncherResult, type LauncherScopes } from '@/utils/launcherResults'

/**
 * 启动台共享结果源（浮窗启动台与主窗口启动台页共用，spec 4C §4.1）。
 *
 * 取数：查询去抖 30ms → `api.launcher.search`（应用 / 命令 / 文件）与
 * `api.browserHistory.search`（仅历史开关打开时；关闭时清空本地结果，重新打开补一次查询）；
 * 笔记 / 待办来自 `useNotes` / `useTodos` 的全量列表（随数据变更事件自动刷新），
 * 怎么合并交给纯函数 `mergeLauncherResults`。
 *
 * 执行：`runActive` 按条目动作分派——launch 走后端启动（mode 由 Tab / Ctrl+Enter 决定）、
 * note 先收起浮窗再开导图 / 回显面板、todos 打开主窗口待办页、calc 走 `cmd:calc`、
 * url 用系统默认浏览器打开。
 *
 * 键盘：`floating: true`（浮窗）额外启用 Esc 隐藏、Tab 循环动作、Alt+1..9 直达；
 * 启动台页只保留 ↑↓ 与 Enter（原型行为），同时不抢走 Tab 的焦点移动。
 */

/** 可直接执行的动作；key 同时是后端 `LaunchMode` 的字面量。 */
export type LauncherActionKey = 'open' | 'admin' | 'reveal'

/** 动作栏的一项。 */
export interface LauncherActionItem {
  key: LauncherActionKey
  label: string
}

/** 动作全集；只有「应用 / 文件 / 文件夹」支持后两项，其余条目只给「打开」。 */
const ACTIONS: LauncherActionItem[] = [
  { key: 'open', label: '打开' },
  { key: 'admin', label: '管理员' },
  { key: 'reveal', label: '打开所在文件夹' },
]

/** 输入去抖（ms）：输入即搜，但避免每个按键都打一次 IPC。 */
const DEBOUNCE_MS = 30

export interface UseLauncherResults {
  query: Ref<string>
  results: ComputedRef<LauncherResult[]>
  active: Ref<number>
  /** 当前条目可用的动作（Tab 循环）。 */
  actions: ComputedRef<LauncherActionItem[]>
  actionIndex: Ref<number>
  /** 清空查询并把选中归零（浮窗每次显示调用）。 */
  reset: () => void
  move: (delta: number) => void
  cycleAction: () => void
  runActive: (index?: number, mode?: LauncherActionKey) => Promise<void>
  onKeydown: (event: KeyboardEvent) => void
}

export function useLauncherResults(
  options: { scope?: string; floating?: boolean } = {},
): UseLauncherResults {
  const scope = options.scope ?? 'launcher'
  // 浮窗专属键位 + 无结果回车后收起浮窗；启动台页两者都不做（原型页面行为）。
  const floating = options.floating ?? false
  const { settings } = useSettings()
  const { notes } = useNotes()
  const { todos } = useTodos()

  const query = ref('')
  const active = ref(0)
  const actionIndex = ref(0)
  /** 后端应用 / 命令 / 文件命中。 */
  const apps = ref<LauncherHit[]>([])
  /** 后端浏览器历史命中（历史开关关闭时恒为空）。 */
  const history = ref<BrowserHistoryRow[]>([])
  let debounce: ReturnType<typeof setTimeout> | null = null

  async function search(): Promise<void> {
    const keyword = query.value
    try {
      if (settings.value.launcher_scope_history) {
        // 两路并行：历史走自有表，与应用检索互不依赖。
        const [hits, rows] = await Promise.all([api.launcher.search(keyword), api.browserHistory.search(keyword)])
        apps.value = hits
        history.value = rows
      } else {
        apps.value = await api.launcher.search(keyword)
        history.value = []
      }
      active.value = 0
      actionIndex.value = 0
      logger.debug(scope, `检索「${keyword}」命中 ${apps.value.length + history.value.length} 条后端结果`)
    } catch (error) {
      logger.error(scope, '检索失败', error)
      apps.value = []
      history.value = []
    }
  }

  watch(
    query,
    () => {
      if (debounce) clearTimeout(debounce)
      debounce = setTimeout(() => void search(), DEBOUNCE_MS)
    },
    { immediate: true },
  )

  // 历史开关重新打开时补一次查询（关闭期间不发请求，本地结果已清空）。
  watch(
    () => settings.value.launcher_scope_history,
    (on) => {
      if (on) void search()
    },
  )

  const scopes = computed<LauncherScopes>(() => ({
    apps: settings.value.launcher_scope_apps,
    notes: settings.value.launcher_scope_notes,
    todos: settings.value.launcher_scope_todos,
    calc: settings.value.launcher_scope_calc,
    history: settings.value.launcher_scope_history,
  }))

  const results = computed<LauncherResult[]>(() =>
    mergeLauncherResults({
      apps: apps.value,
      notes: notes.value,
      todos: todos.value,
      history: history.value,
      query: query.value,
      scopes: scopes.value,
    }),
  )

  // 结果集变短时把选中拉回范围内（例如刚关掉某个检索范围）。
  watch(results, (list) => {
    if (active.value >= list.length) active.value = Math.max(0, list.length - 1)
    if (actionIndex.value > 0) actionIndex.value = 0
  })

  const actions = computed<LauncherActionItem[]>(() => {
    const action = results.value[active.value]?.action
    const multi = action?.type === 'launch' && action.kind !== 'command' && action.kind !== 'uwp'
    return multi ? ACTIONS : ACTIONS.slice(0, 1)
  })

  function reset(): void {
    query.value = ''
    active.value = 0
    actionIndex.value = 0
  }

  function move(delta: number): void {
    if (!results.value.length) return
    active.value = (active.value + delta + results.value.length) % results.value.length
    actionIndex.value = 0
  }

  function cycleAction(): void {
    if (actions.value.length < 2) return
    actionIndex.value = (actionIndex.value + 1) % actions.value.length
  }

  async function runActive(index = active.value, mode: LauncherActionKey = 'open'): Promise<void> {
    const result = results.value[index]
    if (!result) return
    logger.info(scope, `执行 ${result.id}`)
    try {
      switch (result.action.type) {
        case 'launch': {
          // 只有应用 / 文件 / 文件夹支持管理员与定位，其余条目忽略传入的 mode。
          const kind = result.action.kind
          const actionable = kind === 'app' || kind === 'file' || kind === 'folder'
          await api.launcher.launch(result.action.path, kind, actionable ? mode : 'open', query.value)
          return
        }
        case 'note':
          // 先收起浮窗再开导图 / 回显面板，避免启动台压在新窗口上（原型 closeLauncher）。
          await api.launcher.hide()
          if (result.action.mindmap) await api.windows.mindmapOpen(result.action.id)
          else await api.windows.panelOpenNote(result.action.id)
          return
        case 'todos':
          await api.launcher.hide()
          await api.windows.showMain('todos')
          return
        case 'calc':
          // 不内置表达式求值（D38）：后端拉起系统计算器。
          await api.launcher.launch('cmd:calc', 'command', 'open', query.value)
          return
        case 'url':
          // 用系统默认浏览器打开；启动台靠失焦自动隐藏（不显式 hide，避免与浏览器抢焦点打架）。
          await api.system.openUrl(result.action.url)
          return
      }
    } catch (error) {
      logger.error(scope, '执行失败', error)
    }
  }

  /**
   * 键盘：↑↓ 循环、Enter 执行（无结果 → 默认浏览器搜索该词，原型行为）；
   * 浮窗额外支持 Tab 循环动作、Alt+1..9 直达、Ctrl+Enter 管理员、Esc 隐藏。
   */
  function onKeydown(event: KeyboardEvent): void {
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        move(1)
        return
      case 'ArrowUp':
        event.preventDefault()
        move(-1)
        return
      case 'Tab':
        if (!floating) return
        event.preventDefault()
        cycleAction()
        return
      case 'Enter': {
        event.preventDefault()
        if (!results.value.length) {
          const keyword = query.value.trim()
          if (keyword) {
            logger.info(scope, '无结果，改用默认浏览器搜索')
            void api.system.openUrl(`https://www.google.com/search?q=${encodeURIComponent(keyword)}`)
            if (floating) void api.launcher.hide()
          }
          return
        }
        const mode: LauncherActionKey =
          event.ctrlKey && actions.value.length > 1 ? 'admin' : (actions.value[actionIndex.value]?.key ?? 'open')
        void runActive(active.value, mode)
        return
      }
      case 'Escape':
        if (!floating) return
        event.preventDefault()
        void api.launcher.hide()
        return
      default:
        // Alt+1..9 直达（D40）：执行第 N 条的「打开」。
        if (floating && event.altKey && /^[1-9]$/.test(event.key)) {
          const index = Number(event.key) - 1
          if (index < results.value.length) {
            event.preventDefault()
            void runActive(index, 'open')
          }
        }
    }
  }

  onBeforeUnmount(() => {
    if (debounce) clearTimeout(debounce)
  })

  return { query, results, active, actions, actionIndex, reset, move, cycleAction, runActive, onKeydown }
}
```

Run: `pnpm typecheck && pnpm format:check && pnpm test:unit 2>&1 | grep -E "ℹ (pass|fail)"`
Expected: `typecheck` 0 错误；`pass` 增加 5 例、`fail 0`。

- [ ] **Step 4: 校验并提交**

```bash
pnpm test:unit 2>&1 | grep -E "ℹ (pass|fail)" && pnpm typecheck && pnpm format:check
git add src/utils/launcherResults.ts src/utils/launcherResults.test.ts src/composables/useLauncherResults.ts
git commit -m "feat(launcher): 共享结果源（笔记 / 待办 / 计算器 / 历史）+ 纯函数与单测"
```

---

## Task 5: 浮窗启动台对齐原型（DOM / 页脚 / 入场 / 560×420 / 呼出收起面板）

**Files:**
- Modify: `src/windows/Launcher/LauncherApp.vue`（整体重写）
- Modify: `src/windows/launcher.ts:5-11`
- Delete: `src/styles/launcher.css`
- Modify: `src/styles/window-fit.css:141`（灵动岛块之后插入启动台窗口块）
- Modify: `src/styles/extensions.css`（文件末尾追加 §11）
- Modify: `src-tauri/src/app/windows.rs:1150-1152,1157-1158`

**Interfaces:**
- Consumes：Task 4 `useLauncherResults({ scope: 'launcher', floating: true })`。
- Produces：浮窗根元素 `#launcherWindow.glass > .launcher-input-row + .launcher-list + .launcher-footer`；`launcher-keys` 类（`extensions.css` §11）；`windows.rs` `LAUNCHER_WIDTH = 560.0` / `LAUNCHER_HEIGHT = 420.0`；`launcher_show` 呼出前 `panel_hide`。

- [ ] **Step 1: 重写 `src/windows/Launcher/LauncherApp.vue`**

```vue
<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { useLauncherResults } from '@/composables/useLauncherResults'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { enter } from '@/motion'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 浮窗启动台（原型 `#launcherWindow`，spec 4C §4.2）。
 *
 * - DOM 按原型：`.glass > .launcher-input-row(.launcher-ico + #launcherInput) + .launcher-list + .launcher-footer`；
 * - 结果源与主窗口启动台页共用 `useLauncherResults`（应用 / 命令 / 笔记 / 待办 / 计算器 / 浏览器历史，
 *   各受检索范围开关控制）；页脚两态：有结果「↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 N 项」、
 *   无结果「↵ 回车搜索 · Esc 退出」，右侧并入动作 / 快捷键提示（D40）；
 * - 键盘：↑↓ 循环、Enter 执行（无结果 → 默认浏览器搜索）、Tab 切动作、Alt+1..9 直达、
 *   Ctrl+Enter 管理员、Esc 隐藏——监听器挂 window 而非输入框：点击条目后焦点会离开输入框，
 *   挂 window 才能继续响应键盘（现有实现即如此）；
 * - 每次显示（window focus）：清空查询、选中归零、聚焦输入框、播入场（y −18 + 淡入 + scale .98）；
 * - 失焦即隐藏（点了别处）——与原型「点击窗外关闭」能力等价。
 */
applyCachedTheme()
applyCachedGlass()
document.documentElement.dataset.window = 'launcher'

const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
watch(
  () => settings.value.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
)
watch(
  () => settings.value.glass_level,
  (level) => applyGlass(level),
  { immediate: true },
)

const { query, results, active, actions, actionIndex, reset, runActive, onKeydown } = useLauncherResults({
  scope: 'launcher',
  floating: true,
})

const root = ref<HTMLElement | null>(null)
const input = ref<HTMLInputElement | null>(null)

/** 页脚左侧（原型 renderLauncherResults 的两态文案）。 */
const footerHint = computed(() =>
  results.value.length
    ? `↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 ${results.value.length} 项`
    : '↵ 回车搜索 · Esc 退出',
)

/**
 * 页脚右侧（D40，原型无此段）：当前动作 + 键位提示。
 * 命令 / UWP 等只有「打开」的条目省略 Tab 与 Ctrl+Enter 段——它们没有管理员 / 定位动作，
 * 提示了也执行不了（后端会直接报错）。
 */
const actionsHint = computed(() => {
  if (actions.value.length < 2) return 'Alt+1..9 直达'
  const current = actions.value[actionIndex.value]?.label ?? '打开'
  return `当前动作：${current} · Tab 切换动作 · Alt+1..9 直达 · Ctrl+Enter 管理员`
})

/** 点击条目：默认「打开」。 */
function run(index: number): void {
  void runActive(index)
}

/** 窗口每次显示：清空、聚焦、播入场（原型 openLauncher）。 */
function onFocus(): void {
  reset()
  void nextTick(() => input.value?.focus())
  if (root.value) void enter(root.value, { axis: 'y', distance: -18, scale: 0.98, duration: 'base' })
  logger.debug('launcher', '浮窗显示：已清空输入并聚焦')
}

/** 失焦即隐藏（点了别处）。 */
function onBlur(): void {
  void api.launcher.hide()
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('focus', onFocus)
  window.addEventListener('blur', onBlur)
  onFocus()
  logger.info('launcher', '启动台浮窗已挂载')
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('focus', onFocus)
  window.removeEventListener('blur', onBlur)
})
</script>

<template>
  <div id="launcherWindow" ref="root" class="glass">
    <div class="launcher-input-row">
      <span class="launcher-ico">🚀</span>
      <input
        id="launcherInput"
        ref="input"
        v-model="query"
        type="text"
        placeholder="搜索应用 / 命令 / 笔记 / 待办 / 历史，= 开头打开计算器"
        spellcheck="false"
        autocomplete="off"
      />
    </div>

    <div class="launcher-list">
      <div
        v-for="(result, index) in results"
        :key="result.id"
        class="launcher-item"
        :class="{ active: index === active }"
        @mousemove="active = index"
        @click="run(index)"
      >
        <span class="li-ico">{{ result.ico }}</span>
        <span class="li-name"
          >{{ result.name }}<small v-if="result.sub">{{ result.sub }}</small></span
        >
        <span class="li-cat">{{ result.cat }}</span>
      </div>
      <div v-if="!results.length" class="launcher-empty">
        未找到匹配项，按 Enter 用默认浏览器搜索「{{ query.trim() }}」
      </div>
    </div>

    <div class="launcher-footer">
      <span>{{ footerHint }}</span>
      <span class="launcher-keys">{{ actionsHint }}</span>
    </div>
  </div>
</template>
```

- [ ] **Step 2: `launcher.ts` 删 `launcher.css` 引入；删除文件**

`src/windows/launcher.ts` 第 7–11 行替换为：

```ts
import { createApp } from 'vue'
import LauncherApp from './Launcher/LauncherApp.vue'
import '@/styles'
```

头注释第 4–5 行 `全局快捷键（默认 Alt+Space）呼出的键盘驱动一次性查询窗口：光标所在屏居中偏上。\n* 输入即搜（Rust 侧索引 + 匹配），↑/↓ 选择、Enter 启动、Ctrl+Enter 管理员、Tab 切动作、Esc/失焦隐藏。` 替换为 `全局快捷键（默认 Alt+Space）呼出的键盘驱动一次性查询窗口：光标所在屏居中偏上、560×420。\n* 输入即搜（后端索引 + 笔记 / 待办 / 浏览器历史），↑/↓ 选择、Enter 执行、Tab 切动作、Alt+1..9 直达、Ctrl+Enter 管理员、Esc/失焦隐藏。`。

Run: `git rm src/styles/launcher.css`

- [ ] **Step 3: `window-fit.css` 补启动台窗口块**

第 128–141 行的灵动岛块收尾（`  clip-path: inset(0 round 999px);\n}`）之后插入：

```css

/* ═══ 启动台窗口 ═══
 * 原型把 #launcherWindow 当桌面浮层（fixed + top:16vh + translateX(-50%) + min(92vw, 560px)）；
 * 真实窗口就是启动台本身，位置与 560×420 由 Rust 落位（windows.rs::launcher_show），
 * 这里铺满窗口、取消浮层定位，并让结果列表吃掉输入行与页脚之外的剩余高度（D37「列表按内容撑满」）。
 * 毛玻璃绘制会盖掉圆角，与面板同理用 clip-path 裁齐。 */
:root[data-window='launcher'],
:root[data-window='launcher'] body,
:root[data-window='launcher'] #app {
  background: transparent !important;
  overflow: hidden;
}
#launcherWindow {
  position: static;
  top: auto;
  left: auto;
  transform: none;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100vh;
  clip-path: inset(0 round 14px);
}
:root[data-window='launcher'] .launcher-list {
  flex: 1;
  min-height: 0;
  max-height: none;
}
```

- [ ] **Step 4: `extensions.css` 追加 §11**

文件末尾（`.pinned-editor` 规则之后，第 861 行之后）追加：

```css

/* ═══ 11. 浮窗启动台页脚（阶段四 C，2026-09-13） ═══
 * 原型 .launcher-footer 只有左侧一行导航提示；D40 把现有的动作 / 快捷键提示并入其右侧。
 * 生成层没有对应的两栏布局，这里只补 flex 与右对齐——内距 / 字号 / 颜色 / 分隔线仍由生成层提供。
 * 用 :root[data-window='launcher'] 限定，避免影响主窗口启动台页的 .launcher-footer（原型页脚只有左侧一段）。 */
:root[data-window='launcher'] .launcher-footer {
  display: flex;
  align-items: center;
  gap: 12px;
}
:root[data-window='launcher'] .launcher-keys {
  margin-left: auto;
  flex: none;
}
```

- [ ] **Step 5: `windows.rs` 尺寸与呼出顺序**

第 1150–1152 行三个常量替换为：

```rust
/// 宽对齐原型 `#launcherWindow` 的 `min(92vw, 560px)`（spec 4C D37）。
const LAUNCHER_WIDTH: f64 = 560.0;
/// 高对齐原型启动台窗口（列表按内容撑满，spec 4C D37）。
const LAUNCHER_HEIGHT: f64 = 420.0;
```

`launcher_show` 函数体第一行（第 1158 行 `let monitor = …` 之前）插入：

```rust
    // D39：呼出前先收起面板（原型 openLauncher 的 hidePanel）；主窗口保持原样，
    // 用户在主窗口的操作上下文不丢（原型的 closeAllWindows 不采用）。
    if let Err(error) = panel_hide(app) {
        eprintln!("[launcher] 呼出前收起面板失败: {error}");
    }
```

- [ ] **Step 6: 校验并提交**

```bash
pnpm sync:styles --check && pnpm typecheck && pnpm format:check && (grep -rn "launcher-actions\|launcher-actions-hint\|launcher-search-icon\|launcher-kind\|launcher-texts\|launcher-hint\|styles/launcher.css" src || echo "无残留引用")
cd src-tauri && cargo fmt --check && cargo check && cd ..
pnpm exec vite build 2>&1 | grep -E "built in|error"
git add src/windows/Launcher/LauncherApp.vue src/windows/launcher.ts src/styles/launcher.css src/styles/window-fit.css src/styles/extensions.css src-tauri/src/app/windows.rs
git commit -m "feat(launcher): 浮窗启动台对齐原型（DOM / 页脚 / 入场 / 560×420 / 呼出收起面板），删除 launcher.css"
```

---

## Task 6: 启动台页补检索范围与历史设置行

**Files:**
- Modify: `src/windows/Main/LauncherPageView.vue`（脚本段 1–79、模板 81–190）
- Delete: `src/composables/useLauncherSearch.ts`

**Interfaces:**
- Consumes：Task 4 `useLauncherResults({ scope: 'launcher-page' })`；Task 1 `Settings.launcher_scope_*` / `launcher_history_retention_days`；Task 2 `api.browserHistory.count`。
- Produces：设置区行序 = 全局呼出快捷键 → 检索范围：应用与命令 → 检索范围：笔记 → 检索范围：待办 → 计算器（= 开头，打开系统计算器）→ 检索范围：浏览器历史 → 历史保留天数 → 已记录历史地址 → 浮窗启动台 → 项目扩展（全盘文件索引 / 额外排除目录 / 索引 / 文件扫描目录）。

- [ ] **Step 1: 脚本段换结果源**

第 2–10 行 import 替换为：

```ts
import { computed, onMounted, ref } from 'vue'
import { useSettings } from '@/composables/useData'
import { useLauncherResults } from '@/composables/useLauncherResults'
import { useLauncherSettings } from '@/composables/useLauncherSettings'
import { useShortcutRecorder } from '@/composables/useShortcutRecorder'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Settings } from '@/typings/domain'
```

第 33 行 `const { query, hits, active, run, onKeydown } = useLauncherSearch()` 替换为：

```ts
// 结果源与浮窗启动台同源（useLauncherResults + mergeLauncherResults），页面内可直接试用全部数据源。
const { query, results, active, runActive, onKeydown } = useLauncherResults({ scope: 'launcher-page' })

/** 点击条目：默认「打开」。 */
function run(index: number): void {
  void runActive(index)
}
```

第 61–66 行 `footer` / `emptyHint` 替换为：

```ts
/** 页脚提示（原型 renderLauncherPageResults 的两态文案：无结果时提示回车搜索）。 */
const footer = computed(() =>
  results.value.length
    ? `↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 ${results.value.length} 项`
    : '↵ 回车搜索 · 点击条目直接执行',
)

/** 空态文案：有查询词说明没搜到，空查询说明索引尚未就绪。 */
const emptyHint = computed(() => (query.value.trim() ? `未找到匹配「${query.value.trim()}」的项` : '索引尚未就绪'))
```

第 78 行 `onMounted(() => void refreshLauncherStatus())` 替换为：

```ts
/** 已记录历史地址条数（原型 lpHistoryCount：`N 条（保留近 M 天 · 无痕访问不记录）`）。 */
const historyCount = ref(0)

async function refreshHistoryCount(): Promise<void> {
  try {
    historyCount.value = await api.browserHistory.count()
  } catch (error) {
    logger.error('launcher-page', '读取历史条数失败', error)
  }
}

/** 历史保留天数：钳制到 1–365 后写库（后端会顺手清理过期记录），再刷新条数。 */
async function setHistoryRetention(raw: string): Promise<void> {
  const days = Math.min(365, Math.max(1, Number(raw) || 100))
  await patch({ launcher_history_retention_days: days })
  await refreshHistoryCount()
  toast(`浏览器历史保留天数已设为 ${days} 天`)
}

const historyCountText = computed(
  () => `${historyCount.value} 条（保留近 ${settings.value.launcher_history_retention_days} 天 · 无痕访问不记录）`,
)

onMounted(() => {
  void refreshLauncherStatus()
  // 原型进入启动台页即 pruneBrowserHistory 并刷新计数；这里等价地在挂载时读一次
  // （保留天数在设置页改动后由 settings_save 触发的 prune 保证库是最新的）。
  void refreshHistoryCount()
})
```

头注释第 15–18 行 `+ 「启动台设置」：全局快捷键录制、全盘索引开关、额外排除目录、索引状态与重建、扫描根目录、\n * 「呼出浮窗启动台」。原型中依赖笔记 / 待办 / 计算器 / 浏览器历史检索的控件，当前后端没有这些\n * 检索源，不渲染（spec D10，留阶段四）。` 替换为 `+ 「启动台设置」：全局快捷键录制、五个检索范围开关、历史保留天数与已记录地址数、全盘索引开关、\n * 额外排除目录、索引状态与重建、扫描根目录、「呼出浮窗启动台」（spec 4C §4.7；项目扩展行排在原型行之后）。`。

- [ ] **Step 2: 模板换结果源**

第 99–117 行 `.launcher-list` 整段替换为：

```vue
      <div class="launcher-list">
        <div
          v-for="(result, index) in results"
          :key="result.id"
          class="launcher-item"
          :class="{ active: index === active }"
          @mousemove="active = index"
          @click="run(index)"
        >
          <span class="li-ico">{{ result.ico }}</span>
          <span class="li-name"
            >{{ result.name }}<small v-if="result.sub">{{ result.sub }}</small></span
          >
          <span class="li-cat">{{ result.cat }}</span>
        </div>
        <div v-if="!results.length" class="launcher-empty">{{ emptyHint }}</div>
      </div>
```

第 90–97 行输入框的 placeholder 改为 `在此试用：搜索应用 / 命令 / 笔记 / 待办 / 历史，= 开头打开计算器`。

- [ ] **Step 3: 模板补七行设置（顺序见 spec §4.7）**

第 130 行「全局呼出快捷键」的 `</div>` 之后（第 131 行 `全盘文件索引` 的 `<label class="setting-row">` 之前）插入：

```vue
      <label class="setting-row">
        <span>检索范围：应用与命令</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_apps"
          @change="patch({ launcher_scope_apps: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>检索范围：笔记</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_notes"
          @change="patch({ launcher_scope_notes: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>检索范围：待办</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_todos"
          @change="patch({ launcher_scope_todos: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>计算器（= 开头，打开系统计算器）</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_calc"
          @change="patch({ launcher_scope_calc: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>检索范围：浏览器历史</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_history"
          @change="patch({ launcher_scope_history: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>历史保留天数</span>
        <input
          type="number"
          min="1"
          max="365"
          :value="settings.launcher_history_retention_days"
          @change="setHistoryRetention(($event.target as HTMLInputElement).value)"
        />
        天
      </label>
      <div class="setting-row">
        <span>已记录历史地址</span>
        <span class="clip-editor-hint">{{ historyCountText }}</span>
      </div>
```

- [ ] **Step 4: 删除 `useLauncherSearch.ts`**

Run: `git rm src/composables/useLauncherSearch.ts && (grep -rn "useLauncherSearch\|LAUNCHER_KIND_ICON\|LAUNCHER_KIND_LABEL" src || echo "无残留引用")`
Expected: `无残留引用`。

- [ ] **Step 5: 校验并提交**

```bash
pnpm typecheck && pnpm format:check && pnpm test:unit 2>&1 | grep -E "ℹ (pass|fail)"
git add src/windows/Main/LauncherPageView.vue src/composables/useLauncherSearch.ts
git commit -m "feat(launcher): 启动台页补检索范围与历史设置行"
```

---

## Task 7: 实机验收与记录

**Files:**
- Create: `docs/superpowers/plans/2026-09-13-prototype-realign-phase4c-acceptance.md`
- Modify: spec §9（`docs/superpowers/specs/2026-09-13-prototype-realign-phase4c-launcher-design.md:203-205`）；必要时补 `src/styles/extensions.css` §11 之后的「阶段四 C 验收补丁」小节并注明原因

- [ ] **Step 1: 静态检查全绿**

Run: `pnpm sync:styles --check && pnpm typecheck && pnpm test && pnpm format:check && (cd src-tauri && cargo fmt --check && cargo check && cargo test) && pnpm exec vite build`
Expected: 全部通过。记录 `test:unit` 通过数（含新增 `mergeLauncherResults` 5 例）、`cargo test` 通过数（含 `v8_migration_creates_browser_history_and_is_idempotent`、`webkit_timestamp_matches_known_values`、`search_matches_title_or_url_and_orders_desc`、`prune_drops_rows_outside_retention_window`、`import_file_reads_chromium_urls_table`、`builtin_commands_cover_nine_ids`）与 `launcher-*.js` / `main-*.js` 体积。
Run: `grep -rn "useLauncherSearch\|styles/launcher.css\|launcher-actions\|launcher-search-icon" src || echo "无残留"`
Expected: `无残留`。
Run: `grep -n "LAUNCHER_WIDTH\|LAUNCHER_HEIGHT" src-tauri/src/app/windows.rs`
Expected: `560.0` / `420.0`。

- [ ] **Step 2: 实机（`pnpm tauri:dev` 后台；打字机 + 深色各一轮；按记忆「用户在本机时禁用键鼠自动化」——先问再接管键鼠，被拒则只做只读检查与用户口述）按 spec §6 十项逐条验证并截图**

1. 浮窗结构：DevTools 确认 `#launcherWindow.glass > .launcher-input-row(.launcher-ico + #launcherInput) + .launcher-list + .launcher-footer`；窗口宽 560、高 420；呼出时有 y −18 + 淡入 + scale .98 的入场；页脚两态文案正确（有结果 `共 N 项`、无结果 `↵ 回车搜索 · Esc 退出`），右侧动作提示随 Tab 变化。
2. 检索：输入应用名（含拼音 / 首字母）/ 笔记词 / 标签词 / 导图节点词 / 待办词 / 历史词，各命中对应分类与 `.li-cat`；范围开关逐项关闭后对应分类消失，重新打开后恢复（历史段应重新出现结果）；空查询只列应用 / 命令。
3. `=` 与「计算器」→ 首条「🧮 计算器 / 打开系统计算器」，回车打开系统计算器；输入普通查询（如「记事本」）不出现该条。
4. 无结果回车 → 默认浏览器搜索该词，浮窗收起。
5. 键盘：↑↓ 循环（首尾回绕）、Enter 执行、Tab 循环动作（页脚右侧「当前动作」随之变化）、`Alt+3` 直达第 3 条、`Ctrl+Enter` 以管理员执行、Esc 隐藏。
6. 9 条内置命令逐条执行正确：📚 历史归档 / ✅ 待办清单 / 📋 剪贴板历史 / 🧠 新建思维导图（会开窗）/ 📊 统计报表 / ⚙️ 偏好设置 各自主窗口对应页并收起浮窗；🧮 计算器开系统计算器；🔄 重建启动器索引出现索引重建日志；⏻ 退出 Inkling 退出（最后测）。
7. 呼出时面板被收起、主窗口保留：先 Alt+Space 呼出面板并保持打开，再按启动器快捷键 → 面板收起、主窗口仍在原处且可见。
8. 浏览器历史：Chrome 与 Edge 各访问若干页面，等待 30 秒首扫（或重启应用）后进库；设置页「已记录历史地址 N 条」更新；保留天数改小后计数减少（可用 `sqlite3` 直接改 `visited_at` 造超期行验证）；历史条目回车用默认浏览器打开该 URL。
9. 启动台页内嵌试用区与浮窗结果一致（同一次查询，分类顺序与条数相同）；页面不抢 Tab 焦点、Esc 不关任何窗口。
10. 回归：阶段三 / 四 A / 四 B 的面板与浮窗交互不受影响（面板呼出 / 收起、灵动岛轮播与悬停、置顶浮窗双击编辑、删除确认浮层）。

- [ ] **Step 3: 写验收记录并回填 spec §9**

验收记录按 `2026-09-12-prototype-realign-phase4b-acceptance.md` 的结构：环境 / 方法 → 自动化校验表 → 逐项比对表（打字机、深色各一列，十项）→ 发现的问题与补丁清单 → 未验证项。必须记录：
- 浮窗 560×420 下的观感（长路径截断、列表条数与滚动）；
- 各数据源命中情况（应用拼音 / 笔记标签 / 导图节点 / 待办 / 历史各自是否命中）；
- 历史采集（Chrome / Edge 各 Profile）与保留天数清理的实际结果（含首扫时间、导入条数）；
- 差异项：`enter()` 用了 `scale: 0.98`（spec §3 #11 记为「presets 无 scale 参数，只做位移 + 淡入」——实际 `SlideOptions` 支持 `scale`）、浮窗入场时长用 `--dur-base`（原型 .2s）、无结果回车后显式 `launcher_hide`。

spec §9 三条「待填」改为实际结论：各数据源命中情况、历史采集与保留天数清理、补丁清单（含 `OpenFlags::SQLITE_OPEN_READ_ONLY` 读副本是否成功、`calc.exe` 在 WebView2 宿主下是否正常拉起、删除 `launcher.css` 后各主题下浮窗底色可读性）。

- [ ] **Step 4: 提交**

```bash
git add docs/superpowers/plans/2026-09-13-prototype-realign-phase4c-acceptance.md docs/superpowers/specs/2026-09-13-prototype-realign-phase4c-launcher-design.md src/styles/extensions.css
git commit -m "docs(design): 阶段四C 实机验收记录"
```

---

## 自检记录

- **Spec 覆盖**：§2 D35 → Task 2；D36 → Task 3；D37 → Task 5；D38 → Task 3（`open_system_calculator`）+ Task 4（`mergeLauncherResults` 的 calc 段）+ Task 6（开关行）；D39 → Task 5；D40 → Task 4（动作 / Alt+N / Ctrl+Enter）+ Task 5（页脚右侧）。§3 差异表 #1 → Task 5（DOM + 删 `launcher.css`）；#2 → Task 4（`splitCommandName` 拆 emoji）+ Task 5；#3 → Task 4；#4 → Task 4（笔记段 + `mindmapAllText` / `mindmapRootText`）+ Task 4 `runActive`；#5 → Task 4（待办段 + `runActive`）；#6 → Task 3 + Task 4；#7 → Task 2；#8 → Task 3；#9 → Task 5（两态页脚 + `actionsHint`）；#10 → Task 4（`onKeydown` 无结果分支）；#11 → Task 5（`enter` y −18）；#12 → Task 5（`panel_hide`）；#13 → Task 5（常量）；#14 → Task 5（`onBlur`）；#15 → Task 6。§4.1 → Task 4；§4.2 → Task 5；§4.3 → Task 1；§4.4 → Task 2；§4.5 → Task 3；§4.6 → Task 5；§4.7 → Task 6。§5 测试 → Task 1（v8 + 保留天数往返）、Task 2（4 例）、Task 3（1 例）、Task 4（5 例）；§6 → Task 7；§7 七个批次 ↔ 七个 Task 一一对应；§8 风险「Chrome / Edge 独占 History」→ Task 2 `open_copy` + `run_once` 跳过本轮、「WebKit 换算写错」→ Task 2 两个已知值、「历史条目过多」→ Task 2 `WHERE last_visit_time > ?` + `visited_at` 索引 + `prune`、「`=` 前缀冲突」→ Task 4 只在 `=` 开头或「计算器 / calc」触发、「560 宽下长路径截断」→ Task 5 依赖生成层 `.li-name` / `small` 的 ellipsis。
- **类型与函数名一致性**：`Settings` 六键 `launcher_scope_apps / launcher_scope_notes / launcher_scope_todos / launcher_scope_calc / launcher_scope_history / launcher_history_retention_days` 在 Rust `models.rs`、`settings.rs` 键名、TS `domain.ts`、`useData.ts`、`LauncherPageView` 全程同名；`BrowserHistoryRow { url, title, visited_at }` 在 Rust（`dto!`，`url()` / `title()` / `visited_at()`）与 `tauri.ts` 逐字段一致；`browser_history_search(query, limit)` ↔ `api.browserHistory.search(query, limit = 20)`（invoke 参数名 `{ query, limit }`）、`browser_history_count` ↔ `api.browserHistory.count()`；`mergeLauncherResults(MergeInputs)` 的 `apps/notes/todos/history/query/scopes` 在 `utils/launcherResults.ts` 定义、`useLauncherResults` 传参同名；`LauncherResult { id, ico, name, sub?, cat, action }` 与 `LauncherAction` 五分支在 Task 4 定义、Task 5 / Task 6 模板同名取字段；`useLauncherResults` 返回的 `query / results / active / actions / actionIndex / reset / move / cycleAction / runActive / onKeydown` 在 Task 5（取 7 项）与 Task 6（取 6 项）按名解构；`windows::open_system_calculator()` 在 `launch.rs` 定义、`ipc.rs` 以 `use crate::services::launcher::launch::{open_system_calculator, launch, LaunchMode};` 引入；`prune_now(&app)` / `run_once(&app)` / `prune(&store, now, days)` / `search(&store, &query, limit)` / `count(&store)` 在 Task 2 内定义与调用（同 Task）；
  `builtin_commands` 的 9 个 id 在 Task 3 `scan.rs` 与 `ipc.rs` 分发臂逐字一致（`cmd:notes` / `cmd:todos` / `cmd:clips` / `cmd:mindmap` / `cmd:stats` / `cmd:settings` / `cmd:calc` / `cmd:rebuild` / `cmd:quit`）。
- **占位扫描**：无 TBD / 「类似于」/ 「适当处理」；Task 7 按要求只列步骤与记录要点，不含实现代码。
- **自检发现并就地修正**（与 spec / 任务书不符之处）：

  1. **任务书写「`launcher_launch` 已是 async」，实际是 `pub fn`（同步）**（`ipc.rs:694`）——Task 3 明确把它改成 `pub async fn` 并写明理由（`cmd:mindmap` 会建窗，同步命令与事件循环互等），否则 `cmd:mindmap` 会冻结应用。
  2. **spec §4.1 写「url → `api.openUrl`」，代码里是 `api.system.openUrl`**（`tauri.ts:172`）——Task 4 `runActive` 用实际路径。
  3. **spec §3 #11 写「`presets` 无 scale 参数，只做位移 + 淡入」，实际 `SlideOptions.scale` 存在**（`presets.ts:24,93`）——Task 5 传 `scale: 0.98` 还原原型（`enter` 非 slow 档用 `--ease-out`，正好对上原型的 `power2.out`）。
  4. **spec §4.2 的 `actionsHint` 只有静态键位提示，D40 又要求「保留现有的 Tab 动作栏」**——纯静态文案下按 Tab 没有任何可见反馈。Task 5 的 `actionsHint` 前置「当前动作：X」，动作段与 Ctrl+Enter 段仅在「当前项有管理员 / 定位动作」时出现（命令 / UWP 上提示了也执行不了，后端会报错）。
  5. **spec §4.2 模板把 `@keydown` 挂在 `#launcherInput` 上，点击条目后焦点离开输入框，键盘随即失效**——Task 5 沿用现有实现的做法把 `keydown` 挂 `window`（行为覆盖原型，能力不变）。
  6. **spec 未规定「保留天数改小后立即清理」的触发点**，只有 §4.4 的「每轮导入末尾 prune」与 §6 验收 8「保留天数改小后清理生效」——若只等 10 分钟一轮，验收与设置页计数都会「没反应」。Task 2 在 `ipc::settings_save` 末尾补 `prune_now(&app)`（一笔 DELETE，代价可忽略），Task 6 改保留天数后刷新计数。
  7. **spec §4.7 写「已记录历史地址 `N 条`（`browser_history_count`，挂载与导入后刷新）」，但导入完成后端没有任何事件可订阅**——Task 6 改为「挂载时读一次 + 保留天数变更后刷新」，并在验收记录里说明（`LauncherPageView` 在主窗口是 `v-if` 切换，每次进入页面都会重新挂载，等价于「进页面即刷新」）。
  8. **spec §4.5 让命令名自带 emoji、前端直接显示名称，但 `.li-ico` 是独立图标位**——照字面实现会出现「图标位空白 + 名称里再带一个 emoji」。Task 4 的 `splitCommandName()` 把首个 emoji 拆进 `.li-ico`（数据不改，仍是后端那份名称）。
  9. **spec §4.7 的设置行顺序把「计算器」夹在待办与浏览器历史之间（原型如此），而 §3 #15 的表述是「检索范围 ×5、历史保留天数、已记录历史地址」**——Task 6 按原型 DOM 顺序落（应用与命令 / 笔记 / 待办 / 计算器 / 浏览器历史 / 保留天数 / 已记录地址），与 §4.7 正文一致，共七行。
  10. **spec §4.4 未提导入行数上限**——`WHERE last_visit_time > ?` 已按保留窗口过滤，但异常库（如损坏的时间戳）仍可能一次命中全部行；Task 2 加 `LIMIT 20000` 兜底。
  11. **`.launcher-footer` 在生成层是单行块、无 flex**（`components.css:1570`），页脚右侧的动作提示落不到右边——Task 5 在 `extensions.css` 追加 §11（两条声明，仅作用于 `:root[data-window='launcher']`，不影响启动台页的同一类名）。
  12. **`launcher.css` 删除后，`data-window='launcher'` 的透明底与 `#launcherWindow` 的浮层定位（`fixed + top:16vh + translateX(-50%) + min(92vw,560px)`，`components.css:1486`）没人接管**——Task 5 在 `window-fit.css` 补启动台窗口块（透明根 + 铺满 + `clip-path` 圆角裁剪 + 列表 `flex: 1` 撑满）。
  13. **`.glass` 用 `--glass-bg`，而四 B 的灵动岛曾因底色可读性改用 `--menu-bg`**（`extensions.css:727-746`）——核实：`themes.css` 里 30 套主题**都**覆盖了 `--glass-bg`（`grep -c` = 30），可读性有保障，故浮窗照原型用 `.glass`，不额外覆盖底色（验收第 1 项复核）。
  14. **（控制者订正 2026-09-13，Task 4 实施后）本计划 Task 4 的笔记段示例代码与同一任务的用例 5 自相矛盾**：示例把「正文 / 标签 / 导图」三种命中合成一个集合再 `slice(0, 8)`，而用例 5 断言标签命中（`note:tag`）与导图命中（`note:map`）必须出现——9 条正文命中会把它们整段挤掉（实测 1 例失败）。已按用例改为**按命中原因分档限流**（正文 → 标签 → 导图，各 ≤8，合计 ≤24），实现见 `513fd53`，口径已回填 spec §4.1。同时订正两处过时措辞：① spec §4.1 笔记段的「取前 8 条」；② spec §3 #11 的「`presets` 无 scale 参数」（本节第 3 条已确认存在，此处同步正文）。另外把笔记副文案的「否则显示时间」落实为 `formatStamp(updated_at)`（原型 `n.time` 同格式），不再是空串。
  15. **rusqlite 只读打开外部库的 `OpenFlags` 在仓库里此前没有先例**（`grep OpenFlags src-tauri/src` 无命中）——Task 2 首次引入 `Connection::open_with_flags(…, OpenFlags::SQLITE_OPEN_READ_ONLY)`，并在验收记录里确认能打开浏览器副本。
  16. **`Keyword::generate` 对含 emoji 的名称是否仍产出拼音关键字，spec 未讨论**——Task 3 的单测显式断言 9 条命令的 `keywords` 非空（含「📚 历史归档」这类混合串），避免 emoji 名称让命令在拼音 / 首字母检索里漏掉。
  17. **`useLauncherSearch.ts` 的删除时机**：Task 4 引入 `useLauncherResults` 时它仍被 `LauncherPageView` 使用，若同期删除会留下一个编译不过的中间态——放在 Task 6（最后一个调用方改完）删除，保证每个提交都是绿的。
  18. **（控制者订正 2026-09-13，实机验收后）两处 doc/实现不一致随 §9 记录一并订正**：① spec §4.2 的浮窗页脚示例写成**页面**版文案「点击直接执行」（浮窗原型是「Esc 关闭」，`docs/app.js:3300`）、模板里是 `@mousemove`（实现与原型用 `mouseenter`）、右栏写的是带「当前动作：」前缀的旧形态（受 560 宽限制改为动作芯片）；② 本节第 4 条说的「`actionsHint` 前置『当前动作：X』」同样已被芯片方案取代。

