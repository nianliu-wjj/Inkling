# 灵动岛 + 启动器搜索 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 按 `2026-09-08-dynamic-island-design.md` 与 `2026-09-08-launcher-search-design.md` 落地两项功能：主屏顶部胶囊灵动岛（轮播当日待办、悬停详情、点击唤出面板到待办页、可配尺寸/透明/穿透、插件注册表），以及全局快捷键呼出的本地启动器搜索（程序/UWP/文件/文件夹索引，拼音/首字母/纠错匹配，历史加权，三层缓存）。

**Architecture:** 灵动岛复用感应区的窗口/轮询/记账基础设施：新增 `island` 窗口与 `AppState.island_rect`，watcher 每轮附带判定灵动岛悬停与左键边沿；前端独立窗口 `island.html` + 编译期插件注册表。启动器全部索引与匹配在 Rust `services/launcher/` 完成（纯函数模块带单测），前端 `launcher.html` 只做输入与结果展示。

**Tech Stack:** Rust（Tauri 2、walkdir、rayon、pinyin、windows-sys `Win32_UI_Input_KeyboardAndMouse`、serde_json）、Vue 3 + TypeScript、既有 CSS 令牌。

## Global Constraints

- 回答与提交信息用中文；提交格式 `<type>(<scope>): <中文描述>`；每完成一个 Task 提交一次。
- Rust 文件保持既有注释密度；关键节点 `eprintln!("[island] …")` / `eprintln!("[launcher] …")`；前端 `logger.*`，禁止裸 `console.*`。
- Rust 提交前 `cargo fmt` + `cargo test` + `cargo clippy` 无告警；前端 `npx vue-tsc --noEmit` + `npx prettier --check`。
- 设置项范围（clamp 在 Rust）：`island_width` 200–800、`island_height` 28–72、`island_opacity` 0.3–1.0、`island_cycle_seconds` 2–30。
- 所有窗口坐标一律**物理像素**（`WorkArea` 带 scale，见 windows.rs），新窗口 label 必须加进 `capabilities/default.json`。
- 启动器索引上限 200,000 条；快照与历史存 `app_data_dir/launcher/`；不出网。

---

## A 阶段：灵动岛

### Task A1：设置字段与事件常量

**Files:** Modify `src-tauri/src/domain/models.rs`（Settings 加 7 字段 + 默认值函数）、`src-tauri/src/data/settings.rs`（读取与保存各 7 行）、`src-tauri/src/events.rs`（`ISLAND_HOVER / ISLAND_CLICK / PANEL_NAVIGATE`）、`src/typings/domain.ts`（Settings 接口）、`src/service/events.ts`（三个事件名）。

- [ ] 字段：`island_enabled: bool=true`、`island_width: i64=360`、`island_height: i64=36`、`island_opacity: f64=0.85`、`island_click_through: bool=false`、`island_cycle_seconds: i64=4`、`island_plugins: String="today-todos"`，均 `#[serde(default = …)]`。
- [ ] `settings.rs`：读取用 `flag / parse().unwrap_or(default)`，保存加 7 个 `("island_*", …)`。
- [ ] 提交：`feat(island): 灵动岛设置字段与事件常量`。

### Task A2：Rust 窗口与轮询

**Files:** Modify `src-tauri/src/app/windows.rs`、`src-tauri/src/app/state.rs`（`island_rect: Mutex<Option<(f64,f64,f64,f64)>>`）、`src-tauri/src/services/hotzone_watcher.rs`、`src-tauri/src/ipc.rs`、`src-tauri/src/main.rs`、`src-tauri/Cargo.toml`（windows-sys 加 `Win32_UI_Input_KeyboardAndMouse`）、`src-tauri/capabilities/default.json`（加 `island`）。

- [ ] `pub fn island_clamp(settings) -> (w,h,opacity,cycle)`（纯函数，单测四个边界）。
- [ ] `fn island_geometry(work: &WorkArea, width: f64, height: f64) -> (x,y,w,h)`：x 居中，y = `work.top + (HOTZONE_HEIGHT + 4) * scale`；单测 1.0 / 1.5。
- [ ] `pub fn create_island(app)`：主屏（`primary_monitor` 回退 `cursor_monitor`）；`island.html`；属性见 spec D10；`set_ignore_cursor_events(click_through)`；物理落位；`AppState.set_island_rect`；`island_enabled=false` 时不建。
- [ ] `pub fn island_apply(app)`：设置变更后重算尺寸/位置/穿透；`enabled` 由 false→true 建窗、true→false 关窗。
- [ ] `pub fn island_expand(app, expanded: bool)`：高度切换为 `max(height,120)` / `height`，顶部位置不变，更新记账矩形。
- [ ] watcher：每轮读 `island_rect`；悬停翻转 → `emit_to("island", ISLAND_HOVER, bool)`；左键边沿（`GetAsyncKeyState(VK_LBUTTON) & 0x8000`）且在矩形内 → `windows::panel_show` + `emit_all(PANEL_NAVIGATE, "todo")` + `emit_to("island", ISLAND_CLICK)`。面板可见时不判定。
- [ ] ipc：`island_expand(expanded)`；`settings_save` 末尾调用 `windows::island_apply`。
- [ ] 提交：`feat(island): 灵动岛窗口、几何与穿透模式下的悬停/点击探测`。

### Task A3：前端窗口、插件注册表与默认插件

**Files:** Create `island.html`、`src/windows/island.ts`、`src/windows/Island/IslandApp.vue`、`src/island-plugins/index.ts`、`src/island-plugins/today-todos.ts`、`src/island-plugins/TodoDetail.vue`、`src/styles/island.css`；Modify `vite.config.ts`（input 加 island）、`src/service/tauri.ts`（`api.island.expand`）、`src/windows/Panel/PanelApp.vue`（订阅 `PANEL_NAVIGATE` 切页）。

- [ ] 注册表接口见 spec D8；`resolveIslandPlugins(setting)` 兜底全部内置。
- [ ] `today-todos.useItems()`：`useTodos()` → 筛选顶级、未完成、当天或逾期 → 按 due 升序 → `IslandItem{ dot=优先级色, title=content, meta=HH:mm, detail=TodoDetail }`。
- [ ] `IslandApp.vue`：`applyCachedTheme/Glass`；`useSettings` 取 opacity/cycle/plugins；轮播索引 + `setInterval`（悬停暂停）；`expanded` 由 DOM `mouseenter/leave`（非穿透）或 `ISLAND_HOVER`（穿透）驱动，变化时 `api.island.expand`；点击（DOM）→ `api.windows.panelShow()` + `emit` 走 Rust；空态文案。
- [ ] `PanelApp.vue`：`onAppEvent(AppEvents.panelNavigate, id => activeId = id)`。
- [ ] 样式：胶囊 `border-radius: 999px`、背景 `rgba(var(--wsa-inverse?), var(--island-alpha))`——用 `var(--menu-bg)` 叠加 `opacity` 变量；文字滚动 `translateY` 240ms。
- [ ] 提交：`feat(island): 灵动岛窗口前端、插件注册表与当日待办插件`。

### Task A4：设置页分区与验收

- [ ] `SettingsView.vue` 新增「灵动岛」分区：启用开关、宽/高数字输入（min/max）、透明度滑块、穿透开关、轮播秒数、插件多选排序（复用面板插件的列表控件）。
- [ ] 实机：默认显示、轮播、悬停展开、点击唤出并切到待办页、穿透模式悬停/点击、设置即时生效；`vue-tsc` / `prettier` / `cargo test`。
- [ ] 提交：`feat(island): 设置页灵动岛分区`。

## B 阶段：启动器搜索

### Task B1：关键字生成与打分纯函数（TDD）

**Files:** Create `src-tauri/src/services/launcher/mod.rs`、`keyword.rs`、`score.rs`；Modify `Cargo.toml`（`pinyin = "0.10"`, `rayon = "1"`, `walkdir = "2"`）。

- [ ] `keyword::generate(name: &str) -> Vec<String>`：小写 → 去版本号（正则 `\s*[\(（]?\d+(\.\d+)*[\)）]?\s*(bit|位|x64|x86)?` 与 `\s+20\d\d`）→ 空格归一 → 去符号；拼音（`pinyin` crate Plain，连续汉字空格分隔）→ 首字母（拼音输出按空格取首字母）；大写缩写（纯 ASCII）；去重、去空。测试：`微信`→含 `weixin`、`wx`；`Visual Studio Code`→含 `visualstudiocode`、`vsc`；`Notepad++ (64-bit)`→含 `notepad`。
- [ ] `score::score_keyword(keyword, input) -> Option<f64>`（容错门槛不满足返回 None）与 `score::best(keywords, input, bias)`；子函数 `edit_distance_suffix / adjust_log2 / subset / kmp`。测试：`wx` 对 `wx` 高于对 `weixin`；`weixn` 能命中 `weixin`；`ab` 对 `abc`（≤2 严格）不容错。
- [ ] 提交：`feat(launcher): 关键字生成管道与标准打分引擎（纯函数 + 单测）`。

### Task B2：历史加权与快照

**Files:** Create `history.rs`、`snapshot.rs`。

- [ ] `History { counts, daily(7 天滚动), last_launch, affinity }`，`record(path, query)`，`boost(path, base_score, now) -> f64` = `(base/15).clamp(0,1) × (0.8·ln1p(count) + 1.5·recent + 0.5·temporal)` + `3.0·affinity(query,path)`；JSON 落盘 `launcher/history.json`。测试：抑制因子、7 天滚动。
- [ ] `snapshot::save/load(Vec<Candidate>)` 原子写；损坏返回 None。
- [ ] 提交：`feat(launcher): 启动历史加权与索引快照`。

### Task B3：数据源扫描与索引服务

**Files:** Create `scan.rs`、`search.rs`、`launch.rs`；Modify `mod.rs`（`LauncherState`、`start(app)`）、`services/mod.rs`、`main.rs`（`services::launcher::start`）。

- [ ] `scan::programs()`：`%ProgramData%\Microsoft\Windows\Start Menu`、`%AppData%\Microsoft\Windows\Start Menu`、用户桌面；walkdir depth 5；`exe/lnk/url`；排除关键字；名字取文件名去扩展名。
- [ ] `scan::uwp()`：`powershell -NoProfile -NonInteractive -Command "Get-StartApps | ConvertTo-Json -Compress"`，10s 超时；`Kind::Uwp`，path=`shell:AppsFolder\{AppID}`。
- [ ] `scan::files(roots)`：按 `LauncherRoot{path,depth,excludes}`；文件与文件夹都入；总上限 200k。
- [ ] `scan::commands()`：打开设置 / 重建索引 / 退出 Inkling。
- [ ] `LauncherState`：`RwLock<Arc<Index>>`（候选 + 代际）、`Mutex<History>`、L1 `LruCache<String, Vec<Hit>>`(容量 64，代际变化清空)。`start`：加载快照 → 立即可搜 → 后台重扫替换 → 每 30 分钟重扫。
- [ ] `search::search(state, query, top_k=9) -> Vec<Hit>`：空查询按历史分排序；否则 rayon `par_iter` 打分 + 历史加权 → `select_nth_unstable_by` → 排序前 K → 高亮区间（匹配关键字在 name 中的前缀/子串位置，找不到则不高亮）。
- [ ] `launch::open / open_admin / reveal / copy_path`（explorer 代理、`Start-Process -Verb RunAs`）。
- [ ] ipc：`launcher_search(query)`、`launcher_launch(id, mode)`、`launcher_rebuild`、`launcher_status`、`launcher_show/hide`；capabilities 加 `launcher`。
- [ ] 提交：`feat(launcher): 数据源扫描、索引服务与搜索/启动命令`。

### Task B4：窗口与快捷键

**Files:** Modify `app/windows.rs`（`launcher_show/hide`：光标所在屏居中偏上，640×（56 + 9×44）逻辑，物理落位；失焦隐藏）、`app/shortcut.rs`（第二个全局快捷键 `launcher_shortcut`，默认 `Alt+Space`，与面板快捷键冲突拒绝）、`domain/models.rs` + `data/settings.rs`（`launcher_shortcut`、`launcher_roots`）、`ipc.rs`（`rebind_launcher_shortcut`）。

- [ ] 提交：`feat(launcher): 搜索窗口与全局快捷键`。

### Task B5：前端搜索窗口

**Files:** Create `launcher.html`、`src/windows/launcher.ts`、`src/windows/Launcher/LauncherApp.vue`、`src/styles/launcher.css`；Modify `vite.config.ts`、`src/service/tauri.ts`（`api.launcher.*`）。

- [ ] 输入去抖 30ms → `launcher_search`；结果列表（类型徽章 + 高亮名 + 路径灰字）；↑/↓、Enter、Ctrl+Enter、Tab 切动作（打开所在文件夹 / 复制路径）、Esc 隐藏、`Alt+1..9` 直达；失焦隐藏；空查询显示常用。
- [ ] 提交：`feat(launcher): 搜索窗口前端`。

### Task B6：设置页与验收

- [ ] `SettingsView.vue` 「启动器」分区：快捷键改键、根目录列表（增删、深度、排除关键字）、立即重建、索引状态。
- [ ] 实机：呼出/隐藏、拼音/首字母/纠错命中、Ctrl+Enter、Tab 动作、重建、空查询常用项；`cargo test`、`vue-tsc`、`prettier`。
- [ ] 提交：`feat(launcher): 设置页启动器分区`。

## C 阶段：文档

- [ ] `docs/features/dynamic-island.md`、`docs/features/launcher.md`：功能说明、设置项、快捷键、算法原理（含公式与与参考项目的对应关系）、数据存放位置与隐私说明、已知限制与后续子项目（全盘索引 / 图标 / 窗口唤醒）。
- [ ] 提交：`docs: 灵动岛与启动器功能说明`。
