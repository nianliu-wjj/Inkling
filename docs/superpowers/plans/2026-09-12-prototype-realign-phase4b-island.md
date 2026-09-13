# 原型对齐 · 阶段四 B「灵动岛 / 感应区 / 提醒卡片 / 置顶浮窗」实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把四个独立小窗口对齐原型 `docs/`：灵动岛改为原型 DOM（`#dynamicIsland.glass > .di-ticker > .di-track > .di-item`）与逐条停留轮播，数据口径含子任务与播放范围（`pickIslandTodos`），新增播放范围 / 悬停穿透 / 流光 / 全屏自动隐藏四个设置项与右下角手柄拖拽，提醒岛内呈现、面板展开淡出；感应区指示器改用主题令牌；提醒卡片横向入退场；置顶浮窗双击编辑回写；修复 `settings_save` 同步建窗死锁（D34）；落实 4A 遗留的主窗口删除确认兜底（D29）。

**Architecture:** 后端只加「能力」：四个 KV 设置键 + v7 版本号、`island_resize` 命令、`AppState` 缓存的灵动岛旗标（悬停穿透 / 全屏隐藏）、`platform::foreground_is_fullscreen()`，探测与隐藏都塞进既有 `hotzone_watcher` 轮询线程；前端一个纯函数 `pickIslandTodos()` 同时服务灵动岛胶囊与主窗口灵动岛页预览，`IslandApp.vue` 就地重写为原型结构并用 `setTimeout` 链 + `translateY` 做逐条停留轮播；所有样式差异只落在 `extensions.css`（删 §8、§6 指示器令牌化、新增 §10）与 `window-fit.css`（灵动岛窗铺满 + 圆角裁剪）。

**Tech Stack:** Tauri 2 / Rust（`windows-sys 0.59` 加 `Win32_UI_Shell`）· Vue 3.5 `<script setup lang="ts">` · TypeScript 5.9 · Vite 7 · animejs 4.5（`enter` / `exit` 预设）· Node 24（`node:test`）· rusqlite

**设计文档：** `docs/superpowers/specs/2026-09-12-prototype-realign-phase4b-island-design.md`（下文简称 spec；决策 §2 D29–D34、差异表 §3 #1–#25、设计 §4、测试 §5、验收 §6、提交批次 §7、风险 §8）

## Global Constraints

- 唯一参考原型 `docs/index.html` + `docs/styles.css` + `docs/app.js`；生成层样式（`tokens.css` / `base.css` / `components.css` / `themes.css`）**不改**，`pnpm sync:styles --check` 必须始终通过；项目自有样式只进 `src/styles/extensions.css`（本阶段：**删除 §8 整节**、§6 `.hotzone-indicator` 底色 / 边框改令牌、§3 追加 `#cardConfirm.above .cc-caret`、**新增 §10「灵动岛与置顶扩展」**）与 `src/styles/window-fit.css`（灵动岛窗铺满与 `clip-path: inset(0 round 999px)`）。`src/styles/island.css` 整文件删除。
- 组件模板只用原型类名（`#dynamicIsland` / `.di-ticker` / `.di-track` / `.di-item` / `.di-dot.{high|medium|low|idle}` / `.di-text(.dim)` / `.di-date` / `.di-time(.ovd)` / `.di-alert` / `.di-alert-ico` / `.di-alert-text` / `.di-resizer` / `.di-fade` / `.di-pass-click` / `.di-glow` / `#reminderCard` / `#pinnedWindow` / `.hotzone-indicator`），展开详情沿用现有 `.island-expanded` / `.island-detail*` / `.island-hint`（项目扩展，迁入 §10）；颜色只用令牌。
- 已拍定决策（spec §2）：D29 主窗口删除确认也走卡片下方兜底，`anchorBeside` 补 `above`；D30 全屏探测用 `SHQueryUserNotificationState`；D31 置顶浮窗双击编辑态 + Ctrl+Enter / 失焦保存；D32 参数范围 200–480 / 32–56 / 1–10、默认 320×36·3 秒、存量不迁移；D33 悬停穿透只加守卫不改悬停语义；D34 `settings_save` 改 `pub async fn`。
- 编码规范：Vue 一律 `<script setup lang="ts">`，禁止 `any`；关键节点走 `src/service/logger.ts`（Rust 侧 `eprintln!`，前缀 `[island]` / `[settings]` / `[platform]` / `[data]`）；每个文件头部有职责说明注释；Rust 新增 `pub` 项一律带 `///` 文档注释。
- **所有可能建窗或做 show / hide 序列的 Tauri 命令一律 `pub async fn`**：本阶段 `settings_save` 改 async（内部 `island_apply` 可能 `create_island`）；`island_resize` 只做 `set_size / set_position`，不建窗，保持同步。
- 动效硬约束沿用阶段一：只动 transform / opacity；`.glass` 静止态无 transform（`.di-track` 的 `translateY` 作用在胶囊**内部**的轨道元素上，不违反）；入退场由 `src/motion` 预设驱动。
- 验证：每任务结束 `pnpm typecheck` 零错误；改 Rust 的任务 `cargo fmt --check && cargo check && cargo test`（均在 `src-tauri/` 下或加 `--manifest-path src-tauri/Cargo.toml`）；改纯函数（Task 3 / Task 8）`pnpm test:unit`；改样式（Task 4 / 6 / 8）`pnpm sync:styles --check`；`pnpm format:check` 每任务；最后 `pnpm exec vite build`。
- 提交：中文提交信息，按 spec §7 的 9 个批次，每任务一次提交（Task 9 为验收记录提交）。

---

## File Structure

```
src-tauri/
├─ Cargo.toml                          windows-sys features 加 Win32_UI_Shell
└─ src/
   ├─ domain/models.rs                 Settings 增 island_scope / island_pass_hover / island_glow / island_auto_hide；默认宽 320、停留 3
   ├─ data/settings.rs                 get / save 四键
   ├─ data/mod.rs                      migrate 链加 v7（with_v7 留空）+ 单测
   ├─ app/state.rs                     island_flags 缓存（pass_hover / auto_hide）
   ├─ app/windows.rs                   island_clamp 新范围；island_resize()；PINNED_SIZE 宽 230→220
   ├─ app/ipc.rs                       settings_save 改 async 并刷新缓存；新增 island_resize 命令
   ├─ main.rs                          注册 ipc::island_resize
   ├─ platform.rs                      foreground_is_fullscreen()
   └─ services/hotzone_watcher.rs      悬停穿透守卫；每 12 轮全屏探测翻转 hide / show
src/
├─ typings/domain.ts                   IslandScope 类型；Settings 四键；范围注释
├─ composables/useData.ts              DEFAULT_SETTINGS 四键与新默认
├─ service/tauri.ts                    api.island.resize
├─ utils/
│  ├─ island.ts                        新：pickIslandTodos() 纯函数（IslandTodoRow）
│  ├─ island.test.ts                   新：四个用例
│  ├─ anchor.ts                        placement 增 'above'
│  └─ anchor.test.ts                   above 一例
├─ island-plugins/
│  ├─ index.ts                         IslandItem 增 date / overdue / priority；emptyText 改原型文案
│  └─ today-todos.ts                   删 pickTodayTodos；useTodayTodos 改用 pickIslandTodos + island_scope
├─ windows/
│  ├─ island.ts                        删 island.css 引入
│  ├─ Island/IslandApp.vue             整体重写（原型 DOM / 逐条停留轮播 / 提醒 / 淡出 / 手柄 / 守卫）
│  ├─ Main/IslandPageView.vue          pickIslandTodos 预览；ISLAND_LIMITS；补四行；状态行
│  ├─ Reminder/ReminderApp.vue         enter / exit 横向 60px
│  ├─ Pinned/PinnedApp.vue             双击编辑回写 + ToastHost
│  └─ Main/{NotesView,ClipsView,DayView,TodosView}.vue   删 fallback="center" / confirm-fallback="center"
├─ components/base/CardConfirm.vue     .above 类
├─ components/card/TodoTree.vue        confirmFallback 注释
└─ styles/
   ├─ island.css                       删除
   ├─ extensions.css                   §3 +.above 箭头；§6 指示器令牌；删 §8；新增 §10
   └─ window-fit.css                   灵动岛窗铺满 + clip-path
docs/superpowers/plans/2026-09-12-prototype-realign-phase4b-acceptance.md   验收记录（Task 9）
```

---

## Task 1: 四个新设置项、v7 迁移、参数范围对齐原型、`settings_save` 改 async

**Files:**
- Modify: `src-tauri/src/domain/models.rs:106-124,186-207,230-249`
- Modify: `src-tauri/src/data/settings.rs:117,183`
- Modify: `src-tauri/src/data/mod.rs:85,189-196,316-319,545-548`
- Modify: `src-tauri/src/app/windows.rs:529-533,554,1217-1236`
- Modify: `src-tauri/src/ipc.rs:601-624`
- Modify: `src/typings/domain.ts:17,122-135`
- Modify: `src/composables/useData.ts:37-43`

**Interfaces:**
- Produces（Rust）：`Settings` 新增字段 `island_scope: String`（`"today"` / `"all"`，默认 `"today"`）、`island_pass_hover: bool`（默认 false）、`island_glow: bool`（默认 false）、`island_auto_hide: bool`（默认 true），`dto!` 自动生成 `island_scope()` / `set_island_scope()` 等访问器；`default_island_width()` 返回 320、`default_island_cycle_seconds()` 返回 3；`island_clamp` 范围宽 200–480 / 高 32–56 / 停留 1–10；`Store::with_v7()`；`ipc::settings_save` 签名改 `pub async fn settings_save(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<(), String>`（Tauri async 命令带借用参数须返回 `Result`，已满足）。
- Produces（TS）：`export type IslandScope = 'today' | 'all'`；`Settings` 增 `island_scope: IslandScope`、`island_pass_hover: boolean`、`island_glow: boolean`、`island_auto_hide: boolean`。

- [x] **Step 1: 先写失败的 v7 迁移单测**

`src-tauri/src/data/mod.rs` `mod tests` 末尾（`v6_migration_adds_source_app_and_keeps_old_rows_null` 之后、模块收尾 `}` 之前）追加：

```rust
    /// v7 只推进版本号（四个灵动岛新键靠 Settings::default 兜底）：版本 6 → 7，再跑一次仍是 7，且不凭空插入设置行。
    #[test]
    fn v7_migration_bumps_version_and_is_idempotent() {
        let db = settings_db(None);
        db.pragma_update(None, "user_version", 6).unwrap();
        let mut store = Store {
            db,
            data_dir: std::path::PathBuf::from("."),
        };
        store.migrate().unwrap();
        let version: i64 = store
            .db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 7);
        store.migrate().unwrap();
        let again: i64 = store
            .db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(again, 7);
        let rows: i64 = store
            .db
            .query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }
```

- [x] **Step 2: 运行，确认失败**

Run: `cd src-tauri && cargo test v7_migration 2>&1 | grep -E "assert|panicked|test result" | head`
Expected: `assertion `left == right` failed`（left 6 right 7）——`migrate()` 尚无 v7 分支。

- [x] **Step 3: `data/mod.rs` 加 v7 链**

第 85 行注释改为：

```rust
    /// 版本化迁移：v0（初版）→ v1（archived_at / 附件路径 / 统计事件 / 提醒实例 / 提醒抑制标记）→ v6（clipboard_entries.source_app）→ v7（灵动岛四个新设置键，仅推进版本号）。
```

第 189–196 行 `if version < 6 {…}` 之后、`Ok(())` 之前插入：

```rust
        if version < 7 {
            self.with_v7()
                .map_err(|e| format!("数据库迁移到 v7 失败: {e}"))?;
            self.db
                .pragma_update(None, "user_version", 7)
                .map_err(db_err)?;
        }
```

第 316–319 行 `with_v6` 之后插入：

```rust
    /// v7 增量：灵动岛播放范围 / 悬停穿透 / 流光 / 全屏隐藏四个设置键（spec 4B §4.2）。
    ///
    /// settings 是 KV 表，缺键由 `Settings::default()` 兜底，这里不写任何行——
    /// 与 v5 同一取舍：迁移不凭空插入设置行。保留函数只为让链完整、版本号可追溯。
    fn with_v7(&self) -> Result<(), String> {
        eprintln!("[data] v7 迁移：灵动岛新设置键靠默认值兜底，无需改表");
        Ok(())
    }
```

Run: `cd src-tauri && cargo test v7_migration 2>&1 | grep "test result"`
Expected: `test result: ok. 1 passed`。

- [x] **Step 4: `domain/models.rs` 四字段与新默认**

第 106–109 行 `default_island_width` 替换为：

```rust
/// 灵动岛默认宽度（逻辑像素）；原型 DI.w = 320（spec 4B D32）。
fn default_island_width() -> i64 {
    320
}
```

第 121–124 行 `default_island_cycle_seconds` 替换为：

```rust
/// 灵动岛默认每条停留秒数；原型 DI.stay = 3（spec 4B D32）。
fn default_island_cycle_seconds() -> i64 {
    3
}
```

紧接其后（`default_island_plugins` 之前）插入：

```rust
/// 灵动岛默认播放范围：仅当天待办（原型 DI.scope = 'today'）。
fn default_island_scope() -> String {
    "today".into()
}
```

`Settings` 结构体内第 189 行注释 `/// 灵动岛宽度（逻辑像素，200–800）。` 改为 `/// 灵动岛宽度（逻辑像素，200–480）。`；第 192 行 `28–72` 改为 `32–56`；第 201 行 `/// 灵动岛轮播间隔秒数（2–30）。` 改为 `/// 灵动岛每条停留秒数（1–10）。`。第 204–206 行 `island_plugins` 字段之后插入：

```rust
        /// 灵动岛播放范围：today（仅当天待办）/ all（全部未完成待办）。
        #[serde(default = "default_island_scope")]
        island_scope: String,
        /// 灵动岛悬停穿透：为真时悬停不撑高详情、不暂停轮播（后端 watcher 不发 ISLAND_HOVER）。
        #[serde(default)]
        island_pass_hover: bool,
        /// 灵动岛流光边框（原型 .di-glow）。
        #[serde(default)]
        island_glow: bool,
        /// 前台应用全屏时自动隐藏灵动岛（D30：SHQueryUserNotificationState）。
        #[serde(default = "default_true")]
        island_auto_hide: bool,
```

`impl Default for Settings` 里第 248 行 `island_plugins: default_island_plugins(),` 之后插入：

```rust
            island_scope: default_island_scope(),
            island_pass_hover: false,
            island_glow: false,
            island_auto_hide: true,
```

- [x] **Step 5: `data/settings.rs` 读写四键**

第 117 行 `.island_plugins(pick(&values, "island_plugins", defaults.island_plugins()))` 之后插入：

```rust
            .island_scope(pick(&values, "island_scope", defaults.island_scope()))
            .island_pass_hover(flag(
                &values,
                "island_pass_hover",
                *defaults.island_pass_hover(),
            ))
            .island_glow(flag(&values, "island_glow", *defaults.island_glow()))
            .island_auto_hide(flag(
                &values,
                "island_auto_hide",
                *defaults.island_auto_hide(),
            ))
```

第 183 行 `("island_plugins", settings.island_plugins().clone()),` 之后插入：

```rust
            ("island_scope", settings.island_scope().clone()),
            ("island_pass_hover", settings.island_pass_hover().to_string()),
            ("island_glow", settings.island_glow().to_string()),
            ("island_auto_hide", settings.island_auto_hide().to_string()),
```

第 7 行注释 `17 个字段里大半都是这个形状` 改为 `三十余个字段里大半都是这个形状`。

- [x] **Step 6: `app/windows.rs` 钳制范围与单测断言**

第 529–532 行四个常量替换为：

```rust
/// 灵动岛尺寸范围（逻辑像素）：原型 DI.W_MIN/W_MAX/H_MIN/H_MAX（spec 4B D32）。
const ISLAND_MIN_WIDTH: f64 = 200.0;
const ISLAND_MAX_WIDTH: f64 = 480.0;
const ISLAND_MIN_HEIGHT: f64 = 32.0;
const ISLAND_MAX_HEIGHT: f64 = 56.0;
/// 每条停留秒数范围：原型灵动岛页 min=1 max=10。
const ISLAND_MIN_CYCLE: i64 = 1;
const ISLAND_MAX_CYCLE: i64 = 10;
```

第 554 行 `let c = cycle_seconds.clamp(2, 30);` 改为 `let c = cycle_seconds.clamp(ISLAND_MIN_CYCLE, ISLAND_MAX_CYCLE);`。

单测 `island_clamp_bounds`（第 1217–1236 行）整体替换为：

```rust
    /// 灵动岛参数钳制（D32 范围）：四个维度越界都按边界生效，非法透明度回默认。
    #[test]
    fn island_clamp_bounds() {
        let p = island_clamp(100, 10, 2.0, 0);
        assert_eq!(
            (p.width, p.height, p.opacity, p.cycle_seconds),
            (200.0, 32.0, 1.0, 1)
        );
        let p = island_clamp(9000, 999, 0.1, 999);
        assert_eq!(
            (p.width, p.height, p.opacity, p.cycle_seconds),
            (480.0, 56.0, 0.3, 10)
        );
        let p = island_clamp(320, 36, f64::NAN, 3);
        assert_eq!(p.opacity, 0.85);
        let p = island_clamp(320, 36, 0.85, 3);
        assert_eq!(
            (p.width, p.height, p.opacity, p.cycle_seconds),
            (320.0, 36.0, 0.85, 3)
        );
    }
```

- [x] **Step 7: `ipc.rs` `settings_save` 改 async**

第 601–624 行替换为：

```rust
/// 保存偏好设置并即时应用（感应区位置 / 面板位置 / 灵动岛）。
///
/// 必须是 **async** 命令（D34，与 `pin_create` / `launcher_show` 同款）：灵动岛「启用」关→开时
/// `island_apply` 会走 `create_island` 建窗，同步命令跑在主线程上与事件循环互等，整个应用冻结。
#[tauri::command]
pub async fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    state.lock_store()?.save_settings(&settings)?;
    if *settings.start_on_boot() {
        let _ = app.autolaunch().enable();
    } else {
        let _ = app.autolaunch().disable();
    }
    emit_all(&app, events::SETTINGS_CHANGED, settings);
    // 唤出方向变更：感应区与面板一起移到新的屏幕边缘（面板已打开时立即生效）。
    if let Err(error) = windows::reposition_hotzone(&app) {
        eprintln!("[settings] 移动感应区失败: {error}");
    }
    let _ = windows::reposition_panel(&app);
    // 灵动岛：显隐 / 尺寸 / 穿透随设置即时生效。
    if let Err(error) = windows::island_apply(&app) {
        eprintln!("[settings] 应用灵动岛设置失败: {error}");
    }
    Ok(())
}
```

- [x] **Step 8: 前端类型与默认值**

`src/typings/domain.ts` 第 17 行 `GlassLevel` 之后加：

```ts
/** 灵动岛播放范围（domain/models.rs::Settings 注释「today / all」）。 */
export type IslandScope = 'today' | 'all'
```

第 122–133 行注释：`/** 灵动岛宽度（逻辑像素，200–800）。 */` 改 `200–480`；`28–72` 改 `32–56`；`/** 灵动岛轮播间隔秒数（2–30）。 */` 改为 `/** 灵动岛每条停留秒数（1–10）。 */`。第 135 行 `island_plugins: string` 之后加：

```ts
  /** 灵动岛播放范围：仅当天 / 全部未完成。 */
  island_scope: IslandScope
  /** 灵动岛悬停穿透：悬停不撑高详情、不暂停轮播。 */
  island_pass_hover: boolean
  /** 灵动岛流光边框。 */
  island_glow: boolean
  /** 前台应用全屏时自动隐藏灵动岛。 */
  island_auto_hide: boolean
```

`src/composables/useData.ts` 第 38 行 `island_width: 360,` 改 `island_width: 320,`；第 42 行 `island_cycle_seconds: 4,` 改 `island_cycle_seconds: 3,`；第 43 行 `island_plugins: 'today-todos',` 之后加：

```ts
  island_scope: 'today',
  island_pass_hover: false,
  island_glow: false,
  island_auto_hide: true,
```

- [x] **Step 9: 校验并提交**

```bash
cd src-tauri && cargo fmt --check && cargo check && cargo test 2>&1 | grep -E "test result|FAILED"
cd .. && pnpm typecheck && pnpm format:check
git add src-tauri/src/domain/models.rs src-tauri/src/data/settings.rs src-tauri/src/data/mod.rs src-tauri/src/app/windows.rs src-tauri/src/ipc.rs src/typings/domain.ts src/composables/useData.ts
git commit -m "feat(settings): 灵动岛四个新设置项（播放范围 / 悬停穿透 / 流光 / 全屏隐藏）、v7 迁移、参数范围对齐原型、settings_save 改 async"
```

---

## Task 2: `island_resize` 命令、悬停穿透守卫、全屏探测与自动隐藏

**Files:**
- Modify: `src-tauri/Cargo.toml:47-55`
- Modify: `src-tauri/src/app/state.rs:1-46`
- Modify: `src-tauri/src/app/windows.rs:659-697`（`island_apply` 之后加 `island_resize`）
- Modify: `src-tauri/src/ipc.rs:602-624,626-630`
- Modify: `src-tauri/src/main.rs:142`
- Modify: `src-tauri/src/platform.rs:108`（`foreground_app_name` 之前插入）
- Modify: `src-tauri/src/services/hotzone_watcher.rs:1-16,27-30,38-42,77-94`
- Modify: `src/service/tauri.ts:77-80`

**Interfaces:**
- Produces（Rust）：
  - `AppState.island_flags: Mutex<IslandFlags>`，`pub struct IslandFlags { pub pass_hover: bool, pub auto_hide: bool }`；`AppState::set_island_flags(&self, flags: IslandFlags)`、`AppState::island_flags(&self) -> IslandFlags`；`AppState::with_store` 从 store 读初值。
  - `windows::island_resize(app: &AppHandle, width: i64, height: i64) -> Result<(), String>`：钳制 → 写 `island_width/height` → `place_island` → emit `SETTINGS_CHANGED`。
  - `ipc::island_resize(app, state, width: i64, height: i64) -> Result<(), String>`（同步）。
  - `platform::foreground_is_fullscreen() -> bool`。
- Produces（TS）：`api.island.resize(width: number, height: number): Promise<void>`。

- [x] **Step 1: `Cargo.toml` 加 Shell 特性**

第 47–55 行 `windows-sys` features 数组里 `"Win32_System_Threading",` 之后加：

```toml
  # 灵动岛全屏自动隐藏：SHQueryUserNotificationState（D30）
  "Win32_UI_Shell",
```

- [x] **Step 2: `app/state.rs` 缓存灵动岛旗标**

第 1 行头注释改为 `//! 全局共享状态：数据库连接池 + 剪贴板回声抑制标记 + 编辑窗口打开参数 + 灵动岛轮询旗标。`。

`pub struct AppState {` 之前插入：

```rust
/// hotzone_watcher 每 80ms 读一次的灵动岛旗标（spec 4B §4.2）。
///
/// 缓存在内存而不是每轮锁库读 settings：轮询线程与 IPC 共用同一把 Store 锁，
/// 每 80ms 拿一次会和保存操作互相拖慢。`settings_save` 时刷新。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IslandFlags {
    /// 悬停穿透：为真时不发 ISLAND_HOVER。
    pub pass_hover: bool,
    /// 前台全屏时自动隐藏灵动岛。
    pub auto_hide: bool,
}
```

`pub pending_panel_intent: Mutex<Option<String>>,` 之后（结构体内）加：

```rust
    /// 灵动岛轮询旗标（悬停穿透 / 全屏隐藏），见 `IslandFlags`。
    pub island_flags: Mutex<IslandFlags>,
```

`with_store` 整体替换为：

```rust
    pub fn with_store(store: Store) -> Self {
        // 启动即按库中设置初始化旗标，避免首次 settings_save 之前 watcher 用错默认值。
        let flags = store
            .get_settings()
            .map(|settings| IslandFlags {
                pass_hover: *settings.island_pass_hover(),
                auto_hide: *settings.island_auto_hide(),
            })
            .unwrap_or(IslandFlags {
                pass_hover: false,
                auto_hide: true,
            });
        eprintln!("[island] 初始旗标 {flags:?}");
        Self {
            store: Mutex::new(store),
            echo: Mutex::new(None),
            editor_payload: Mutex::new(None),
            mindmap_payloads: Mutex::new(std::collections::HashMap::new()),
            hotzone_rects: Mutex::new(std::collections::HashMap::new()),
            island_rect: Mutex::new(None),
            pending_panel_intent: Mutex::new(None),
            island_flags: Mutex::new(flags),
        }
    }

    /// 刷新灵动岛旗标（settings_save 调用）。
    pub fn set_island_flags(&self, flags: IslandFlags) {
        if let Ok(mut slot) = self.island_flags.lock() {
            *slot = flags;
        }
    }

    /// 读取灵动岛旗标；锁损坏时按「不穿透、自动隐藏」兜底。
    pub fn island_flags(&self) -> IslandFlags {
        self.island_flags
            .lock()
            .map(|slot| *slot)
            .unwrap_or(IslandFlags {
                pass_hover: false,
                auto_hide: true,
            })
    }
```

- [x] **Step 3: `windows.rs` 新增 `island_resize`**

`island_apply` 之后（第 682 行 `}` 之后、`/// 悬停展开 / 收起` 之前）插入：

```rust
/// 手柄拖拽落库（原型 diResizer mouseup → renderIsland）：钳制 → 只改宽 / 高两键 → 重新落位 → 广播设置。
///
/// 不经 `settings_save`：那条命令会顺带移动感应区 / 面板并可能建窗；这里窗口已存在，只做几何。
pub fn island_resize(app: &AppHandle, width: i64, height: i64) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut settings = state.lock_store()?.get_settings()?;
    let params = island_clamp(
        width,
        height,
        *settings.island_opacity(),
        *settings.island_cycle_seconds(),
    );
    settings.set_island_width(params.width as i64);
    settings.set_island_height(params.height as i64);
    state.lock_store()?.save_settings(&settings)?;
    eprintln!(
        "[island] 手柄拖拽落库 width={} height={}",
        params.width, params.height
    );
    if let Some(window) = app.get_webview_window(ISLAND_LABEL) {
        let monitor = island_monitor(app).ok_or("未找到主显示器")?;
        let rect = island_geometry(&WorkArea::of(&monitor), params.width, params.height);
        place_island(app, &window, rect)?;
    }
    let _ = app.emit(events::SETTINGS_CHANGED, settings);
    Ok(())
}
```

- [x] **Step 4: `ipc.rs` 刷新缓存 + `island_resize` 命令**

`settings_save`（Task 1 Step 7 版本）里 `state.lock_store()?.save_settings(&settings)?;` 之后插入：

```rust
    // watcher 每轮读的是内存缓存，保存后立刻刷新，悬停穿透 / 全屏隐藏即时生效。
    state.set_island_flags(crate::app::state::IslandFlags {
        pass_hover: *settings.island_pass_hover(),
        auto_hide: *settings.island_auto_hide(),
    });
```

`island_expand` 命令之后插入：

```rust
/// 灵动岛手柄拖拽结束：钳制后写库并重新落位（只做 set_size / set_position，不建窗，保持同步）。
#[tauri::command]
pub fn island_resize(app: AppHandle, width: i64, height: i64) -> Result<(), String> {
    windows::island_resize(&app, width, height)
}
```

`src-tauri/src/main.rs` 第 142 行 `ipc::island_expand,` 之后加 `ipc::island_resize,`。

- [x] **Step 5: `platform.rs` 全屏探测**

第 108 行 `/// 当前前台应用的名字` 之前插入：

```rust
/// 前台是否处于全屏 / 演示态（D30）：`SHQueryUserNotificationState` 返回
/// `QUNS_RUNNING_D3D_FULL_SCREEN`（全屏 D3D 应用）/ `QUNS_PRESENTATION_MODE`（演示模式）/
/// `QUNS_BUSY`（全屏应用占用，如 PPT 放映、视频全屏）时视为全屏；`QUNS_NOT_PRESENT` 与调用失败视为非全屏。
/// 非 Windows 恒为 false。
pub fn foreground_is_fullscreen() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Shell::{
            SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
            QUNS_RUNNING_D3D_FULL_SCREEN, QUERY_USER_NOTIFICATION_STATE,
        };
        let mut state: QUERY_USER_NOTIFICATION_STATE = 0;
        // SAFETY: 只读查询；出参是我们栈上的 i32，系统只写这一个值。
        let hr = unsafe { SHQueryUserNotificationState(&mut state) };
        if hr < 0 {
            eprintln!("[platform] SHQueryUserNotificationState 失败 hr={hr:#x}，视为非全屏");
            return false;
        }
        matches!(
            state,
            QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE | QUNS_BUSY
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
```

- [x] **Step 6: `hotzone_watcher.rs` 守卫与全屏探测**

头注释第 15 行 `//! outer_position：…` 之后追加两行：

```rust
//!
//! 灵动岛（spec 4B §4.2）：悬停穿透开关为真时不发 ISLAND_HOVER；每 12 轮（≈1s）探测前台是否全屏，
//! 与上一次不同且「全屏自动隐藏」为真时 hide / show 灵动岛窗口。
```

第 27–30 行 `POLL_INTERVAL` 之后加：

```rust
/// 全屏探测节拍：每 12 轮（12 × 80ms ≈ 1s）调一次 `platform::foreground_is_fullscreen`。
const FULLSCREEN_PROBE_TICKS: u32 = 12;
```

第 38–42 行局部变量段 `let mut left_was_down = false;` 之后加：

```rust
    // 灵动岛是否已因前台全屏而隐藏（D30）；由本线程独占翻转。
    let mut fullscreen_hidden = false;
```

第 44–48 行 `if tick.is_multiple_of(25) {…}` 之后插入：

```rust
        // 全屏自动隐藏：只在翻转时 hide / show，避免每秒重复调窗口 API。
        if tick.is_multiple_of(FULLSCREEN_PROBE_TICKS) {
            let flags = app.state::<AppState>().island_flags();
            let want_hidden = flags.auto_hide && crate::platform::foreground_is_fullscreen();
            if want_hidden != fullscreen_hidden {
                fullscreen_hidden = want_hidden;
                eprintln!(
                    "[island] 全屏 {}",
                    if want_hidden { "enter" } else { "exit" }
                );
                if let Some(window) = app.get_webview_window(crate::app::windows::ISLAND_LABEL) {
                    let result = if want_hidden {
                        window.hide()
                    } else {
                        window.show()
                    };
                    if let Err(error) = result {
                        eprintln!("[island] 全屏翻转显隐失败: {error}");
                    }
                }
            }
        }
```

第 77–94 行灵动岛悬停段替换为：

```rust
        // 灵动岛：悬停翻转 + 左键按下边沿。穿透模式下窗口自身收不到鼠标事件，
        // 这里是唯一的事件来源；非穿透模式下前端同样只认这一来源，两种模式行为一致。
        // 全屏隐藏期间窗口不可见，一律视为不在区内。
        let island_rect = app.state::<AppState>().island_rect();
        let now_inside = match (island_rect, cursor) {
            (Some(rect), Some(c)) => {
                !panel_visible && !fullscreen_hidden && point_in_rect(c.x, c.y, rect)
            }
            _ => false,
        };
        if now_inside != island_inside {
            island_inside = now_inside;
            // D33 悬停穿透：状态照记（左键边沿仍要用），但不通知前端撑高 / 暂停轮播。
            if app.state::<AppState>().island_flags().pass_hover {
                eprintln!("[island] 悬停状态翻转 inside={now_inside}（悬停穿透，不通知）");
            } else {
                eprintln!("[island] 悬停状态翻转 inside={now_inside}");
                if let Err(error) = app.emit_to(
                    crate::app::windows::ISLAND_LABEL,
                    events::ISLAND_HOVER,
                    now_inside,
                ) {
                    eprintln!("[island] 通知悬停状态失败: {error}");
                }
            }
        }
```

- [x] **Step 7: `tauri.ts` `api.island.resize`**

第 77–80 行 `island: {…}` 替换为：

```ts
  /** 灵动岛。 */
  island: {
    /** 悬停展开 / 收起（只改窗口高度）。 */
    expand: (expanded: boolean) => invoke<void>('island_expand', { expanded }),
    /** 手柄拖拽结束：钳制后写库并重新落位（逻辑像素）。 */
    resize: (width: number, height: number) => invoke<void>('island_resize', { width, height }),
  },
```

- [x] **Step 8: 校验并提交**

```bash
cd src-tauri && cargo fmt --check && cargo check && cargo test 2>&1 | grep -E "test result|FAILED"
cd .. && pnpm typecheck && pnpm format:check
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/app/state.rs src-tauri/src/app/windows.rs src-tauri/src/ipc.rs src-tauri/src/main.rs src-tauri/src/platform.rs src-tauri/src/services/hotzone_watcher.rs src/service/tauri.ts
git commit -m "feat(island): island_resize 命令、悬停穿透守卫、全屏探测与自动隐藏"
```

---

## Task 3: `pickIslandTodos` 纯函数（子任务 / 播放范围 / 逾期优先）+ 单测（TDD）

**Files:**
- Create: `src/utils/island.test.ts`
- Create: `src/utils/island.ts`
- Modify: `src/island-plugins/index.ts:1,13-26,47`
- Modify: `src/island-plugins/today-todos.ts`（整体重写）
- Modify: `src/windows/Main/IslandPageView.vue:6,9-10,33-35,39,106-116`

**Interfaces:**
- Produces（`src/utils/island.ts`）：

```ts
export interface IslandTodoRow {
  id: string
  /** 胶囊文案：顶级 = content；子任务 = `父内容 / 子内容`。 */
  text: string
  /** 归属日（YYYY-MM-DD，本地时区）。 */
  date: string
  /** HH:mm；解析失败为空串。 */
  time: string
  overdue: boolean
  priority: Priority
  todo: Todo
}
export function pickIslandTodos(todos: readonly Todo[], scope: IslandScope, today?: string, now?: Date): IslandTodoRow[]
```

- `IslandItem` 新增可选字段 `date?: string`（非当天时 `MM-DD`）、`overdue?: boolean`、`priority?: Priority`。
- `useTodayTodos()` 内部改为 `pickIslandTodos(todos.value, settings.value.island_scope)`；`pickTodayTodos` **删除**（仅 `IslandPageView` 引用，一并改掉）。

- [x] **Step 1: 写四个失败用例**

`src/utils/island.test.ts`：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import type { Todo } from '@/typings/domain'
import { pickIslandTodos } from './island'

/** 本地时区构造 RFC3339：所有用例以 2026-09-12 为「今天」，当前时刻 12:00。 */
function at(year: number, month: number, day: number, hour: number, minute = 0): string {
  return new Date(year, month - 1, day, hour, minute, 0, 0).toISOString()
}
const TODAY = '2026-09-12'
const NOW = new Date(2026, 8, 12, 12, 0, 0, 0)

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

test('pickIslandTodos：scope=today 取当天与此前逾期的未完成（明天与已完成排除），逾期在前', () => {
  const rows = pickIslandTodos(
    [
      todo({ id: 'a', content: '今天 A', due_at: at(2026, 9, 12, 15) }),
      todo({ id: 'b', content: '明天 B', due_at: at(2026, 9, 13, 9) }),
      todo({ id: 'c', content: '昨天 C', due_at: at(2026, 9, 11, 9) }),
      todo({ id: 'd', content: '已完成 D', due_at: at(2026, 9, 12, 16), status: 'done' }),
    ],
    'today',
    TODAY,
    NOW,
  )
  assert.deepEqual(
    rows.map((row) => row.id),
    ['c', 'a'],
  )
  assert.equal(rows[0].overdue, true)
  assert.equal(rows[1].text, '今天 A')
  assert.equal(rows[1].date, TODAY)
  assert.equal(rows[1].time, '15:00')
  assert.equal(rows[1].overdue, false)
})

test('pickIslandTodos：scope=all 取全部未完成，逾期在前，其余按完成时间升序', () => {
  const rows = pickIslandTodos(
    [
      todo({ id: 'later', content: '明天', due_at: at(2026, 9, 13, 9) }),
      todo({ id: 'today', content: '今天下午', due_at: at(2026, 9, 12, 15) }),
      todo({ id: 'ovd-today', content: '今天早上已过', due_at: at(2026, 9, 12, 9) }),
      todo({ id: 'ovd-old', content: '昨天', due_at: at(2026, 9, 11, 18) }),
      todo({ id: 'done', content: '完成', due_at: at(2026, 9, 10, 9), status: 'done' }),
    ],
    'all',
    TODAY,
    NOW,
  )
  assert.deepEqual(
    rows.map((row) => row.id),
    ['ovd-old', 'ovd-today', 'today', 'later'],
  )
  assert.equal(rows[0].overdue, true)
  assert.equal(rows[0].date, '2026-09-11')
  assert.equal(rows[3].overdue, false)
})

test('pickIslandTodos：子任务文案为「父 / 子」，优先级取子任务自身；父已完成仍按子任务状态入选', () => {
  const rows = pickIslandTodos(
    [
      todo({ id: 'p', content: '写周报', due_at: at(2026, 9, 12, 18), priority: 'high', status: 'done' }),
      todo({ id: 'c1', content: '收集数据', due_at: at(2026, 9, 12, 14), parent_id: 'p', priority: 'low' }),
      todo({ id: 'c2', content: '孤儿子任务', due_at: at(2026, 9, 12, 16), parent_id: 'missing' }),
    ],
    'today',
    TODAY,
    NOW,
  )
  assert.deepEqual(
    rows.map((row) => [row.id, row.text, row.priority]),
    [
      ['c1', '写周报 / 收集数据', 'low'],
      ['c2', '孤儿子任务', 'medium'],
    ],
  )
})

test('pickIslandTodos：空集与全部完成返回空数组', () => {
  assert.deepEqual(pickIslandTodos([], 'today', TODAY, NOW), [])
  assert.deepEqual(
    pickIslandTodos([todo({ id: 'x', content: '完成', due_at: at(2026, 9, 12, 9), status: 'done' })], 'all', TODAY, NOW),
    [],
  )
})
```

- [x] **Step 2: 运行，确认失败**

Run: `pnpm test:unit 2>&1 | grep -E "island|Cannot find" | head`
Expected: `tsc -p tsconfig.test.json` 报 `Cannot find module './island'`（`tsconfig.test.json` include 已含 `src/utils/**/*.ts`）。

- [x] **Step 3: 实现 `src/utils/island.ts`**

```ts
import type { IslandScope, Priority, Todo } from '@/typings/domain'
import { dateKeyOf, formatClock, todayKey } from './datetime'
import { isOverdue } from './todo'

/**
 * 灵动岛播放口径纯函数（原型 app.js `renderIsland` 的收集与排序段，spec 4B §3 #3）。
 *
 * - 未完成的顶级待办与子任务都入选；子任务文案 `父内容 / 子内容`（父不存在时只用子内容），
 *   优先级 / 归属日 / 逾期都取子任务自身；
 * - scope = 'today' 只取归属日等于 today 的；'all' 取全部未完成；
 * - 排序：逾期在前，其余按完成时间升序（原型按 dueTime 字串比较，这里按 due_at 全时刻比较，
 *   'all' 范围下跨日也能排对）。
 *
 * 不碰 DOM、不读时钟（today / now 可注入），灵动岛胶囊与主窗口灵动岛页预览共用，保证两处同源。
 */

export interface IslandTodoRow {
  id: string
  /** 胶囊文案：顶级 = content；子任务 = `父内容 / 子内容`。 */
  text: string
  /** 归属日（YYYY-MM-DD，本地时区）。 */
  date: string
  /** HH:mm；解析失败为空串。 */
  time: string
  overdue: boolean
  priority: Priority
  todo: Todo
}

export function pickIslandTodos(
  todos: readonly Todo[],
  scope: IslandScope,
  today: string = todayKey(),
  now: Date = new Date(),
): IslandTodoRow[] {
  const byId = new Map(todos.map((todo) => [todo.id, todo] as const))
  const rows: IslandTodoRow[] = []
  for (const todo of todos) {
    if (todo.status === 'done') continue
    const date = dateKeyOf(todo.due_at)
    // today 口径与面板待办页 / 托盘一致：当天 + 此前逾期未完成（原型 inScope 只看当天，但本项目自阶段二起
    // 把逾期置顶作为产品语义，灵动岛不能让跨日逾期项凭空消失）；all 取全部未完成。
    const overdue = isOverdue(todo, now)
    if (scope === 'today' && date !== today && !overdue) continue
    const parent = todo.parent_id ? byId.get(todo.parent_id) : undefined
    rows.push({
      id: todo.id,
      text: parent ? `${parent.content} / ${todo.content}` : todo.content,
      date,
      time: formatClock(todo.due_at),
      overdue,
      priority: todo.priority,
      todo,
    })
  }
  rows.sort((a, b) => Number(b.overdue) - Number(a.overdue) || a.todo.due_at.localeCompare(b.todo.due_at))
  return rows
}
```

- [x] **Step 4: 运行测试通过**

Run: `pnpm test:unit 2>&1 | grep -E "pickIslandTodos|^# (pass|fail)"`
Expected: 四个 `ok`，`# fail 0`。

- [x] **Step 5: `island-plugins/index.ts` 增字段、空态文案**

第 1 行 `import type { Component, Ref } from 'vue'` 之后加一行 `import type { Priority } from '@/typings/domain'`。第 13–26 行 `IslandItem` 替换为：

```ts
/** 灵动岛上轮播的一条内容。 */
export interface IslandItem {
  id: string
  /** 胶囊单行主文案。 */
  title: string
  /** 胶囊右侧短文案（时间 / 数量等）。 */
  meta?: string
  /** 左侧色点（CSS 颜色，悬停详情组件用）。 */
  dot?: string
  /** 非当天条目的日期徽章（MM-DD，原型 .di-date）；当天不设。 */
  date?: string
  /** 已逾期（原型 .di-time.ovd）。 */
  overdue?: boolean
  /** 优先级 → 原型 .di-dot.{high|medium|low}；不设时渲染 .idle。 */
  priority?: Priority
  /** 悬停详情组件，接收 `item` prop；不提供则悬停只放大显示 title。 */
  detail?: Component
  /** 原始数据，交给 detail 组件使用。 */
  payload?: unknown
}
```

第 47 行 `emptyText: '今天没有待办 · 点此新建',` 改为 `emptyText: '今日待办已全部完成 🎉',`（scope = all 的文案由 `IslandApp` 按设置覆盖）。

- [x] **Step 6: `today-todos.ts` 整体重写**

```ts
import { computed, type Ref } from 'vue'
import { useSettings, useTodos } from '@/composables/useData'
import type { Priority } from '@/typings/domain'
import { todayKey } from '@/utils/datetime'
import { pickIslandTodos } from '@/utils/island'
import type { IslandItem } from './index'
import TodoDetail from './TodoDetail.vue'

/**
 * 「当日待办」灵动岛插件的条目来源（spec 4B §3 #3）。
 *
 * 口径由 `utils/island.ts::pickIslandTodos` 统一（含子任务、播放范围 today / all、逾期优先），
 * 与主窗口灵动岛页预览同源；跟随 todos-changed 与 settings-changed 自动刷新。
 */

/** 优先级 → 色点颜色（悬停详情组件 TodoDetail 用；胶囊本体改用原型 .di-dot.{prio} 类）。 */
const PRIORITY_DOT: Record<Priority, string> = {
  high: 'var(--red)',
  medium: 'var(--gold)',
  low: 'var(--green)',
}

export function useTodayTodos(): Ref<IslandItem[]> {
  const { todos } = useTodos()
  const { settings } = useSettings()
  return computed<IslandItem[]>(() => {
    const today = todayKey()
    return pickIslandTodos(todos.value, settings.value.island_scope, today).map((row) => ({
      id: row.id,
      title: row.text,
      meta: row.time,
      dot: PRIORITY_DOT[row.priority] ?? 'var(--text-dim)',
      // 原型：非当天才显示 MM-DD 徽章（t.date.slice(5)）。
      date: row.date !== today ? row.date.slice(5) : undefined,
      overdue: row.overdue,
      priority: row.priority,
      detail: TodoDetail,
      payload: row.todo,
    }))
  })
}
```

- [x] **Step 7: `IslandPageView.vue` 预览改用 `pickIslandTodos`**

第 6 行 `import { pickTodayTodos } from '@/island-plugins/today-todos'` 改为 `import { pickIslandTodos } from '@/utils/island'`；第 9 行 `import { formatClock } from '@/utils/datetime'` 改为 `import { todayKey } from '@/utils/datetime'`；第 10 行 `import { isOverdue } from '@/utils/todo'` 删除。

第 33–35 行替换为：

```ts
/** 播放口径与胶囊同源（utils/island.ts：含子任务、按播放范围、逾期优先）。 */
const islandRows = computed(() => pickIslandTodos(todos.value, settings.value.island_scope))
const first = computed(() => islandRows.value[0] ?? null)
/** 预览里非当天条目带 MM-DD 徽章（原型 .di-date）。 */
const firstDate = computed(() => (first.value && first.value.date !== todayKey() ? first.value.date.slice(5) : ''))
```

第 39 行 `当前播放 ${todayTodos.value.length} 条待办` 改为 `当前播放 ${islandRows.value.length} 条待办`。

模板第 106–116 行替换为：

```vue
        <div class="di-item island-preview-item">
          <template v-if="first">
            <span class="di-dot" :class="first.priority" />
            <span class="di-text"><span v-if="firstDate" class="di-date">{{ firstDate }}</span>{{ first.text }}</span>
            <span class="di-time" :class="{ ovd: first.overdue }">{{ first.overdue ? `逾期 ${first.time}` : first.time }}</span>
          </template>
          <template v-else>
            <span class="di-dot idle" />
            <span class="di-text dim">{{ settings.island_scope === 'all' ? '没有未完成待办 🎉' : '今日待办已全部完成 🎉' }}</span>
          </template>
        </div>
```

- [x] **Step 8: 校验并提交**

```bash
pnpm test:unit 2>&1 | grep -E "^# (pass|fail)" && pnpm typecheck && pnpm format:check && (grep -rn "pickTodayTodos" src || echo "无残留引用")
git add src/utils/island.ts src/utils/island.test.ts src/island-plugins/index.ts src/island-plugins/today-todos.ts src/windows/Main/IslandPageView.vue
git commit -m "feat(island): pickIslandTodos 纯函数（子任务 / 播放范围 / 逾期优先）+ 单测"
```

---

## Task 4: 灵动岛 DOM 与逐条停留轮播对齐原型，提醒岛内呈现、面板展开淡出、手柄拖拽，删除 `island.css`

**Files:**
- Modify: `src/windows/Island/IslandApp.vue`（整体重写）
- Modify: `src/windows/island.ts:8-11`
- Delete: `src/styles/island.css`
- Modify: `src/styles/extensions.css:698-707`（§8：删 `.island,` 选择器行，本任务只留 `.hotzone-indicator`，Task 6 整节删除）；文件末尾新增 §10
- Modify: `src/styles/window-fit.css`（`#pinnedWindow, #reminderCard { … clip-path }` 段之后新增灵动岛窗段）

**Interfaces:**
- Consumes：Task 2 `api.island.resize`；Task 3 `IslandItem.date / overdue / priority`；`Settings.island_scope / island_pass_hover / island_glow / island_click_through / island_cycle_seconds / island_height / island_opacity`；事件 `AppEvents.islandHover / islandClick / reminderFired / panelShown / panelHidden`；`useTodos().todos`（按 `reminderFired` 的 todo id 取内容）。
- Produces：灵动岛窗口根元素 `#dynamicIsland.glass`，类 `di-fade` / `di-pass-click` / `di-glow` / `expanded` / `clicked`，CSS 变量 `--island-alpha` / `--cap-h`；`extensions.css` §10 规则 `#dynamicIsland.glass` 底色按 `--island-alpha` 混合、`--cap-h` 过渡、`.expanded`、`.island-expanded` 系列、`.island-fade-*`、`.clicked`、`.pinned-editor`（Task 7 用）；`window-fit.css` 灵动岛窗铺满 + `clip-path: inset(0 round 999px)`。

- [x] **Step 1: 重写 `src/windows/Island/IslandApp.vue`**

```vue
<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch, type CSSProperties } from 'vue'
import { useSettings, useTodos } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { resolveIslandPlugins, type IslandItem } from '@/island-plugins'
import { AppEvents, onAppEvent } from '@/service/events'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 灵动岛根组件（原型 #dynamicIsland / renderIsland / diStep，spec 4B §4.1）。
 *
 * - DOM 按原型：`.di-ticker > .di-track > .di-item*`，条目末尾追加第一条副本做无缝回环；
 * - 逐条停留轮播：每条静止 `island_cycle_seconds` 秒 → `translateY(-i*h)` 0.45s ease → 滚到副本后 480ms
 *   无过渡复位（diStep）；仅一条或空态静止；展开 / 提醒 / 面板打开时不步进；
 * - 悬停 / 点击不用 DOM 鼠标事件，统一订阅后端推送的 island-hover / island-click（穿透模式下窗口收不到鼠标事件）；
 *   悬停穿透（D33）开关为真时忽略悬停事件（后端也不发，双保险）；
 * - 提醒岛内呈现：reminder-fired（payload = todo id）→ `.di-alert`「⏰ 提醒：内容」6s 或点击恢复；
 * - 面板展开淡出：panel-shown → `.di-fade`，panel-hidden / 下一次悬停事件 → 恢复；
 * - 右下角手柄：宽按 Δx×2 对称、高按 Δy，钳制 200–480 / 32–56，松手 `api.island.resize` 落库（穿透时不渲染手柄）。
 */
applyCachedTheme()
applyCachedGlass()
document.documentElement.dataset.window = 'island'

const { settings } = useSettings()
const { todos } = useTodos()
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

// ── 条目 ──

// 插件在 setup 内各调用一次 useItems（组合式函数，不能放进 computed 里反复调用）。
const pluginItems = resolveIslandPlugins(settings.value.island_plugins).map((plugin) => ({
  plugin,
  items: plugin.useItems(),
}))
const enabledIds = computed(() => resolveIslandPlugins(settings.value.island_plugins).map((p) => p.id))
/** 全部条目：按设置里的插件顺序拼接。 */
const items = computed<IslandItem[]>(() =>
  enabledIds.value.flatMap((id) => pluginItems.find((entry) => entry.plugin.id === id)?.items.value ?? []),
)
/** 空态文案（原型按播放范围二选一）。 */
const emptyText = computed(() => {
  if (settings.value.island_scope === 'all') return '没有未完成待办 🎉'
  return pluginItems.find((entry) => enabledIds.value.includes(entry.plugin.id))?.plugin.emptyText ?? '今日待办已全部完成 🎉'
})

/** 轨道上的一行（原型 .di-item 的渲染参数）。 */
interface IslandRow {
  key: string
  dotClass: 'high' | 'medium' | 'low' | 'idle'
  text: string
  date: string
  time: string
  overdue: boolean
  dim: boolean
}

/** 轨道行：条目映射为行；多于一条时末尾追加第一条副本（原型 items[0] 再拼一次）。 */
const rows = computed<IslandRow[]>(() => {
  const list = items.value
  if (!list.length) {
    return [{ key: 'empty', dotClass: 'idle', text: emptyText.value, date: '', time: '', overdue: false, dim: true }]
  }
  const mapped = list.map<IslandRow>((item) => ({
    key: item.id,
    dotClass: item.priority ?? 'idle',
    text: item.title,
    date: item.date ?? '',
    // 原型：逾期时时间前缀「逾期 」。
    time: item.overdue ? `逾期 ${item.meta ?? ''}`.trimEnd() : (item.meta ?? ''),
    overdue: Boolean(item.overdue),
    dim: false,
  }))
  return mapped.length > 1 ? [...mapped, { ...mapped[0], key: `${mapped[0].key}:clone` }] : mapped
})

// ── 几何：行高 = 胶囊折叠高；展开时撑到 max(高, 120) ──

/** 原型 DI.H_MIN / H_MAX / W_MIN / W_MAX（与 Rust island_clamp 一致）。 */
const LIMITS = { wMin: 200, wMax: 480, hMin: 32, hMax: 56 } as const
/** 悬停展开高度下限（与 Rust ISLAND_EXPANDED_HEIGHT 一致）。 */
const EXPANDED_MIN_HEIGHT = 120
/** 胶囊高度 CSS 过渡时长（extensions.css §10 的 0.32s），收起时据此延后缩窗。 */
const EXPAND_MS = 320
/** 原型 diStep：切换过渡 .45s，到副本后 480ms 复位。 */
const STEP_MS = 450
const RESET_MS = 480

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

/** 拖拽中的临时高度；null = 用设置值。 */
const dragHeight = ref<number | null>(null)
const itemH = computed(() => dragHeight.value ?? clamp(settings.value.island_height || 36, LIMITS.hMin, LIMITS.hMax))
const expanded = ref(false)
const capH = computed(() => (expanded.value ? Math.max(itemH.value, EXPANDED_MIN_HEIGHT) : itemH.value))
/** 背景不透明度：只透底色，文字始终不透明（D32 保留）。 */
const alpha = computed(() => clamp(settings.value.island_opacity || 0.85, 0.3, 1))

const rootStyle = computed<CSSProperties>(() => ({
  '--island-alpha': String(alpha.value),
  '--cap-h': `${capH.value}px`,
}))

// ── 逐条停留轮播（原型 diStep）──

const diIndex = ref(0)
const trackStyle = ref<CSSProperties>({ transition: 'none', transform: 'translateY(0)' })
let stepTimer: ReturnType<typeof setTimeout> | null = null
let resetTimer: ReturnType<typeof setTimeout> | null = null

const stayMs = computed(() => clamp(settings.value.island_cycle_seconds || 3, 1, 10) * 1000)
/** 轮播是否应该跑：多于一条，且没有展开 / 提醒 / 面板遮挡。 */
const panelOpen = ref(false)
const alert = ref<string | null>(null)
const canCycle = computed(() => items.value.length > 1 && !expanded.value && alert.value === null && !panelOpen.value)

function stopTicker(): void {
  if (stepTimer) clearTimeout(stepTimer)
  stepTimer = null
}

function scheduleStep(): void {
  stopTicker()
  if (!canCycle.value) return
  stepTimer = setTimeout(step, stayMs.value)
}

/** 原型 diStep：步进一行；到副本后过渡结束时无过渡复位到真正第一条。 */
function step(): void {
  stepTimer = null
  if (!canCycle.value) return
  const count = items.value.length
  diIndex.value += 1
  trackStyle.value = {
    transition: `transform ${STEP_MS}ms ease`,
    transform: `translateY(${-diIndex.value * itemH.value}px)`,
  }
  if (diIndex.value === count) {
    if (resetTimer) clearTimeout(resetTimer)
    resetTimer = setTimeout(() => {
      resetTimer = null
      diIndex.value = 0
      trackStyle.value = { transition: 'none', transform: 'translateY(0)' }
    }, RESET_MS)
  }
  scheduleStep()
}

/** 条目 / 停留时长变化：回到第一条重新计时（原型 renderIsland 重建轨道）。 */
function resetTicker(): void {
  stopTicker()
  if (resetTimer) clearTimeout(resetTimer)
  resetTimer = null
  diIndex.value = 0
  trackStyle.value = { transition: 'none', transform: 'translateY(0)' }
  logger.debug('island', `轮播重置 items=${items.value.length} stay=${stayMs.value}ms`)
  scheduleStep()
}

watch(() => items.value.map((item) => item.id).join('|'), resetTicker)
watch(stayMs, resetTicker)
watch(canCycle, (can) => {
  if (can) scheduleStep()
  else stopTicker()
})

// ── 提醒岛内呈现（原型 showIslandAlert / hideIslandAlert）──

const ALERT_MS = 6000
let alertTimer: ReturnType<typeof setTimeout> | null = null

function showAlert(content: string): void {
  if (alertTimer) clearTimeout(alertTimer)
  alert.value = content
  logger.info('island', `提醒岛内呈现：${content}`)
  alertTimer = setTimeout(hideAlert, ALERT_MS)
}

function hideAlert(): void {
  if (alertTimer) clearTimeout(alertTimer)
  alertTimer = null
  if (alert.value === null) return
  alert.value = null
}

// ── 悬停展开（后端推送；D33 悬停穿透守卫）──

const clicked = ref(false)
const current = computed<IslandItem | null>(() => items.value[diIndex.value % Math.max(items.value.length, 1)] ?? null)

/** 展开序号：async 期间用于「后来者优先」，避免快速进出导致窗口尺寸错乱。 */
let expandSeq = 0

/**
 * 切换展开态。为消除窗口 set_size 的瞬时跳变：
 * - 展开：先把窗口撑到展开高（透明空间落在胶囊下方，不可见），再置 expanded → 胶囊用 CSS 平滑生长、详情淡入；
 * - 收起：先置 expanded=false → 胶囊 CSS 收缩，待动画结束再缩窗。
 */
async function setExpanded(next: boolean): Promise<void> {
  if (expanded.value === next) return
  const seq = ++expandSeq
  try {
    if (next) {
      await api.island.expand(true)
      if (seq !== expandSeq) return
      expanded.value = true
    } else {
      expanded.value = false
      await new Promise((resolve) => setTimeout(resolve, EXPAND_MS))
      if (seq !== expandSeq) return
      await api.island.expand(false)
    }
  } catch (error) {
    logger.error('island', '切换展开状态失败', error)
  }
}

// ── 手柄拖拽（原型 diResizer）──

/** 拖拽起点与起始尺寸；null = 未在拖拽。 */
let drag: { x: number; y: number; w: number; h: number } | null = null
/** 拖拽中的目标宽度（逻辑像素）；窗口宽度由后端在松手后统一调整。 */
const dragWidth = ref<number | null>(null)

function startResize(event: MouseEvent): void {
  if (settings.value.island_click_through) return
  drag = {
    x: event.clientX,
    y: event.clientY,
    w: clamp(settings.value.island_width || 320, LIMITS.wMin, LIMITS.wMax),
    h: itemH.value,
  }
  stopTicker()
  document.addEventListener('mousemove', onResizeMove)
  document.addEventListener('mouseup', onResizeEnd)
  logger.debug('island', `开始拖拽手柄 w=${drag.w} h=${drag.h}`)
}

function onResizeMove(event: MouseEvent): void {
  if (!drag) return
  dragWidth.value = clamp(Math.round(drag.w + (event.clientX - drag.x) * 2), LIMITS.wMin, LIMITS.wMax)
  dragHeight.value = clamp(Math.round(drag.h + (event.clientY - drag.y)), LIMITS.hMin, LIMITS.hMax)
}

async function onResizeEnd(): Promise<void> {
  document.removeEventListener('mousemove', onResizeMove)
  document.removeEventListener('mouseup', onResizeEnd)
  if (!drag) return
  const width = dragWidth.value ?? drag.w
  const height = dragHeight.value ?? drag.h
  drag = null
  logger.info('island', `手柄拖拽结束 width=${width} height=${height}`)
  try {
    await api.island.resize(width, height)
  } catch (error) {
    logger.error('island', '落库灵动岛尺寸失败', error)
  } finally {
    // 设置已由后端广播回来，清掉临时值改用设置值；轨道按新行高重排。
    dragWidth.value = null
    dragHeight.value = null
    resetTicker()
  }
}

// ── 事件订阅 ──

onMounted(() => {
  scheduleStep()
  void onAppEvent<boolean>(AppEvents.islandHover, (inside) => {
    // 后端在面板可见时不发悬停：收到悬停即说明面板已收起，补一次淡出恢复（隐藏期间事件可能丢）。
    panelOpen.value = false
    if (settings.value.island_pass_hover) {
      logger.debug('island', '悬停穿透开启，忽略悬停事件')
      return
    }
    logger.debug('island', inside ? '光标进入' : '光标离开')
    void setExpanded(inside)
  })
  void onAppEvent(AppEvents.islandClick, () => {
    logger.info('island', '点击，面板已由后端呼出')
    clicked.value = true
    setTimeout(() => (clicked.value = false), 300)
  })
  void onAppEvent<boolean>(AppEvents.panelShown, () => {
    panelOpen.value = true
  })
  void onAppEvent(AppEvents.panelHidden, () => {
    panelOpen.value = false
  })
  void onAppEvent<string>(AppEvents.reminderFired, (id) => {
    const target = todos.value.find((todo) => todo.id === id)
    if (!target) {
      logger.warn('island', `提醒 ${id} 未在本地待办列表中，跳过岛内呈现`)
      return
    }
    showAlert(target.content)
  })
})

onBeforeUnmount(() => {
  stopTicker()
  if (resetTimer) clearTimeout(resetTimer)
  if (alertTimer) clearTimeout(alertTimer)
  document.removeEventListener('mousemove', onResizeMove)
  document.removeEventListener('mouseup', onResizeEnd)
})
</script>

<template>
  <div
    id="dynamicIsland"
    class="glass"
    :class="{
      'di-fade': panelOpen,
      'di-pass-click': settings.island_click_through,
      'di-glow': settings.island_glow,
      expanded,
      clicked,
    }"
    :style="rootStyle"
  >
    <!-- 收起态：原型轨道，JS 步进 translateY；展开态 v-if 隐藏轨道避免与 --cap-h 过渡冲突（spec §8） -->
    <div v-if="!expanded" class="di-ticker">
      <div class="di-track" :style="trackStyle">
        <div v-for="row in rows" :key="row.key" class="di-item" :style="{ height: itemH + 'px' }">
          <span class="di-dot" :class="row.dotClass" />
          <span class="di-text" :class="{ dim: row.dim }"
            ><span v-if="row.date" class="di-date">{{ row.date }}</span>{{ row.text }}</span
          >
          <span v-if="row.time" class="di-time" :class="{ ovd: row.overdue }">{{ row.time }}</span>
        </div>
      </div>
    </div>

    <!-- 提醒岛内呈现：覆盖轨道，点击或 6s 后恢复 -->
    <div v-if="alert !== null" class="di-alert" title="点击关闭提醒并恢复轮播" @click="hideAlert">
      <span class="di-alert-ico">⏰</span><span class="di-alert-text">提醒：{{ alert }}</span>
    </div>

    <!-- 右下角手柄（穿透模式不渲染：窗口整体 set_ignore_cursor_events，手柄本就收不到鼠标） -->
    <div
      v-if="!settings.island_click_through && !expanded"
      class="di-resizer"
      title="拖动调整胶囊大小（宽 200-480 · 高 32-56）"
      @mousedown.prevent="startResize"
    />

    <!-- 展开态：当前条目的详情组件（随胶囊生长淡入） -->
    <Transition name="island-fade">
      <div v-if="expanded" class="island-expanded">
        <component :is="current.detail" v-if="current?.detail" :item="current" />
        <div v-else class="island-detail">
          <div class="island-detail-title">{{ current?.title ?? emptyText }}</div>
        </div>
        <div class="island-hint">左键点击打开待办面板</div>
      </div>
    </Transition>
  </div>
</template>
```

- [x] **Step 2: `island.ts` 删 `island.css` 引入；删除文件**

`src/windows/island.ts` 第 8–11 行替换为：

```ts
import { createApp } from 'vue'
import IslandApp from './Island/IslandApp.vue'
import '@/styles'
```

头注释第 5 行改为 `* 内容由 \`@/island-plugins\` 注册表提供，按原型 #dynamicIsland 结构逐条停留轮播；悬停展开详情、左键点击唤出面板到待办页。`。

`git rm src/styles/island.css`。

- [x] **Step 3: `extensions.css` §8 删 `.island,`；新增 §10**

（a）第 698–707 行 §8：注释第一行改为 `/* ═══ 8. 阶段一验收补丁（实机发现，Task 6 感应区指示器令牌化后整节删除） ═══`，第 702 行 `.island,` 整行删除（选择器只剩 `.hotzone-indicator {`）；注释第二行 `island.css 的胶囊与本文件第 6 节的感应区指示器` 改为 `本文件第 6 节的感应区指示器`。

（b）文件末尾（§9 之后）追加：

```css

/* ═══ 10. 灵动岛与置顶扩展（阶段四 B，2026-09-12） ═══
 * 原型 #dynamicIsland / .di-* 全部由生成层提供；这里只放项目扩展：
 *   - 透明度只透底色（D32）：.glass 的 --glass-bg 按 --island-alpha 与透明混合，文字不透明；
 *   - 胶囊高度走 --cap-h 过渡（窗口 set_size 的瞬时跳变被透明区遮住，见 IslandApp.setExpanded）；
 *   - 悬停展开态（原型无，阶段四 D24 保留）：放宽圆角、详情淡入；
 *   - 点击反馈 .clicked；
 *   - 置顶浮窗编辑态 .pinned-editor（D31）。 */
:root[data-window='island'],
:root[data-window='island'] body,
:root[data-window='island'] #app {
  background: transparent !important;
  overflow: hidden;
}
#dynamicIsland.glass {
  --island-alpha: 0.85;
  --cap-h: 36px;
  background: color-mix(in srgb, var(--glass-bg) calc(var(--island-alpha) * 100%), transparent);
  color: var(--text);
  display: flex;
  align-items: center;
  /* 丝滑展开：高度 / 圆角 / 内边距同步缓动（收尾减速），点击态用更快的回弹；opacity 沿用原型 .2s（.di-fade）。 */
  transition:
    height 0.32s cubic-bezier(0.22, 1, 0.36, 1),
    border-radius 0.32s cubic-bezier(0.22, 1, 0.36, 1),
    padding 0.32s cubic-bezier(0.22, 1, 0.36, 1),
    opacity 0.2s ease,
    background 0.2s ease;
}
#dynamicIsland .di-ticker {
  flex: 1;
  min-width: 0;
}
#dynamicIsland.clicked .di-ticker {
  transform: scale(0.98);
  transition: transform 0.12s ease;
}
#dynamicIsland.expanded {
  align-items: flex-start;
  padding: 12px 16px;
  border-radius: 18px;
  clip-path: inset(0 round 18px);
}
/* 展开内容淡入 / 收起淡出（随胶囊生长同步呈现，避免详情硬跳出）。 */
.island-fade-enter-active {
  transition:
    opacity 0.26s ease 0.06s,
    transform 0.26s cubic-bezier(0.22, 1, 0.36, 1) 0.06s;
}
.island-fade-leave-active {
  transition:
    opacity 0.14s ease,
    transform 0.14s ease;
}
.island-fade-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.island-fade-leave-to {
  opacity: 0;
}
.island-expanded {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
  min-width: 0;
  font-size: 12.5px;
}
.island-detail {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.island-detail-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  line-height: 1.4;
}
.island-dot {
  flex: none;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.island-overdue {
  padding: 0 6px;
  border-radius: 999px;
  background: rgba(255, 95, 87, 0.18);
  color: var(--red);
  font-size: 10.5px;
  font-weight: 500;
}
.island-detail-row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  color: var(--text-dim);
  font-size: 11.5px;
}
.island-detail-remark {
  color: var(--text-dim);
  font-size: 11.5px;
  line-height: 1.5;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
.island-hint {
  margin-top: auto;
  color: var(--text-dim);
  font-size: 10.5px;
  opacity: 0.8;
}
/* 置顶浮窗编辑态（D31）：文本域铺满正文区，透明底、无边框，字号行高与 .pinned-body 一致。 */
.pinned-editor {
  flex: 1;
  min-height: 0;
  width: 100%;
  resize: none;
  background: transparent;
  border: 0;
  outline: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  line-height: 1.55;
}
```

- [x] **Step 4: `window-fit.css` 灵动岛窗铺满 + 圆角裁剪**

`#pinnedWindow, #reminderCard { border-radius: 8px; clip-path: inset(0 round 8px); }`（第 122–126 行）之后插入：

```css

/* ═══ 灵动岛窗口 ═══
 * 原型把 #dynamicIsland 当桌面浮层（fixed + translateX(-50%) + 320×36）；真实窗口就是胶囊本身，
 * 由 Rust 落位并 set_size，这里铺满窗宽、高度走 --cap-h（折叠 = 设置高，展开 = max(高, 120)）。
 * 毛玻璃绘制会盖掉 999px 圆角，与面板同理用 clip-path 裁齐（spec §8 风险项）。 */
#dynamicIsland {
  position: relative;
  top: auto;
  left: auto;
  transform: none;
  box-sizing: border-box;
  width: 100%;
  height: var(--cap-h);
  clip-path: inset(0 round 999px);
}
```

- [x] **Step 5: 校验并提交**

```bash
pnpm sync:styles --check && pnpm typecheck && pnpm format:check && (grep -rn "island\.css\|island-line\|island-row\|island-title\|island-meta\|island-count\|island-roll" src || echo "无残留引用")
pnpm exec vite build 2>&1 | grep -E "built in|error"
git add -A src/windows/Island/IslandApp.vue src/windows/island.ts src/styles/island.css src/styles/extensions.css src/styles/window-fit.css
git commit -m "feat(island): 灵动岛 DOM 与逐条停留轮播对齐原型，提醒岛内呈现、面板展开淡出、手柄拖拽，删除 island.css"
```

---

## Task 5: 灵动岛页补四个设置项与状态行

**Files:**
- Modify: `src/windows/Main/IslandPageView.vue:8,12-18,37-51,124-163`

**Interfaces:**
- Consumes：Task 1 `Settings.island_scope / island_pass_hover / island_glow / island_auto_hide`、`IslandScope`。
- Produces：`ISLAND_LIMITS = { width: 200–480, height: 32–56, opacity: 0.3–1, cycle: 1–10 }`；状态行含「范围：仅当天待办 / 全部未完成待办」与「悬停穿透」；设置顺序按原型：启用 → 停留 → 播放范围 → 透明度 → 点击穿透 → 悬停穿透 → 流光 → 全屏隐藏 → 宽 → 高 → 插件。

- [x] **Step 1: 脚本段**

第 8 行 `import type { Settings } from '@/typings/domain'` 改为 `import type { IslandScope, Settings } from '@/typings/domain'`。

头注释第 15–17 行（`+ 状态行 + 「灵动岛设置」：…` 至 `…（spec D10，留阶段四）。`）替换为：

```
 * + 状态行 + 「灵动岛设置」八项（原型顺序：启用 / 停留 / 播放范围 / 透明度 / 点击穿透 / 悬停穿透 / 流光 / 全屏隐藏）
 * + 项目扩展（宽 / 高 / 插件，排在其后）。预览口径与胶囊同源（utils/island.ts）。
```

第 37–43 行 `stat` 替换为：

```ts
/** 状态行（原型 islandPageStat：播放条数 · 停留 · 范围 · 停用 · 点击穿透 · 悬停穿透）。 */
const stat = computed(() => {
  const parts = [
    `当前播放 ${islandRows.value.length} 条待办`,
    `每条停留 ${settings.value.island_cycle_seconds} 秒`,
    `范围：${settings.value.island_scope === 'all' ? '全部未完成待办' : '仅当天待办'}`,
  ]
  if (!settings.value.island_enabled) parts.push('灵动岛已停用')
  if (settings.value.island_click_through) parts.push('点击穿透')
  if (settings.value.island_pass_hover) parts.push('悬停穿透')
  return parts.join(' · ')
})
```

第 45–51 行 `ISLAND_LIMITS` 替换为：

```ts
/** 灵动岛数值项的允许范围（原型 DI.W_MIN/W_MAX/H_MIN/H_MAX 与停留 1–10；与 Rust island_clamp 一致，前端先钳一遍避免来回抖动）。 */
const ISLAND_LIMITS = {
  width: { min: 200, max: 480 },
  height: { min: 32, max: 56 },
  opacity: { min: 0.3, max: 1 },
  cycle: { min: 1, max: 10 },
} as const
```

- [x] **Step 2: 模板补四行**

「滚动停留时间」`</label>`（第 143 行）之后插入：

```vue
      <label class="setting-row">
        <span>播放范围</span>
        <select
          :value="settings.island_scope"
          @change="patch({ island_scope: ($event.target as HTMLSelectElement).value as IslandScope })"
        >
          <option value="today">仅当天待办（默认）</option>
          <option value="all">全部未完成待办</option>
        </select>
      </label>
```

「点击穿透」`</label>`（原第 162 行）之后插入：

```vue
      <label class="setting-row">
        <span>悬停穿透（悬停不展开详情、不暂停轮播）</span>
        <input
          type="checkbox"
          :checked="settings.island_pass_hover"
          @change="patch({ island_pass_hover: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>流光边框</span>
        <input
          type="checkbox"
          :checked="settings.island_glow"
          @change="patch({ island_glow: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>全屏自动隐藏</span>
        <input
          type="checkbox"
          :checked="settings.island_auto_hide"
          @change="patch({ island_auto_hide: ($event.target as HTMLInputElement).checked })"
        />
      </label>
```

- [x] **Step 3: 校验并提交**

```bash
pnpm typecheck && pnpm format:check
git add src/windows/Main/IslandPageView.vue
git commit -m "feat(island): 灵动岛页补四个设置项与状态行"
```

---

## Task 6: 感应区指示器改用令牌、提醒卡片入退场动效，删除 `extensions.css` §8

**Files:**
- Modify: `src/styles/extensions.css:141-163`（§6 `.hotzone-indicator`）、`698-706`（删 §8 整节）
- Modify: `src/windows/Reminder/ReminderApp.vue`

**Interfaces:**
- Consumes：`enter(el, { axis: 'x', distance: 60, duration: 'slow' })` / `exit(el, { axis: 'x', distance: 60, duration: 'base' })`（`src/motion/presets.ts` 既有签名，`enter` slow 档 = outBack(1.6)，原型 back.out(1.5) 差异接受）；`--glass-bg` / `--glass-border` 令牌（`tokens.css:73-74`，30 套主题均有覆盖）。
- Produces：`ReminderApp` 内部 `closeCard(): Promise<void>`（退场后 `reminderClose`），`dismiss` / `snooze` 改调它。

- [x] **Step 1: §6 指示器令牌化**

第 141–163 行 `.hotzone-indicator {…}` 里两条声明替换：`border: 1px solid rgba(var(--wsa), 0.16);` → `border: 1px solid var(--glass-border);`；`background: rgba(24, 26, 40, 0.74);` → `background: var(--glass-bg);`。其余声明（`--hz-rest` / `--hz-in` / 尺寸 / 阴影 / `backdrop-filter` / 过渡）不动；`.hotzone-indicator-label` 已是 `color: var(--text-dim)`，不改。

- [x] **Step 2: 删除 §8 整节**

删除第 698–706 行（`/* ═══ 8. 阶段一验收补丁 … ═══` 注释块 + `.hotzone-indicator { --text / --text-dim / --wsa }` 规则 + 其后空行），§9 紧跟 §7 之后。

- [x] **Step 3: `ReminderApp.vue` 入退场**

第 3 行 import 改为 `import { computed, onMounted, ref, watch } from 'vue'` 之后加一行 `import { enter, exit } from '@/motion'`。

头注释第 10–16 行末尾追加一行：`* 入场 x:60 → 0（--dur-slow，回弹）、退场 0 → x:60（--dur-base）后再关窗（原型 showReminder / hideReminder）。`

`const content = ref('')` 之后加：

```ts
const card = ref<HTMLElement | null>(null)
let closing = false

/** 退场动效播完再关窗；关闭 / 顺延 / 完成三条路径共用，重复触发只走一次。 */
async function closeCard(): Promise<void> {
  if (closing) return
  closing = true
  if (card.value) await exit(card.value, { axis: 'x', distance: 60, duration: 'base' })
  await api.windows.reminderClose(todoId.value).catch(() => undefined)
}
```

`dismiss()` 末行、`snooze()` 里两处 `await api.windows.reminderClose(todoId.value).catch(() => undefined)` 均改为 `await closeCard()`。

第 98 行 `onMounted(load)` 替换为：

```ts
onMounted(() => {
  void load()
  if (card.value) void enter(card.value, { axis: 'x', distance: 60, duration: 'slow' })
})
```

模板第 102 行 `<div id="reminderCard" class="glass">` 改为 `<div id="reminderCard" ref="card" class="glass">`。

- [x] **Step 4: 校验并提交**

```bash
pnpm sync:styles --check && pnpm typecheck && pnpm format:check && (grep -n "rgba(24, 26, 40" src/styles/extensions.css || echo "固定深色底已清除")
git add src/styles/extensions.css src/windows/Reminder/ReminderApp.vue
git commit -m "feat(ui): 感应区指示器改用令牌、提醒卡片入退场动效，删除 extensions.css §8"
```

---

## Task 7: 置顶浮窗双击编辑回写

**Files:**
- Modify: `src/windows/Pinned/PinnedApp.vue`（整体重写）
- Modify: `src-tauri/src/app/windows.rs:20`

**Interfaces:**
- Consumes：`api.notes.get(id) / api.notes.save(NoteInput)`、`api.todos.list() / api.todos.save(TodoInput)`、`api.clipboard.update(id, content)`、`api.windows.pinSetEditing(label, expanded)`；`useToast().toast`；Task 4 §10 `.pinned-editor`。
- Produces：`PinnedApp` 状态 `editing: Ref<boolean>`、`draft: Ref<string>`；`beginEdit()` / `save()` / `cancelEdit()`；模板加 `<ToastHost />`。`windows.rs` `PINNED_SIZE` 宽 230 → 220（spec #25，用户提法 `PIN_WIDTH` 即此常量）。

- [x] **Step 1: 重写 `PinnedApp.vue`**

```vue
<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import { useSettings } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Note, Todo } from '@/typings/domain'
import { renderMarkdown } from '@/utils/format'

/**
 * 桌面置顶浮窗（需求 2.5，原型 #pinnedWindow；spec 4B D31）。
 *
 * 每个置顶项是一个独立窗口，label 形如 `pinned-{kind}-{id}`，前端据此解析自己该显示哪条内容。
 * - 透明度调节作用于整窗；
 * - 双击正文进入编辑态：窗口由 Rust 放大（pinSetEditing），正文换成 .pinned-editor 文本域并聚焦置末；
 *   Ctrl+Enter / 失焦保存（按 kind 走 notes.save / todos.save / clipboard.update），toast「已同步回数据库 ✔」；Esc 放弃；
 * - 笔记回写先 notes.get 取原字段（标签 / 编辑模式 / 导图数据），否则会被清空（spec §8）；
 *   待办回写保留其余字段，已完成待办后端会拒绝改内容，错误原样 toast。
 */
applyCachedTheme()
applyCachedGlass()

const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
const { toast } = useToast()
watch(() => settings.value.theme, applyTheme, { immediate: true })
// 玻璃质感与主题同源：后端设置变化时一并同步。
watch(() => settings.value.glass_level, applyGlass, { immediate: true })

const label = getCurrentWindow().label
/** 从窗口 label 解析出 kind 与 id：pinned-note-xxxx。 */
const parsed = computed(() => {
  const match = /^pinned-(note|todo|clip)-(.+)$/.exec(label)
  return match ? { kind: match[1] as 'note' | 'todo' | 'clip', id: match[2] } : null
})

const content = ref('')
const opacity = ref(100)
const editing = ref(false)
const draft = ref('')
const editor = ref<HTMLTextAreaElement | null>(null)
/** 目标已被删除时不允许进入编辑。 */
const missing = ref(false)

const html = computed(() => renderMarkdown(content.value))

/** 按 kind 拉取对应内容。 */
async function load(): Promise<void> {
  const target = parsed.value
  if (!target) {
    logger.error('pinned', `无法解析窗口 label: ${label}`)
    return
  }

  try {
    let found: string | undefined
    if (target.kind === 'note') {
      found = (await api.notes.get(target.id))?.content
    } else if (target.kind === 'todo') {
      const todos = await api.todos.list()
      found = todos.find((t) => t.id === target.id)?.content
    } else {
      const clips = await api.clipboard.list()
      found = clips.find((c) => c.id === target.id)?.content
    }
    missing.value = found === undefined
    content.value =
      found ?? (target.kind === 'note' ? '（该笔记已被删除）' : target.kind === 'todo' ? '（该待办已被删除）' : '（该条目已被删除）')
    logger.info('pinned', `已加载置顶内容 ${label}`)
  } catch (error) {
    logger.error('pinned', '加载置顶内容失败', error)
  }
}

/** 透明度实时作用于整个窗口。 */
watch(opacity, (value) => {
  document.documentElement.style.opacity = String(value / 100)
})

async function close(): Promise<void> {
  try {
    await api.windows.pinClose(label)
  } catch (error) {
    logger.error('pinned', '关闭置顶窗失败', error)
  }
}

/** 双击进入编辑态：窗口放大（Rust）、文本域聚焦且光标置末（原型 prompt 的替代）。 */
async function beginEdit(): Promise<void> {
  if (editing.value || missing.value) return
  draft.value = content.value
  editing.value = true
  logger.info('pinned', `进入编辑态 ${label}`)
  try {
    await api.windows.pinSetEditing(label, true)
  } catch (error) {
    logger.error('pinned', '切换编辑态失败', error)
  }
  await nextTick()
  const el = editor.value
  if (el) {
    el.focus()
    el.setSelectionRange(el.value.length, el.value.length)
  }
}

/** 退出编辑态并还原窗口尺寸。 */
async function endEdit(): Promise<void> {
  editing.value = false
  try {
    await api.windows.pinSetEditing(label, false)
  } catch (error) {
    logger.error('pinned', '还原窗口尺寸失败', error)
  }
}

/** 按 kind 回写：笔记先取原字段再保存；待办保留其余字段；剪贴板直接更新。 */
async function writeBack(kind: 'note' | 'todo' | 'clip', id: string, next: string): Promise<void> {
  if (kind === 'note') {
    const note: Note | null = await api.notes.get(id)
    if (!note) throw new Error('该笔记已被删除')
    await api.notes.save({
      id,
      content: next,
      tags: note.tags,
      editorMode: note.editor_mode,
      mindmapData: note.mindmap_data,
      draft: false,
    })
    return
  }
  if (kind === 'todo') {
    const todo: Todo | undefined = (await api.todos.list()).find((t) => t.id === id)
    if (!todo) throw new Error('该待办已被删除')
    await api.todos.save({
      id,
      content: next,
      dueAt: todo.due_at,
      remindOffsetMinutes: todo.remind_offset_minutes,
      remindDesktop: todo.remind_desktop,
      remindEmail: todo.remind_email,
      repeatRule: todo.repeat_rule,
      priority: todo.priority,
      remark: todo.remark,
      tags: todo.tags,
      parentId: todo.parent_id,
      allowPast: true,
    })
    return
  }
  await api.clipboard.update(id, next)
}

let saving = false

/** Ctrl+Enter / 失焦保存：空内容或未改动只退出编辑态，不写库。 */
async function save(): Promise<void> {
  if (!editing.value || saving) return
  const target = parsed.value
  const next = draft.value
  if (!target || !next.trim() || next === content.value) {
    if (!next.trim()) toast('内容为空，未保存')
    await endEdit()
    return
  }
  saving = true
  logger.info('pinned', `回写 ${target.kind} ${target.id}，长度 ${next.length}`)
  try {
    await writeBack(target.kind, target.id, next)
    content.value = next
    toast('已同步回数据库 ✔')
    await endEdit()
  } catch (error) {
    logger.error('pinned', '回写失败', error)
    toast(String(error))
    // 保存失败保留编辑态让用户改，但失焦触发的保存不应反复弹 toast：重新聚焦。
    editor.value?.focus()
  } finally {
    saving = false
  }
}

/** Esc 放弃：丢弃草稿、还原尺寸。 */
async function cancelEdit(): Promise<void> {
  if (!editing.value) return
  logger.info('pinned', '放弃编辑')
  draft.value = content.value
  await endEdit()
}

onMounted(load)
</script>

<template>
  <div id="pinnedWindow" class="glass">
    <!-- 仅标题行可拖拽：drag 区域会被子元素继承，放在根元素上会让整窗按钮全部点不动 -->
    <div class="pinned-header" data-tauri-drag-region>
      <span><span class="ix">📌</span> 置顶</span>
      <span class="pinned-close no-drag" title="关闭" @click="close">✕</span>
    </div>

    <textarea
      v-if="editing"
      ref="editor"
      v-model="draft"
      class="pinned-editor no-drag"
      spellcheck="false"
      placeholder="编辑内容…（Ctrl+Enter 保存 · Esc 放弃）"
      @keydown.ctrl.enter.prevent="save"
      @keydown.esc.prevent="cancelEdit"
      @blur="save"
    />
    <div v-else class="pinned-body no-drag" title="双击编辑" @dblclick="beginEdit" v-html="html" />

    <div class="pinned-footer no-drag">
      <span>透明度</span>
      <input v-model.number="opacity" type="range" min="30" max="100" />
    </div>

    <ToastHost />
  </div>
</template>
```

- [x] **Step 2: `windows.rs` 宽度 230 → 220**

第 20 行 `const PINNED_SIZE: (f64, f64) = (230.0, 150.0);` 替换为：

```rust
/// 置顶浮窗初始尺寸（逻辑像素）：宽对齐原型 #pinnedWindow 的 220px（spec 4B #25）。
const PINNED_SIZE: (f64, f64) = (220.0, 150.0);
```

- [x] **Step 3: 校验并提交**

```bash
pnpm typecheck && pnpm format:check
cd src-tauri && cargo fmt --check && cargo check && cd ..
git add src/windows/Pinned/PinnedApp.vue src-tauri/src/app/windows.rs
git commit -m "feat(pinned): 置顶浮窗双击编辑回写"
```

---

## Task 8: 主窗口删除确认改卡片下方兜底，`anchorBeside` 补 `above` 分支（D29）

**Files:**
- Modify: `src/utils/anchor.ts:1-9,30-35,80-88`
- Modify: `src/utils/anchor.test.ts:43-66`
- Modify: `src/components/base/CardConfirm.vue:9-16,29-32,164`
- Modify: `src/components/card/TodoTree.vue:36-37`
- Modify: `src/styles/extensions.css:112-122`（§3 `#cardConfirm.below .cc-caret` 之后）
- Modify: `src/windows/Main/NotesView.vue:197`、`ClipsView.vue:116`、`DayView.vue:349`、`TodosView.vue:213`

**Interfaces:**
- Produces：`AnchorResult.placement: 'right' | 'left' | 'center' | 'below' | 'above'`；`fallback: 'below'` 下方放不下时返回 `'above'`（`caretY` 不再使用）；`CardConfirm` 对 `above` 加类 `.above`；`extensions.css` §3 `#cardConfirm.above .cc-caret` 箭头朝下贴底缘。`TodoEditorPanel` 只检查 `placement === 'left'`，联合类型扩大不影响它。

- [x] **Step 1: 先改测试**

`src/utils/anchor.test.ts` 第 43 行用例名改为 `'anchorBeside：两侧都不够，fallback=below → 卡片下方 +8、左缘对齐；下方放不下则改到上方（placement=above）'`；第 60–65 行 `above` 断言改为：

```ts
  // 卡片贴底：下方 + 8 放不下 → 放到卡片上方 −8，placement 改为 'above'（D29：箭头朝下）
  const above = anchorBeside(
    { left: 20, top: 520, width: 440, height: 70 },
    opts({ viewport: { width: 480, height: 600 }, fallback: 'below' }),
  )
  assert.equal(above.placement, 'above')
  assert.equal(above.left, 20)
  assert.equal(above.top, 452) // 520 − 60 − 8
```

Run: `pnpm test:unit 2>&1 | grep -E "not ok|above|^# fail"`
Expected: 该用例 `not ok`（实际 `'below'`）。

- [x] **Step 2: `anchor.ts` 加 `above`**

头注释第 5–6 行 `'below' 是 spec D27 的卡片下方（箭头朝上、左缘对齐，\n * 下方也放不下时改到卡片上方）` 改为 `'below' 是 spec D27 的卡片下方（箭头朝上、左缘对齐），\n * 下方也放不下时改到卡片上方并返回 'above'（4B D29：箭头朝下）`。

第 30–35 行 `AnchorResult` 替换为：

```ts
export interface AnchorResult {
  left: number
  top: number
  placement: 'right' | 'left' | 'center' | 'below' | 'above'
  /** 箭头相对浮层顶部的纵向位置（px）；placement = 'below' / 'above' 时箭头改为朝上 / 朝下贴边，此值不再使用。 */
  caretY: number
}
```

第 80–88 行 `else {…}` 分支替换为：

```ts
  } else {
    // D27：卡片下方、左缘对齐；下方放不下则改到卡片上方（D29：placement = 'above'，箭头朝下）。
    left = clamp(anchor.left, VIEWPORT_MARGIN, maxLeft)
    let top = bottom + BELOW_GAP
    let vertical: AnchorResult['placement'] = 'below'
    if (top > maxTop) {
      top = anchor.top - H - BELOW_GAP
      vertical = 'above'
    }
    top = clamp(top, VIEWPORT_MARGIN, maxTop)
    return { left, top, placement: vertical, caretY: clamp(centerY - top, CARET_MIN, H - CARET_MIN) }
  }
```

Run: `pnpm test:unit 2>&1 | grep -E "^# (pass|fail)"`
Expected: `# fail 0`。

- [x] **Step 3: `CardConfirm.vue` `.above` 类与注释**

头注释第 11–12 行 `两侧都不够按 \`fallback\`：主窗口 'center'、\n *   面板 'below' = 卡片下方箭头朝上，spec D27）；` 改为 `两侧都不够按 \`fallback\`（默认 'below'：卡片下方箭头朝上，\n *   下方放不下翻到上方 \`.above\` 箭头朝下——主窗口与面板一致，spec D27 / 4B D29）；`。props 第 29 行注释改为 `/** 左右都放不下时的兜底：'center' 原型 / 'below' D27（主窗口与面板都用它，D29）。 */`。

第 164 行 `:class="{ flip: position.placement === 'left', below: position.placement === 'below' }"` 改为：

```vue
      :class="{ flip: position.placement === 'left', below: position.placement === 'below', above: position.placement === 'above' }"
```

`src/components/card/TodoTree.vue` 第 36 行注释改为 `/** 删除确认浮层左右都放不下时的兜底：默认 'below'（D27 / D29，主窗口与面板一致）。 */`。

- [x] **Step 4: `extensions.css` §3 追加朝下箭头**

`#cardConfirm.below .cc-caret {…}`（第 115–122 行）之后追加：

```css
/* D29：下方也放不下翻到卡片上方——箭头贴底缘朝下，可见边改成下 + 右两条。 */
#cardConfirm.above .cc-caret {
  top: auto;
  bottom: calc(-1 * var(--sp-sm));
  left: var(--sp-xl);
  right: auto;
  border: 1px solid rgba(255, 95, 87, 0.5);
  border-left: none;
  border-top: none;
  transform: rotate(45deg);
}
```

- [x] **Step 5: 四处主窗口视图删兜底传值**

- `src/windows/Main/NotesView.vue` 第 197 行 `      fallback="center"` 删除；
- `src/windows/Main/ClipsView.vue` 第 116 行 `      fallback="center"` 删除；
- `src/windows/Main/DayView.vue` 第 349 行 `      fallback="center"` 删除；
- `src/windows/Main/TodosView.vue` 第 213 行 `      confirm-fallback="center"` 删除（`TodoTree` 默认 `confirmFallback: 'below'`，`CardConfirm` 默认 `fallback: 'below'`，删后即卡片下方）。

- [x] **Step 6: 校验并提交**

```bash
pnpm test:unit 2>&1 | grep -E "^# (pass|fail)" && pnpm sync:styles --check && pnpm typecheck && pnpm format:check && (grep -rn 'fallback="center"' src || echo "无 center 兜底残留")
pnpm exec vite build 2>&1 | grep -E "built in|error"
git add src/utils/anchor.ts src/utils/anchor.test.ts src/components/base/CardConfirm.vue src/components/card/TodoTree.vue src/styles/extensions.css src/windows/Main/NotesView.vue src/windows/Main/ClipsView.vue src/windows/Main/DayView.vue src/windows/Main/TodosView.vue
git commit -m "fix(ui): 主窗口删除确认改卡片下方兜底，anchorBeside 补 above 分支"
```

---

## Task 9: 实机验收与记录

**Files:**
- Create: `docs/superpowers/plans/2026-09-12-prototype-realign-phase4b-acceptance.md`
- Modify: spec §9（`docs/superpowers/specs/2026-09-12-prototype-realign-phase4b-island-design.md:154-158`）；必要时 `src/styles/extensions.css`（§10 末尾追加「阶段四 B 验收补丁」小节并注明原因）

- [x] **Step 1: 静态检查全绿**

Run: `pnpm sync:styles --check && pnpm typecheck && pnpm test && pnpm format:check && (cd src-tauri && cargo fmt --check && cargo check && cargo test) && pnpm exec vite build`
Expected: 全部通过；记录 `test:unit` 通过数（含新增 `pickIslandTodos` 4 例、`anchorBeside` above 1 例）、`cargo test` 通过数（含 `v7_migration_bumps_version_and_is_idempotent`、改断言的 `island_clamp_bounds`）与 `island-*.js` / `pinned-*.js` / `reminder-*.js` 体积。
Run: `grep -rn "island\.css\|pickTodayTodos\|island-line\|island-row\|rgba(24, 26, 40\|fallback=\"center\"" src || echo "无残留"`
Expected: `无残留`。
Run: `grep -n "═══ 8\." src/styles/extensions.css || echo "§8 已删除"`
Expected: `§8 已删除`。

- [x] **Step 2: 实机（`pnpm tauri:dev` 后台；打字机 + 深色各一轮；按记忆「用户在本机时禁用键鼠自动化」——先问再接管键鼠，被拒则只做只读检查与用户口述）按 spec §6 十项逐条验证并截图**

1. 灵动岛 DOM 与轮播：DevTools 确认 `#dynamicIsland.glass > .di-ticker > .di-track > .di-item`，末尾有副本行；≥2 条待办时每条停留 3 秒、0.45s 上滚、滚到副本后复位无跳变；1 条静止；空态文案随「播放范围」切换（仅当天 → 「今日待办已全部完成 🎉」，全部 → 「没有未完成待办 🎉」）。
2. 数据口径：造一个子任务 → 岛内显示「父 / 子」；切 scope=all 后非当天条目带 `.di-date` `MM-DD` 徽章；逾期条目排最前且 `.di-time.ovd` 红色、文案「逾期 HH:mm」；主窗口灵动岛页预览与岛内第一条一致。
3. 设置即时生效：停留秒数（改 1 与 10）、透明度（底色变化、文字不变）、宽 / 高（窗口尺寸随之变化）、流光（`.di-glow` 旋转描边）、悬停穿透（悬停不撑高、轮播不停；关闭后恢复撑高）、播放范围；**启用开关关→开，灵动岛重建且主窗口不冻结**（D34）。
4. 手柄拖拽：悬停岛右下角出现手柄；拖拽后松手，岛宽对称变化、高变化；库值（灵动岛页宽 / 高数值）同步；越界拖到 200 / 480 / 32 / 56 停止；点击穿透开启时手柄不出现。
5. 提醒岛内呈现：造一个 1 分钟后提醒的待办 → 到期时岛切换为「⏰ 提醒：…」（图标脉冲、金色文字），6s 后恢复轮播；再造一次，点击 alert 立即恢复；提醒卡片同时右上角从右滑入（60px 回弹），关闭 / 顺延时右滑退场后关窗。
6. 面板展开淡出：感应区 3 秒或 Alt+Space 呼出面板 → 岛 `.di-fade` 淡出；收起 → 恢复。
7. 全屏自动隐藏：运行一个全屏 D3D 应用或 PowerPoint 放映（或浏览器视频全屏）→ ~1s 内岛隐藏，日志 `[island] 全屏 enter`；退出 → 恢复，`[island] 全屏 exit`；关闭「全屏自动隐藏」后重复，岛不隐藏；记录三种场景各自是否命中（`QUNS_BUSY` 对视频全屏的覆盖情况）。
8. 感应区指示器：打字机 / 深色下悬停顶部感应区，指示器底色 / 边框随主题、文字可读；§8 删除后灵动岛文字在两主题下可读（`.glass` 底色已按主题）。
9. 置顶浮窗：置顶一条笔记 / 待办 / 粘贴板；双击 → 窗口放大、文本域聚焦光标在末尾；改内容 Ctrl+Enter → toast「已同步回数据库 ✔」、窗口还原、主窗口对应列表刷新；再双击改内容按 Esc → 内容不变；失焦保存同样回写；已完成待办改内容 → toast 后端错误「已完成待办仅允许修改备注」；笔记回写后标签 / 导图数据未丢；浮窗宽 220。
10. 主窗口删除确认：笔记 / 粘贴板 / 待办 / 日期详情四页点 ✕ → 浮层在卡片下方、箭头朝上、左缘对齐，不盖正文；把列表滚到末张卡贴底再点 ✕ → 浮层翻到卡片上方、箭头朝下（`.above`）；面板末张卡同样箭头朝下。

- [x] **Step 3: 写验收记录并回填 spec §9**

验收记录按 `2026-09-12-prototype-realign-phase4a-acceptance.md` 的结构：环境 / 方法 → 自动化校验表 → 逐项比对表（打字机、深色各一列，十项）→ 发现的问题与补丁清单 → 未验证项。spec §9 三条「待填」改为：轮播回环与手柄拖拽观感结论（复位是否可见跳变、拖拽中窗口不随动只在松手后变化的观感是否可接受）、全屏探测命中情况（游戏 / PPT 放映 / 视频全屏三种各自是否触发 `QUNS_RUNNING_D3D_FULL_SCREEN` / `QUNS_BUSY`）、补丁清单（含 `color-mix` 透明度在 WebView2 下是否生效、`.glass` 圆角裁剪是否干净）。

- [x] **Step 4: 提交**

```bash
git add docs/superpowers/plans/2026-09-12-prototype-realign-phase4b-acceptance.md docs/superpowers/specs/2026-09-12-prototype-realign-phase4b-island-design.md src/styles/extensions.css
git commit -m "docs(design): 阶段四B 实机验收记录"
```

---

## 自检记录

- **Spec 覆盖**：§3 差异表 #1 → Task 4；#2 → Task 4（`step` / `resetTicker`）；#3 → Task 3；#4 → Task 3（`emptyText`）+ Task 4；#5 → Task 2（watcher 守卫）+ Task 4（前端守卫）；#6 → Task 2（`island_resize`）+ Task 4（`startResize`）；#7 → Task 4（`panelOpen`）；#8 → Task 4（`showAlert`）；#9 → Task 1（`island_glow`）+ Task 4（class）；#10 → Task 1（`island_auto_hide`）+ Task 2（`foreground_is_fullscreen` / watcher）；#11 → Task 1（`island_scope`）+ Task 5（select）；#12 → Task 1（`island_clamp`）+ Task 5（`ISLAND_LIMITS`）；#13 / #14 保留（Task 4 §10 `color-mix` 只透底色；`di-pass-click` 类照加）；#15 / #16 → Task 5；#17 → Task 1（D34）；#18 → Task 6；#19 → Task 6；#20 由 #8 覆盖；#21 / #22 / #24 保留无任务；#23 → Task 7；#25 → Task 7。§4.1 → Task 3 / 4；§4.2 → Task 1 / 2；§4.3 → Task 6；§4.4 → Task 6；§4.5 → Task 7；§4.6 → Task 8；§5 测试 → Task 1（v7 + clamp）、Task 3（4 例）、Task 8（+1）；§6 → Task 9；§7 九个批次 ↔ 九个 Task 一一对应；§8 风险「`.glass` 圆角」→ Task 4 `window-fit.css` `clip-path`、「轮播与 `--cap-h` 冲突」→ Task 4 展开态 `v-if` 隐藏 `.di-ticker`、「`QUNS_NOT_PRESENT`」→ Task 2 `matches!` 只认三种状态、「置顶回写丢标签」→ Task 7 `notes.get` 先取原字段。
- **类型与函数名一致性**：`IslandScope` 在 Task 1 `domain.ts` 定义，Task 3（`pickIslandTodos` 参数）/ Task 5（select 断言）引用；`Settings` 四键名 `island_scope / island_pass_hover / island_glow / island_auto_hide` 在 Rust `models.rs`、`settings.rs` 键名、TS `domain.ts`、`useData.ts`、`IslandPageView` / `IslandApp` 全程同名；`IslandFlags { pass_hover, auto_hide }` 在 Task 2 `state.rs` 定义，`ipc.rs` 以 `crate::app::state::IslandFlags` 构造、`hotzone_watcher` 读 `.pass_hover` / `.auto_hide`；`windows::island_resize(app, width, height)` ↔ `ipc::island_resize(app, width, height)` ↔ `api.island.resize(width, height)` ↔ invoke 参数 `{ width, height }`；`IslandTodoRow { id, text, date, time, overdue, priority, todo }` 在 Task 3 定义，`today-todos.ts` 与 `IslandPageView` 同名取字段；`IslandItem.date / overdue / priority` 在 Task 3 加、Task 4 `rows` 消费；`enter / exit` 的 `duration: 'slow' | 'base'` 与 `presets.ts:26` 既有档位一致；`AnchorResult.placement` 的 `'above'` 在 Task 8 定义、`CardConfirm` 同名判断；`PINNED_SIZE` 是 `windows.rs:20` 实际常量名（spec / 任务书写作 `PIN_WIDTH`）。
- **占位扫描**：无 TBD / 「类似于」/ 「适当处理」；Task 9 按要求只列步骤不含代码。
- **自检发现并就地修正**：
  1. spec §4.2 只说「`AppState` 增 `island_pass_hover` 缓存」，但 watcher 每 ~1s 还要读 `island_auto_hide`，若走锁库读 settings 会和 IPC 抢同一把 Store 锁——Task 2 改为缓存一个 `IslandFlags { pass_hover, auto_hide }` 结构，`with_store` 启动初始化、`settings_save` 刷新。
  2. spec §4.2 的 `settings_save` 改 async 后，`island_resize` 也写了 `save_settings`，若它也经 `settings_save` 会顺带移感应区 / 面板并可能建窗——Task 2 `windows::island_resize` 直接 `store.save_settings` 并自行 `place_island` + emit，保持同步命令（任务书约定）。
  3. 新增 `Settings` 必填键会让 `useData.ts` 的 `DEFAULT_SETTINGS` 字面量类型检查失败（spec 未提）——Task 1 Step 8 同步补四键并把默认宽 / 停留改为 320 / 3。
  4. spec §3 #4 空态文案按 scope 二选一，但 `IslandPlugin.emptyText` 是插件级常量无法感知 scope——Task 3 把内置插件的 `emptyText` 改为原型 today 文案，Task 4 `IslandApp.emptyText` 在 `island_scope === 'all'` 时覆盖为「没有未完成待办 🎉」。
  5. spec §3 #3 写「today 仅当天」（原型 `inScope`），现状 `pickTodayTodos` 把此前逾期的也算进「今天」——**控制者裁定沿用现状口径**（当天 + 此前逾期），与面板待办页 / 托盘一致，避免跨日逾期项从岛上消失；Task 3 测试 1 用「昨天 C」显式覆盖并断言其逾期置顶，Task 9 验收第 2 项核对。差异记入验收记录。
  6. 原型手柄拖拽时 `island.style.width/height` 即时改变，但真实窗口尺寸只有后端能改，拖拽中往外拉会被窗口裁掉——Task 4 拖拽中只更新 `--cap-h` / 行高的本地值、松手后 `api.island.resize` 落库并重排，观感差异记入 Task 9 验收结论（spec §9 第一条）。
  7. spec §4.1 模板给 `.di-item` 用 `:key="row.key + ':' + i"`，而副本行与第一条同 id 会重复 key——Task 4 `rows` 给副本 key 加 `:clone` 后缀，模板直接 `:key="row.key"`。
  8. spec §4.1 说「展开态 `v-if` 隐藏 `.di-ticker`」（§8 风险），但手柄 `.di-resizer` 在展开态仍会叠在详情上——Task 4 手柄也加 `!expanded` 条件。
  9. spec §4.3 写 `.hotzone-indicator` 文字改 `var(--text)` / `var(--text-dim)`，实际 §6 指示器文字只有 `.hotzone-indicator-label` 且已是 `var(--text-dim)`——Task 6 只改底色与边框两条声明，不重复。
  10. spec §4.4 `ReminderApp` 三条关闭路径（关闭 / 顺延 / 完成）各自调 `reminderClose`，若只在 `dismiss` / `snooze` 末尾包退场会写三遍——Task 6 抽 `closeCard()` 并加 `closing` 门闩防重复触发。
  11. spec §4.5 置顶回写待办写「`api.todos.save({...原字段, content: draft})`」，但 `TodoInput` 是 camelCase 且与 `Todo` 字段名不同（`dueAt` / `remindOffsetMinutes` …）——Task 7 `writeBack` 逐字段映射并传 `allowPast: true`（编辑既有事项不受「不得早于当前」限制）。
  12. `PinnedApp` 没有 `<ToastHost />`，spec 的 toast「已同步回数据库 ✔」无处显示——Task 7 模板加 `<ToastHost />`。
  13. spec §4.6 只说「`CardConfirm` 对 `above` 加类」，`anchor.test.ts` 第 4 例现有断言 `above.placement === 'below'` 会直接失败——Task 8 Step 1 先改该断言为 `'above'`（TDD 红灯），而非另加一例后留下矛盾断言。
  14. 任务书与 spec 写 `PIN_WIDTH` 230→220，`windows.rs` 实际常量是 `PINNED_SIZE: (f64, f64) = (230.0, 150.0)`——Task 7 按实际常量改宽度分量。
  15. spec §4.1 模板根元素 `:style="{ '--island-alpha': alpha, '--cap-h': capH + 'px' }"`，`.glass` 的 `--glass-bg` 是带自身 alpha 的 rgba，无法直接乘系数——Task 4 §10 用 `color-mix(in srgb, var(--glass-bg) calc(var(--island-alpha) * 100%), transparent)`（WebView2 / Chromium 111+ 支持），Task 9 验收核对。
  16. spec 未提 `island_clamp_bounds` 的 `island_geometry` 测试仍用 360 宽——该测试只测几何不经钳制，360 也在新范围内，不改。
