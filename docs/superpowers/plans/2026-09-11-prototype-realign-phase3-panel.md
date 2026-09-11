# 原型对齐 · 阶段三「呼出面板」实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把呼出面板（`src/windows/Panel/`）对齐原型 `docs/index.html#panel`：矢量三态圆点导航、Zen 专注模式、编辑器底栏与暂存文案、笔记「回显到面板」编辑态（兑现阶段二 D13）、粘贴板卡片（含来源应用）、待办页提示行、失焦收起触发源并存、Esc 链与切页副作用。

**Architecture:** 就地对齐现有组件，不重写数据流。后端只加三样东西：面板呼出「意图」槽位（`pending_panel_intent` JSON，取代单一 `pending_panel_page`）、Zen 窗口几何（`panel_set_zen` 把常驻面板窗口放大到光标屏工作区再还原）、粘贴板来源应用（v6 迁移 `source_app` 列 + Win32 前台进程名）。前端以 `PanelPageExpose` 契约让 `PanelApp` 通过实例引用驱动各插件页（回显 / 聚焦 / 关浮层 / 收面板通知），`NotePage` 新增回显编辑态并保证「编辑态不自动暂存、收面板即丢弃并恢复草稿」。

**Tech Stack:** Vue 3.5 `<script setup lang="ts">` · TypeScript 5.9 · Vite 7 · animejs 4.5 · Node 24（`node:test`）· Tauri 2 / Rust · rusqlite 0.37 · windows-sys 0.59

**设计文档：** `docs/superpowers/specs/2026-09-11-prototype-realign-phase3-panel-design.md`（下文简称 spec；差异表 §3、设计 §4、验收 §6、提交批次 §7）

## Global Constraints

- 唯一参考原型 `docs/index.html` + `docs/styles.css` + `docs/app.js`；生成层样式（`tokens.css` / `base.css` / `components.css` / `themes.css`）**不改**，`pnpm sync:styles --check` 必须始终通过；项目自有样式只进 `src/styles/extensions.css`（本阶段在 §3 补充、新开「§10 阶段三」分节）、`window-fit.css`、`motion.css`。
- 组件模板只用原型类名；不新增设计语言；颜色只用令牌。
- 已拍定决策（spec §2）：D15 Zen 用同一面板窗口放大；D16 回显编辑态完全按原型（先落库草稿 → 载入笔记 → 收面板丢弃）；D17 失焦触发 `blur` 与 `mouseleave` 并存；D18 保持 Enter 换行 / Mod+Enter 归档；D19 感应区阈值保持 3 秒；D20 来源应用采集前台进程名；D21 面板内弹窗结构留阶段四，本阶段只改副标题文案。
- 编码规范：Vue 一律 `<script setup lang="ts">`，禁止 `any`；关键节点走 `src/service/logger.ts`（前端）与 `eprintln!`（Rust，沿用现有风格）；Rust 新增项带 `///` 注释；每个文件头部有职责说明注释。
- **所有可能建窗或做 show / hide 序列的 Tauri 命令一律 `pub async fn`**（项目踩坑：同步命令建窗会与主线程事件循环互等死锁；`panel_open_note` 走 hide_main + panel_show 序列、`panel_set_zen` 走几何序列且退出后紧接 hide，均按 async 写）。
- 动效硬约束沿用阶段一（只动 transform / opacity；`.glass` 静止态无 transform；入场由 JS 驱动并可重播）；Zen 态跳过位移动画、暂停高度上报。
- 验证：每任务结束 `pnpm typecheck` 零错误；改动 Rust 时 `cargo fmt --manifest-path src-tauri/Cargo.toml --check && cargo check --manifest-path src-tauri/Cargo.toml && cargo test --manifest-path src-tauri/Cargo.toml`；改动纯函数时 `pnpm test:unit`；最后 `pnpm exec vite build`。
- 提交：中文提交信息，按 spec §7 的 8 个批次，每任务一次提交。

---

## File Structure

```
src-tauri/
├─ Cargo.toml                          windows-sys 增 Win32_UI_WindowsAndMessaging / Win32_System_Threading
└─ src/
   ├─ app/state.rs                     pending_panel_page → pending_panel_intent（JSON 意图槽位）
   ├─ app/windows.rs                   panel_show_page 写意图；新增 panel_open_note / panel_set_zen / PANEL_ZEN；panel_hide / panel_resize 感知 Zen
   ├─ ipc.rs                           panel_take_intent / panel_open_note(async) / panel_set_zen(async) / note_get
   ├─ main.rs                          命令注册
   ├─ platform.rs                      foreground_app_name / app_name_from_path(+单测)
   ├─ data/mod.rs                      v6 迁移 source_app(+单测)
   ├─ data/clipboard.rs                Capture.source_app、ROW_COLUMNS / row_entry / INSERT(+单测)
   ├─ data/notes.rs                    get_note(id) -> Option<Note>
   ├─ domain/models.rs                 ClipboardEntry.source_app
   └─ services/clipboard_watcher.rs    捕获时写入前台应用名
src/
├─ service/tauri.ts                    PanelIntent、panelTakeIntent / panelOpenNote / panelSetZen、notes.get
├─ typings/domain.ts                   ClipboardEntry.source_app
├─ panel-plugins/index.ts              dot → dotClass；PanelPageExpose 契约
├─ windows/panel.ts                    dataset.window = 'panel'
├─ windows/Panel/
│  ├─ PanelApp.vue                     圆点导航 / Zen / 意图消费 / pageRefs / 失焦触发源 / Esc 链 / 切页清确认 / 归档收起 / 呼出聚焦
│  ├─ NotePage.vue                     回显编辑态 / 暂存三段文案 / 标签预览 / zenExit / dismissOverlays
│  ├─ ClipPage.vue                     dismissOverlays
│  └─ TodoPage.vue                     提示行 / edit-repeat 接线 / dismissOverlays
├─ windows/Main/NotesView.vue          ✏️ 文本笔记 → panelOpenNote
├─ windows/Main/DayView.vue            ✏️ 文本笔记 → panelOpenNote
├─ windows/Main/SettingsView.vue       面板插件行不再引用 plugin.dot
├─ components/note/NoteEditModal.vue   删除
├─ components/card/ClipCard.vue        结构对齐原型 renderClips（card-close / 📌 / clip-from / 类型徽章 / 全 Icon）
├─ components/card/ClipArchiveCard.vue 加 clip-from
├─ components/card/TodoTree.vue        expose dismissConfirm
├─ components/tag/TagList.vue          prop moreAction: 'expand' | 'open'
└─ styles/
   ├─ extensions.css                   §3 补 .nav-dots 兜底令牌；删 §5 圆点两段
   ├─ motion.css                       删 .nav-dot 段
   └─ window-fit.css                   #panel.zen-mode 纵向撑满
docs/superpowers/plans/2026-09-11-prototype-realign-phase3-acceptance.md   验收记录（Task 8）
```

---

## Task 1: 后端 intent 槽位 + `panel_open_note` / `note_get` / `panel_set_zen` + 前端契约

**Files:**
- Modify: `src-tauri/src/app/state.rs:28-31,43,57-69`
- Modify: `src-tauri/src/app/windows.rs:698-709`（`panel_show_page`）、`:729-749`（`PANEL_LOGICAL_HEIGHT` 之后）、`:805-830`（`panel_hide` / `panel_resize`）
- Modify: `src-tauri/src/ipc.rs:626-630`（`panel_take_page`）、`:181-184` 之后（`note_get`）
- Modify: `src-tauri/src/data/notes.rs:38-45` 之后
- Modify: `src-tauri/src/main.rs:141-142`
- Modify: `src/service/tauri.ts:64-65,94-100`
- Modify: `src/windows/Panel/PanelApp.vue:328-335`

**Interfaces:**
- Consumes: `windows::panel_show / hide_main / place_panel / panel_logical_height / cursor_monitor / WorkArea::of`（既有）；`Store::note(id)`（既有）。
- Produces:
  - Rust：`AppState::set_pending_panel_intent(Option<String>)`、`AppState::take_pending_panel_intent() -> Option<String>`；`windows::panel_open_note(app, note_id: &str) -> Result<(), String>`；`windows::panel_set_zen(app, on: bool) -> Result<(), String>`；`windows::PANEL_ZEN: AtomicBool`；`Store::get_note(&self, id) -> Result<Option<Note>, String>`；命令 `panel_take_intent() -> Option<String>`、`panel_open_note(note_id: String)`（async）、`panel_set_zen(on: bool)`（async）、`note_get(id: String) -> Result<Option<Note>, String>`。
  - TS：`export interface PanelIntent { page: string; noteId?: string }`；`api.windows.panelTakeIntent(): Promise<PanelIntent | null>`、`api.windows.panelOpenNote(noteId: string)`、`api.windows.panelSetZen(on: boolean)`；`api.notes.get(id: string): Promise<Note | null>`。

- [ ] **Step 1: `state.rs` 改名为意图槽位**

把第 28–31 行的字段与注释替换为：

```rust
    /// 面板下一次显示时应执行的呼出意图（JSON 字符串）。
    ///
    /// 形如 `{"page":"todo"}`（灵动岛点击）或 `{"page":"note","noteId":"…"}`（主窗口 ✏️ 回显）。
    /// 由 `windows::panel_show_page` / `panel_open_note` 写入，面板收到 panel-shown 后取走。
    /// 不直接向隐藏的面板 emit：WebView2 在窗口 hide 后被挂起，此时投递的事件会丢。
    pub pending_panel_intent: Mutex<Option<String>>,
```

第 43 行 `pending_panel_page: Mutex::new(None),` 改为 `pending_panel_intent: Mutex::new(None),`。

第 57–69 行两个方法替换为：

```rust
    pub fn set_pending_panel_intent(&self, intent: Option<String>) {
        if let Ok(mut slot) = self.pending_panel_intent.lock() {
            *slot = intent;
        }
    }

    /// 取走并清空待执行的呼出意图。
    pub fn take_pending_panel_intent(&self) -> Option<String> {
        self.pending_panel_intent
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }
```

- [ ] **Step 2: `windows.rs` 意图写入、`panel_open_note`、Zen**

文件顶部 `use tauri::{…}` 之后追加：

```rust
use std::sync::atomic::{AtomicBool, Ordering};
```

把第 698–709 行 `panel_show_page` 替换为：

```rust
/// 呼出面板并要求它切到指定插件页（灵动岛点击使用）。
///
/// 意图 `{"page":…}` 先存进 AppState，再 show 面板；面板在 panel-shown 事件后主动来取。
/// 顺带广播一次 PANEL_NAVIGATE，面板若已可见能立即响应。
pub fn panel_show_page(app: &AppHandle, page: &str) -> Result<(), String> {
    eprintln!("[panel] 带页呼出 page={page}");
    let intent = serde_json::json!({ "page": page }).to_string();
    app.state::<AppState>().set_pending_panel_intent(Some(intent));
    panel_show(app)?;
    let _ = app.emit(events::PANEL_NAVIGATE, page.to_string());
    Ok(())
}

/// 把一条笔记回显到面板编辑（主窗口笔记列表 / 日期详情的 ✏️，spec D16）。
///
/// 顺序与原型 `openNoteInPanel` 一致：先隐藏主窗口，再呼出面板；意图带 noteId，
/// 面板取走后切到笔记页并调用 `loadNote`。面板已可见时 `panel_show` 仍会广播 PANEL_SHOWN，
/// 前端每次都取意图，因此隐藏 / 可见两种情况走同一条路径。
pub fn panel_open_note(app: &AppHandle, note_id: &str) -> Result<(), String> {
    eprintln!("[panel] 回显笔记到面板 note_id={note_id}");
    let intent = serde_json::json!({ "page": "note", "noteId": note_id }).to_string();
    app.state::<AppState>().set_pending_panel_intent(Some(intent));
    hide_main(app)?;
    panel_show(app)
}
```

在第 739 行 `panel_logical_height()` 函数之后（`place_panel` 之前）插入：

```rust
/// 面板是否处于 Zen 专注模式（spec D15）：窗口铺满光标所在屏工作区，退出时按逻辑高度还原。
static PANEL_ZEN: AtomicBool = AtomicBool::new(false);

/// 进入 / 退出 Zen 专注模式。
///
/// 进入：记录状态，把面板窗口放大到**光标所在屏**的工作区（物理像素，不遮任务栏）；
/// 退出：清状态，按 `PANEL_LOGICAL_HEIGHT` 与当前唤出位置重新 `place_panel`。
/// 只取进入时的光标屏；多屏下退出时按当时光标屏重摆（spec §8 风险项的应对）。
pub fn panel_set_zen(app: &AppHandle, on: bool) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let work = WorkArea::of(&monitor);
    PANEL_ZEN.store(on, Ordering::SeqCst);
    if on {
        eprintln!(
            "[panel] 进入 Zen，铺满工作区 left={} top={} w={} h={}",
            work.left, work.top, work.width, work.height
        );
        let _ = panel.set_size(PhysicalSize::new(
            work.width.round() as u32,
            work.height.round() as u32,
        ));
        let _ = panel.set_position(PhysicalPosition::new(
            work.left.round() as i32,
            work.top.round() as i32,
        ));
    } else {
        let position = app
            .state::<AppState>()
            .lock_store()?
            .get_settings()?
            .panel_position()
            .clone();
        let height = panel_logical_height();
        eprintln!("[panel] 退出 Zen，按逻辑高度 {height} 还原到 {position} 边");
        place_panel(&panel, &work, height, &position);
    }
    Ok(())
}
```

把第 805–812 行 `panel_hide` 替换为：

```rust
/// 收起面板。Zen 态先还原窗口尺寸（原型 hidePanel 的 setZenMode(false)），否则下次呼出仍是全屏窗口。
pub fn panel_hide(app: &AppHandle) -> Result<(), String> {
    if PANEL_ZEN.load(Ordering::SeqCst) {
        if let Err(error) = panel_set_zen(app, false) {
            eprintln!("[panel] 收起前退出 Zen 失败: {error}");
        }
    }
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.hide();
    }
    let _ = app.emit(events::PANEL_HIDDEN, ());
    Ok(())
}
```

把第 814–830 行 `panel_resize` 替换为：

```rust
/// 面板高度自适应（前端测量内容后调用）。
///
/// Zen 态只记录逻辑高度不动窗口：窗口此刻铺满工作区，退出 Zen 时按记录值还原。
pub fn panel_resize(app: &AppHandle, height: f64) -> Result<(), String> {
    let panel = app.get_webview_window("panel").ok_or("面板窗口未初始化")?;
    let clamped = height.clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT);
    if let Ok(mut h) = PANEL_LOGICAL_HEIGHT.lock() {
        *h = clamped;
    }
    if PANEL_ZEN.load(Ordering::SeqCst) {
        return Ok(());
    }
    let monitor = cursor_monitor(app).ok_or("未找到可用显示器")?;
    let position = app
        .state::<AppState>()
        .lock_store()?
        .get_settings()?
        .panel_position()
        .clone();
    place_panel(&panel, &WorkArea::of(&monitor), clamped, &position);
    Ok(())
}
```

- [ ] **Step 3: `notes.rs` 暴露 `get_note`**

在第 45 行 `active_draft` 之后插入：

```rust
    /// 按 id 读取笔记（含标签）；不存在返回 `None` 而非错误，供面板回显前判断。
    pub fn get_note(&self, id: &str) -> Result<Option<Note>, String> {
        let exists = self
            .db
            .query_row("SELECT 1 FROM notes WHERE id=?", [id], |_| Ok(()))
            .optional()
            .map_err(db_err)?
            .is_some();
        if !exists {
            return Ok(None);
        }
        self.note(id).map(Some)
    }
```

- [ ] **Step 4: `ipc.rs` 命令**

在第 184 行 `note_draft` 之后插入：

```rust
/// 按 id 读取单条笔记（面板回显使用）；不存在返回 None。
#[tauri::command]
pub fn note_get(state: State<'_, AppState>, id: String) -> Result<Option<Note>, String> {
    state.lock_store()?.get_note(&id)
}
```

把第 626–630 行 `panel_take_page` 替换为：

```rust
/// 面板显示后取走本次呼出意图（JSON：`{"page":…,"noteId"?:…}`）；没有则返回 None。
#[tauri::command]
pub fn panel_take_intent(state: State<'_, AppState>) -> Option<String> {
    state.take_pending_panel_intent()
}

/// 把笔记回显到面板编辑：隐藏主窗口 → 呼出面板 → 面板取意图后载入笔记。
///
/// 不建窗，但走 hide_main + panel_show 的窗口序列；与 `launcher_show` 同款用 async，
/// 避免同步命令占住主线程与事件循环互等。
#[tauri::command]
pub async fn panel_open_note(app: AppHandle, note_id: String) -> Result<(), String> {
    windows::panel_open_note(&app, &note_id)
}

/// 进入 / 退出 Zen 专注模式（面板窗口放大到工作区 / 还原）。
///
/// 几何序列（set_size + set_position）且退出后前端紧接可能调用 panel_hide，按 async 写以免阻塞主线程。
#[tauri::command]
pub async fn panel_set_zen(app: AppHandle, on: bool) -> Result<(), String> {
    windows::panel_set_zen(&app, on)
}
```

- [ ] **Step 5: `main.rs` 注册**

第 111 行 `ipc::note_draft,` 之后加 `ipc::note_get,`；第 142 行 `ipc::panel_take_page,` 替换为：

```rust
            ipc::panel_take_intent,
            ipc::panel_open_note,
            ipc::panel_set_zen,
```

- [ ] **Step 6: Rust 校验**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check && cargo check --manifest-path src-tauri/Cargo.toml && cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | grep "^test result"`
Expected: fmt 干净、check 通过（无新增警告）、既有测试全部通过。

- [ ] **Step 7: `tauri.ts` 契约**

在 `LauncherStatus` 接口之后追加：

```ts
/** 面板呼出意图（对应 Rust `pending_panel_intent` 的 JSON）：切到哪一页，可选要回显的笔记 id。 */
export interface PanelIntent {
  page: string
  noteId?: string
}

/** 解析后端返回的意图 JSON；格式不合法时视为无意图。 */
function parsePanelIntent(raw: string | null): PanelIntent | null {
  if (!raw) return null
  try {
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null) return null
    const record = parsed as Record<string, unknown>
    if (typeof record.page !== 'string') return null
    return { page: record.page, noteId: typeof record.noteId === 'string' ? record.noteId : undefined }
  } catch {
    return null
  }
}
```

把第 64–65 行替换为：

```ts
    /** 面板显示后取走本次呼出意图（灵动岛点击 / 主窗口 ✏️ 回显写入），无则 null。 */
    panelTakeIntent: (): Promise<PanelIntent | null> =>
      invoke<string | null>('panel_take_intent').then(parsePanelIntent),
    /** 把笔记回显到面板编辑：后端隐藏主窗口并呼出面板。 */
    panelOpenNote: (noteId: string) => invoke<void>('panel_open_note', { noteId }),
    /** 进入 / 退出 Zen 专注模式（面板窗口放大到工作区 / 还原）。 */
    panelSetZen: (on: boolean) => invoke<void>('panel_set_zen', { on }),
```

`notes` 分组第 96 行 `draft` 之后加：

```ts
    /** 按 id 读取单条笔记；不存在返回 null。 */
    get: (id: string) => invoke<Note | null>('note_get', { id }),
```

- [ ] **Step 8: `PanelApp.vue` 最小适配**

把第 331–334 行替换为：

```ts
    void api.windows
      .panelTakeIntent()
      .then((intent) => navigateTo(intent?.page))
      .catch((error) => logger.error('panel', '读取呼出意图失败', error))
```

（noteId 的消费在 Task 4 接入。）

- [ ] **Step 9: 校验并提交**

```bash
pnpm typecheck
git add src-tauri/src/app/state.rs src-tauri/src/app/windows.rs src-tauri/src/ipc.rs src-tauri/src/data/notes.rs src-tauri/src/main.rs src/service/tauri.ts src/windows/Panel/PanelApp.vue
git commit -m "feat(ipc): 面板 intent 槽位、panel_open_note / note_get / panel_set_zen 命令"
```

---

## Task 2: 粘贴板来源应用（v6 迁移 `source_app`）

**Files:**
- Modify: `src-tauri/Cargo.toml:47-53`
- Modify: `src-tauri/src/platform.rs`（文件末尾追加）
- Modify: `src-tauri/src/data/mod.rs:181-188`（迁移链）、`:289-306` 之后（`with_v6`）、`:532-716`（测试）
- Modify: `src-tauri/src/data/clipboard.rs:8-33,72-88`、文件末尾追加测试
- Modify: `src-tauri/src/domain/models.rs:30-45`
- Modify: `src-tauri/src/services/clipboard_watcher.rs:99-105,124-130`
- Modify: `src/typings/domain.ts:32-41`

**Interfaces:**
- Produces: `platform::foreground_app_name() -> Option<String>`、`platform::app_name_from_path(&str) -> Option<String>`；`data::clipboard::Capture.source_app: Option<String>`；`ClipboardEntry.source_app: Option<String>`（getter `source_app()`，序列化时 None 省略）；TS `ClipboardEntry.source_app?: string | null`。

- [ ] **Step 1: 写失败的单测（迁移 + 入库 + 纯函数）**

`src-tauri/src/data/mod.rs` 测试模块末尾（`default_settings_theme_is_typewriter` 之后）追加：

```rust
    /// 建一个 v5 状态的剪贴板表：没有 source_app 列，带一行旧数据。
    fn v5_clipboard_db() -> rusqlite::Connection {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE clipboard_entries (
               id TEXT PRIMARY KEY,
               content_type TEXT NOT NULL,
               content TEXT NOT NULL DEFAULT '',
               preview TEXT NOT NULL DEFAULT '',
               file_path TEXT,
               content_hash TEXT NOT NULL UNIQUE,
               pinned INTEGER NOT NULL DEFAULT 0,
               copied_at TEXT NOT NULL,
               modified_at TEXT NOT NULL,
               created_at TEXT
             );
             INSERT INTO clipboard_entries(id, content_type, content, preview, content_hash, copied_at, modified_at)
             VALUES('c1','text','旧条目','旧条目','h1','x','x');",
        )
        .unwrap();
        db.pragma_update(None, "user_version", 5).unwrap();
        db
    }

    fn clipboard_columns(store: &Store) -> Vec<String> {
        let mut stmt = store
            .db
            .prepare("PRAGMA table_info(clipboard_entries)")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn v6_migration_adds_source_app_and_keeps_old_rows_null() {
        let store = Store {
            db: v5_clipboard_db(),
            data_dir: std::path::PathBuf::from("."),
        };
        store.with_v6().unwrap();
        assert!(clipboard_columns(&store).iter().any(|c| c == "source_app"));
        // 旧行不回填，来源未知就是 NULL（前端据此不渲染来源）。
        let source: Option<String> = store
            .db
            .query_row(
                "SELECT source_app FROM clipboard_entries WHERE id='c1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(source, None);
        // 幂等：再跑一次不报错、不重复加列。
        store.with_v6().unwrap();
        assert_eq!(
            clipboard_columns(&store)
                .iter()
                .filter(|c| *c == "source_app")
                .count(),
            1
        );
    }
```

`src-tauri/src/data/clipboard.rs` 文件末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// 入库时写入来源应用，回读原样返回。
    #[test]
    fn insert_capture_persists_source_app() {
        let dir = std::env::temp_dir().join(format!("inkling-clip-v6-{}", std::process::id()));
        let store = Store::open(dir.clone()).unwrap();
        let entry = store
            .insert_capture(&Capture {
                content: "hello".into(),
                content_type: "text",
                preview: "hello".into(),
                file_path: None,
                hash: "hash-hello".into(),
                source_app: Some("Notepad".into()),
            })
            .unwrap()
            .unwrap();
        assert_eq!(entry.source_app().as_deref(), Some("Notepad"));
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

`src-tauri/src/platform.rs` 文件末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::app_name_from_path;

    /// 只取文件名并去掉 .exe（不区分大小写）；空路径与目录结尾返回 None。
    #[test]
    fn app_name_from_path_strips_directory_and_exe() {
        assert_eq!(
            app_name_from_path(r"C:\Windows\System32\notepad.exe").as_deref(),
            Some("notepad")
        );
        assert_eq!(
            app_name_from_path(r"C:\Program Files\App\Code.EXE").as_deref(),
            Some("Code")
        );
        assert_eq!(app_name_from_path("/usr/bin/vim").as_deref(), Some("vim"));
        assert!(app_name_from_path("").is_none());
        assert!(app_name_from_path(r"C:\dir\").is_none());
    }
}
```

- [ ] **Step 2: 运行，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "error\[|^test result" | head`
Expected: 编译错误（`with_v6` / `source_app` / `app_name_from_path` 不存在）。

- [ ] **Step 3: `Cargo.toml` 特性**

第 48–53 行替换为：

```toml
windows-sys = { version = "0.59", features = [
  "Win32_Foundation",
  "Win32_Graphics_Dwm",
  "Win32_Storage_FileSystem",
  "Win32_UI_Input_KeyboardAndMouse",
  # 粘贴板来源应用：前台窗口 → 进程 id → 可执行文件名
  "Win32_UI_WindowsAndMessaging",
  "Win32_System_Threading",
] }
```

- [ ] **Step 4: `platform.rs` 前台应用名**

在 `is_silent_start` 之后（测试模块之前）追加：

```rust
/// 当前前台应用的名字（可执行文件名去掉 `.exe`），用于标注粘贴板条目来源（spec D20）。
///
/// 只在 Windows 上实现：`GetForegroundWindow` → `GetWindowThreadProcessId` →
/// `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `QueryFullProcessImageNameW`。
/// 前台是本应用自身（面板里手动捕获）时返回 None——回声抑制之外再防一手；
/// 提权进程 `OpenProcess` 会失败，同样返回 None，前端不渲染来源。
pub fn foreground_app_name() -> Option<String> {
    foreground_app_path().and_then(|path| app_name_from_path(&path))
}

#[cfg(target_os = "windows")]
fn foreground_app_path() -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    // SAFETY: 全部是只读查询类 Win32 调用；进程句柄在本函数内打开并在返回前关闭，
    // 缓冲区由我们分配并把长度传给系统，系统只在长度范围内写入。
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 || pid == GetCurrentProcessId() {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            eprintln!("[platform] 打开前台进程 {pid} 失败（可能是提权进程），来源留空");
            return None;
        }
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut length);
        CloseHandle(handle);
        if ok == 0 || length == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

#[cfg(not(target_os = "windows"))]
fn foreground_app_path() -> Option<String> {
    None
}

/// 从可执行文件完整路径取应用名：只留文件名，去掉 `.exe` 后缀（不区分大小写）。
///
/// 抽成纯函数以便单测；空路径或以分隔符结尾（无文件名）返回 None。
pub fn app_name_from_path(path: &str) -> Option<String> {
    let file = path.rsplit(['\\', '/']).next()?.trim();
    if file.is_empty() {
        return None;
    }
    let name = match file.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.eq_ignore_ascii_case("exe") => stem,
        _ => file,
    };
    Some(name.to_string())
}
```

- [ ] **Step 5: `data/mod.rs` v6 迁移**

第 86 行的文档注释末尾追加 `→ v6（clipboard_entries.source_app）`。第 187 行 `}`（v5 分支结束）之后、`Ok(())` 之前插入：

```rust
        if version < 6 {
            self.with_v6()
                .map_err(|e| format!("数据库迁移到 v6 失败: {e}"))?;
            self.db
                .pragma_update(None, "user_version", 6)
                .map_err(db_err)?;
        }
```

`with_v5` 之后插入：

```rust
    /// v6 增量：剪贴板条目来源应用（spec D20）。存量行不回填，来源未知即 NULL。
    fn with_v6(&self) -> Result<(), String> {
        self.add_column_if_missing("clipboard_entries", "source_app", "TEXT")
    }
```

- [ ] **Step 6: `models.rs` 与 `data/clipboard.rs`**

`models.rs` 第 39–44 行之间，在 `pinned: bool,` 之前加：

```rust
        /// 采集时的前台应用名（v6 起）；旧数据与采集失败时为 None，序列化时省略。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_app: Option<String>,
```

`data/clipboard.rs` 第 8–23 行替换为：

```rust
const ROW_COLUMNS: &str =
    "id, content_type, content, preview, file_path, pinned, copied_at, modified_at, source_app";

fn row_entry(r: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    ClipboardEntry::builder()
        .id(r.get(0)?)
        .content_type(r.get(1)?)
        .content(r.get(2)?)
        .preview(r.get(3)?)
        .file_path(r.get(4)?)
        .pinned(r.get::<_, i64>(5)? != 0)
        .copied_at(r.get(6)?)
        .modified_at(r.get(7)?)
        .source_app(r.get(8)?)
        .build()
        .map_err(super::build_err)
}
```

`Capture` 结构（第 26–33 行）在 `hash: String,` 之前加：

```rust
    /// 采集瞬间的前台应用名；None 表示未知。
    pub source_app: Option<String>,
```

`insert_capture` 的 INSERT（第 72–88 行）替换为：

```rust
        self.db
            .execute(
                "INSERT INTO clipboard_entries(id, content_type, content, preview, file_path, content_hash, copied_at, modified_at, created_at, source_app) \
                 VALUES(?,?,?,?,?,?,?,?,?,?)",
                rusqlite::params![
                    id,
                    capture.content_type,
                    capture.content,
                    capture.preview,
                    capture.file_path,
                    capture.hash,
                    timestamp,
                    timestamp,
                    timestamp,
                    capture.source_app
                ],
            )
            .map_err(db_err)?;
```

- [ ] **Step 7: `clipboard_watcher.rs` 写入来源**

`capture_text` 中的 `Capture` 字面量（第 99–105 行）替换为：

```rust
    // 采集瞬间读前台应用：轮询间隔 500ms，此刻前台几乎总还是复制发生的应用；
    // 面板内手动捕获时前台是本应用，platform 层返回 None。
    let source_app = crate::platform::foreground_app_name();
    let input = crate::data::clipboard::Capture {
        content: text.chars().take(MAX_INLINE_TEXT).collect(),
        content_type: kind.as_str(),
        preview: capture,
        file_path: None,
        hash,
        source_app,
    };
```

`capture_image` 中的字面量（第 124–130 行）替换为：

```rust
    let input = crate::data::clipboard::Capture {
        content: String::new(),
        content_type: "image",
        preview,
        file_path: Some(relative),
        hash,
        source_app: crate::platform::foreground_app_name(),
    };
```

（`ipc::clipboard_capture` 复用 `capture_text`，手动捕获随之带上来源，无需另改。）

- [ ] **Step 8: 运行测试通过**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check && cargo check --manifest-path src-tauri/Cargo.toml && cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "v6_migration|insert_capture_persists|app_name_from_path|^test result"`
Expected: 三个新测试 `ok`，`test result: ok`。

- [ ] **Step 9: 前端类型**

`src/typings/domain.ts` 第 37 行 `file_path?: string | null` 之后加：

```ts
  /** 采集时的前台应用名（v6 起）；旧数据或采集失败时缺省。 */
  source_app?: string | null
```

- [ ] **Step 10: 校验并提交**

```bash
pnpm typecheck
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/platform.rs src-tauri/src/data/mod.rs src-tauri/src/data/clipboard.rs src-tauri/src/domain/models.rs src-tauri/src/services/clipboard_watcher.rs src/typings/domain.ts
git commit -m "feat(clipboard): 采集来源应用（v6 迁移 source_app）"
```

---

## Task 3: 矢量圆点导航与 ARIA，删除 emoji 桥接

**Files:**
- Modify: `src/panel-plugins/index.ts:18-40`
- Modify: `src/windows/Panel/PanelApp.vue:363,375-387`
- Modify: `src/windows/Main/SettingsView.vue:279`
- Modify: `src/styles/extensions.css:79-96`（§3 追加）、`:133-149`（删除）
- Modify: `src/styles/motion.css:58-69`（删除）
- Modify: `src/windows/panel.ts`

**Interfaces:**
- Produces: `PanelPlugin.dotClass: string`（内置 `'dot-note' | 'dot-clip' | 'dot-todo'`）；`PanelApp` 的 `hotkeyTitle(plugin, index): string`。

- [ ] **Step 1: 注册表 `dot` → `dotClass`**

`src/panel-plugins/index.ts` 第 24–25 行替换为：

```ts
  /**
   * 圆点导航的颜色类名（原型 `.nav-dot.dot-note / .dot-clip / .dot-todo`）。
   * 生成层用 `--dot-c` 令牌绘制 ::before 矢量圆；新插件若没有对应令牌，
   * 会落到 extensions.css §3 `.nav-dots { --dot-c: var(--accent-rgb) }` 的兜底色。
   */
  dotClass: string
```

第 36–40 行替换为：

```ts
export const builtinPlugins: readonly PanelPlugin[] = [
  { id: 'note', label: '笔记', dotClass: 'dot-note', component: NotePage },
  { id: 'clipboard', label: '粘贴板', dotClass: 'dot-clip', component: ClipPage },
  { id: 'todo', label: '待办', dotClass: 'dot-todo', component: TodoPage },
] as const
```

- [ ] **Step 2: `PanelApp.vue` 圆点模板**

第 363 行 `const activeLabel = …` 之后追加：

```ts
/** 圆点 title：前 9 个带 ⌃N 序号（原型 `title="笔记 (⌃1)"`）。 */
function hotkeyTitle(plugin: PanelPlugin, index: number): string {
  return index < MAX_HOTKEY_SLOTS ? `${plugin.label} (⌃${index + 1})` : plugin.label
}
```

并把第 10 行 import 改为 `import { MAX_HOTKEY_SLOTS, resolvePlugins, type PanelPlugin } from '@/panel-plugins'`。

第 375–387 行 `.panel-nav` 替换为（Zen 入口在 Task 4 加入）：

```vue
    <!-- 插件圆点导航（原型 .nav-dots）：矢量圆由生成层 ::before 绘制，序号即 ⌃N 快捷键 -->
    <div class="panel-nav">
      <div class="nav-dots" role="tablist" aria-label="捕获模式切换">
        <button
          v-for="(plugin, index) in plugins"
          :key="plugin.id"
          type="button"
          class="nav-dot"
          :class="[plugin.dotClass, { active: activeId === plugin.id }]"
          :data-mode="plugin.id"
          role="tab"
          :aria-selected="activeId === plugin.id"
          :aria-label="`${plugin.label}模式`"
          :title="hotkeyTitle(plugin, index)"
          @click="navigateTo(plugin.id)"
        />
      </div>
      <span class="panel-hint">Esc 收起</span>
    </div>
```

- [ ] **Step 3: `SettingsView.vue` 插件行**

第 279 行 `<span>{{ plugin.dot }} {{ plugin.label }}</span>` 改为：

```vue
        <span>{{ plugin.label }}</span>
```

（不再有 emoji 可显示；生成层 `.nav-dot` 是 `display: grid` 的 22px 块级盒，内联进 label 会把文字挤到下一行，且设置页的插件行原型本就只有文字。）

- [ ] **Step 4: 样式：删桥接、补兜底**

`src/styles/extensions.css`：删除第 133–149 行（「圆点导航仍是 emoji 字符…」两段及其注释，共两个规则块 `.nav-dot:hover` / `.nav-dot.active` 与 `.nav-dot::before { content: none }`）。在 §3 末尾（第 96 行 `.prio-opt {…}` 之后、§4 之前）追加：

```css
/* 圆点导航兜底色：生成层 .nav-dot::before 用 rgb(var(--dot-c)) 上色，内置三页在 .dot-* 上声明了 --dot-c；
   第三方 / 新增插件没有对应令牌时变量缺失会让圆点不可见。自定义属性向下继承，
   元素自身 .dot-* 的声明优先于此处父级值，不受加载顺序影响。 */
.nav-dots {
  --dot-c: var(--accent-rgb);
}
```

`src/styles/motion.css`：删除第 58–69 行（「── 圆点导航：选中与悬浮的缩放反馈 ──」整段，生成层 `::before` 自带 transition）。

- [ ] **Step 5: `panel.ts` 设置窗口标识**

```ts
/**
 * 呼出面板窗口入口。
 *
 * 该窗口对启动速度敏感（需求「1 秒原则」），因此只加载面板自身依赖，
 * 不引入归档页、统计图表等重量级模块。
 */
import { createApp } from 'vue'
import PanelApp from './Panel/PanelApp.vue'
import '@/styles'

// window-fit.css 的面板整页化弹窗规则以 :root[data-window='panel'] 为前缀（与 editor / mindmap 同款）。
document.documentElement.dataset.window = 'panel'

createApp(PanelApp).mount('#app')
```

- [ ] **Step 6: 校验并提交**

```bash
pnpm sync:styles --check && pnpm typecheck
git add src/panel-plugins/index.ts src/windows/Panel/PanelApp.vue src/windows/Main/SettingsView.vue src/styles/extensions.css src/styles/motion.css src/windows/panel.ts
git commit -m "feat(panel): 矢量圆点导航与 ARIA，删除 emoji 桥接"
```

---

## Task 4: 笔记回显编辑态 + Zen 专注模式，主窗口 ✏️ 改为回显到面板

**Files:**
- Modify: `src/panel-plugins/index.ts`（`PanelPageExpose`）
- Modify: `src/windows/Panel/PanelApp.vue`
- Rewrite: `src/windows/Panel/NotePage.vue`
- Modify: `src/windows/Main/NotesView.vue`、`src/windows/Main/DayView.vue`
- Delete: `src/components/note/NoteEditModal.vue`
- Modify: `src/styles/window-fit.css:65-68` 之后

**Interfaces:**
- Consumes: Task 1 的 `api.windows.panelTakeIntent / panelOpenNote / panelSetZen`、`api.notes.get`、`PanelIntent`；`NoteEditor` 暴露的 `focus()`。
- Produces:
  - `export interface PanelPageExpose { focus?(): void; dismissOverlays?(): boolean; loadNote?(id: string): Promise<void>; onPanelHide?(): void | Promise<void>; isEditing?: boolean }`（`isEditing` 在 NotePage 里是 `computed`，经模板 ref 拿到的实例代理会自动解包为 boolean）。
  - `PanelApp`：`pageRefs: Record<string, PanelPageExpose | null>`、`setZen(on: boolean): Promise<void>`、`zen: Ref<boolean>`、`visible: Ref<boolean>`、`consumeIntent(): Promise<void>`。
  - `NotePage` expose：`archive / focus / dismissOverlays / loadNote / onPanelHide / isEditing`；emits `modal` / `zen-exit`（Task 5 再加 `archived`）。

- [ ] **Step 1: `PanelPageExpose` 契约**

`src/panel-plugins/index.ts` 在 `PanelPlugin` 接口之前插入：

```ts
/**
 * 插件页可选暴露给 PanelApp 的能力（通过 `defineExpose`）。全部可选，PanelApp 只调用存在的方法。
 *
 * - `focus`：呼出 / 进入 Zen 后把光标放进页面主输入区；
 * - `dismissOverlays`：关闭本页的删除确认 / 弹窗，关了返回 true（Esc 链与切页副作用用）；
 * - `loadNote`：把某条笔记回显进编辑态（只有笔记页实现）；
 * - `onPanelHide`：面板即将隐藏（丢弃回显编辑、恢复草稿），PanelApp 会等待它完成再隐藏窗口；
 * - `isEditing`：是否处于回显编辑态（Zen 入口显隐）。页面 expose 的是 `ComputedRef<boolean>`，
 *   经模板 ref 拿到的实例代理会自动解包，这里按解包后的 boolean 声明。
 */
export interface PanelPageExpose {
  focus?(): void
  dismissOverlays?(): boolean
  loadNote?(id: string): Promise<void>
  onPanelHide?(): void | Promise<void>
  isEditing?: boolean
}
```

- [ ] **Step 2: `NotePage.vue` 整体重写**

```vue
<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import TagList from '@/components/tag/TagList.vue'
import TagManagerModal from '@/components/tag/TagManagerModal.vue'
import NoteEditor from '@/editor/NoteEditor.vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Note } from '@/typings/domain'

/**
 * 面板 · 笔记模式（原型 #page-note）。
 *
 * - Markdown 即时渲染（由 NoteEditor 提供）；新建态下输入停止 500ms 自动暂存为草稿；
 * - 右下角标签区在归档按钮左侧，点击弹标签管理弹窗；「归档念头 ↵」正式落盘；
 * - **回显编辑态**（spec D16，原型 openNoteInPanel / hidePanel）：主窗口 ✏️ → `loadNote(id)`
 *   先落库当前草稿再载入笔记，按钮变「保存修改 ✓」，编辑态**不自动暂存**；
 *   收面板（`onPanelHide`）即丢弃未保存修改并恢复草稿；
 * - 底栏「退出 Zen」按钮显隐完全交给生成层 `#zenExit` / `#panel.zen-mode #zenExit`。
 *
 * 面板**只写文本笔记**：思维导图统一在归档页创建、在独立窗口里编辑（见 NotesView），
 * 因此不从草稿恢复 mindmap 模式，也不渲染模式切换条。
 */
const emit = defineEmits<{
  (e: 'modal', open: boolean): void
  /** 用户点击底栏「退出 Zen」。 */
  (e: 'zen-exit'): void
}>()

const { toast } = useToast()

const editor = ref<InstanceType<typeof NoteEditor> | null>(null)
const content = ref('')
const tags = ref<string[]>([])
/** 草稿 id：首次暂存后由后端返回，后续复用以免产生多条草稿。 */
const draftId = ref<string | undefined>(undefined)
/** 正在回显编辑的笔记 id；null = 新建态。 */
const editingNoteId = ref<string | null>(null)
/** 回显时的笔记快照：保存修改时把 editor_mode / mindmap_data 原样带回，避免被清空。 */
const editingSnapshot = ref<Note | null>(null)
const showTagManager = ref(false)
const saveState = ref<'idle' | 'saving' | 'saved'>('idle')

/** 是否处于回显编辑态（供 PanelApp 决定 Zen 入口显隐）。 */
const isEditing = computed(() => editingNoteId.value !== null)
/** 底栏主按钮文案（原型 #btnArchive）。 */
const archiveLabel = computed(() => (isEditing.value ? '保存修改 ✓' : '归档念头 ↵'))

/** 500ms 防抖的暂存定时器。 */
let debounceTimer: ReturnType<typeof setTimeout> | null = null
/** 程序性写入 content / tags 时置位，让紧随其后的 watch 回调跳过自动暂存。 */
let suppressAutosave = false

const saveLabel = computed(() => {
  if (saveState.value === 'saving') return '暂存中…'
  if (saveState.value === 'saved') return '已暂存'
  return '未保存'
})

function cancelPendingAutosave(): void {
  if (debounceTimer !== null) {
    clearTimeout(debounceTimer)
    debounceTimer = null
  }
}

/**
 * 程序性写入编辑区（恢复草稿 / 回显笔记 / 归档后清空），不触发自动暂存。
 * watch 在下一次 pre-flush 执行，等 nextTick 再解除抑制；无变化时 watch 不触发，同样在此解除。
 */
async function setLocal(nextContent: string, nextTags: string[]): Promise<void> {
  cancelPendingAutosave()
  suppressAutosave = true
  content.value = nextContent
  tags.value = nextTags
  await nextTick()
  suppressAutosave = false
}

/** 拉取既有草稿到编辑区；没有可用草稿时清空（退出编辑态时也靠它把笔记内容换掉）。 */
async function loadDraft(): Promise<void> {
  try {
    const draft = await api.notes.draft()
    // 思维导图草稿不在面板里恢复：面板没有导图编辑能力，恢复它只会得到一块无法输入的空白。
    if (!draft || draft.editor_mode === 'mindmap') {
      if (draft) logger.info('panel-note', `跳过思维导图草稿 id=${draft.id}，请在归档页编辑`)
      draftId.value = undefined
      await setLocal('', [])
      saveState.value = 'idle'
      return
    }
    draftId.value = draft.id
    await setLocal(draft.content, [...draft.tags])
    saveState.value = 'saved'
    logger.info('panel-note', `恢复草稿 id=${draft.id}`)
  } catch (error) {
    logger.error('panel-note', '加载草稿失败', error)
  }
}
void loadDraft()

/** 暂存草稿（不广播 notes-changed，后端对草稿不发事件）。编辑态不暂存。 */
async function persistDraft(): Promise<void> {
  if (editingNoteId.value) return
  if (!content.value.trim() && !tags.value.length) return
  saveState.value = 'saving'
  try {
    const note = await api.notes.save({
      id: draftId.value,
      content: content.value,
      tags: [...tags.value],
      editorMode: 'text',
      mindmapData: null,
      draft: true,
    })
    draftId.value = note.id
    saveState.value = 'saved'
    logger.debug('panel-note', `草稿已暂存 id=${note.id}`)
  } catch (error) {
    saveState.value = 'idle'
    logger.error('panel-note', '暂存失败', error)
  }
}

// 新建态：输入停止 500ms 后自动暂存（需求 2.2「混合存储策略」）；编辑态与程序性写入跳过。
watch([content, tags], () => {
  if (suppressAutosave) return
  if (editingNoteId.value) return
  saveState.value = 'idle'
  cancelPendingAutosave()
  debounceTimer = setTimeout(() => {
    debounceTimer = null
    void persistDraft()
  }, 500)
})

/**
 * 回显一条笔记进入编辑态（原型 openNoteInPanel）。
 * 先把当前草稿落库——回显不能覆盖用户还没归档的念头；已在编辑另一条时直接替换。
 */
async function loadNote(id: string): Promise<void> {
  logger.info('panel-note', `回显笔记 id=${id}`)
  if (!editingNoteId.value) {
    cancelPendingAutosave()
    await persistDraft()
  }
  let note: Note | null
  try {
    note = await api.notes.get(id)
  } catch (error) {
    logger.error('panel-note', '读取笔记失败', error)
    toast('读取笔记失败')
    return
  }
  if (!note) {
    toast('笔记不存在')
    return
  }
  editingNoteId.value = id
  editingSnapshot.value = note
  await setLocal(note.content, [...note.tags])
  toast('内容已回显，修改后点击「保存修改」')
  focus()
}

function exitEditing(): void {
  editingNoteId.value = null
  editingSnapshot.value = null
}

/** 归档：新建态把草稿提升为正式笔记；编辑态更新原笔记并恢复草稿（原型 archiveNote）。 */
async function archive(): Promise<void> {
  if (!content.value.trim()) {
    toast('先写点什么吧')
    return
  }
  const snapshot = editingSnapshot.value
  if (editingNoteId.value && snapshot) {
    logger.info('panel-note', `保存修改 id=${editingNoteId.value}`)
    try {
      await api.notes.save({
        id: editingNoteId.value,
        content: content.value,
        tags: [...tags.value],
        editorMode: snapshot.editor_mode,
        mindmapData: snapshot.mindmap_data,
        draft: false,
      })
    } catch (error) {
      logger.error('panel-note', '保存修改失败', error)
      toast('保存失败')
      return
    }
    exitEditing()
    await loadDraft()
    toast('修改已保存 ✔')
    return
  }

  logger.info('panel-note', '归档念头')
  try {
    await api.notes.save({
      id: draftId.value,
      content: content.value,
      tags: [...tags.value],
      editorMode: 'text',
      mindmapData: null,
      draft: false,
    })
  } catch (error) {
    logger.error('panel-note', '归档失败', error)
    toast('归档失败')
    return
  }
  // 归档后清空面板，进入下一次捕获。
  draftId.value = undefined
  await setLocal('', [])
  saveState.value = 'idle'
  toast('念头已归档 ✔')
}

/** 面板即将隐藏：编辑态视为放弃修改，恢复草稿（原型 hidePanel）。 */
async function onPanelHide(): Promise<void> {
  if (!editingNoteId.value) return
  logger.info('panel-note', `收面板，丢弃未保存修改 id=${editingNoteId.value}`)
  exitEditing()
  await loadDraft()
}

/** 把光标放进编辑器（呼出 / 回显 / 进入 Zen 后）。 */
function focus(): void {
  editor.value?.focus()
}

function openTagManager(): void {
  showTagManager.value = true
  emit('modal', true)
}

function closeTagManager(): void {
  showTagManager.value = false
  emit('modal', false)
}

/** 关闭本页浮层：标签管理弹窗开着就关掉并返回 true（切页副作用用）。 */
function dismissOverlays(): boolean {
  if (!showTagManager.value) return false
  closeTagManager()
  return true
}

function saveTags(next: string[]): void {
  tags.value = next
  closeTagManager()
}

defineExpose({ archive, focus, dismissOverlays, loadNote, onPanelHide, isEditing })
</script>

<template>
  <section class="panel-page">
    <NoteEditor ref="editor" v-model="content" editor-mode="text" :show-mode-bar="false" @submit="archive" />

    <div class="editor-footer">
      <span class="save-state" :class="{ saving: saveState === 'saving' }">{{ saveLabel }}</span>
      <div class="editor-actions">
        <!-- 标签区位于归档按钮左侧（需求 2.2 指定的两处展示位置之一） -->
        <div class="tag-preview" title="点击管理标签">
          <TagList :tags="tags" :max="3" @open="openTagManager" />
        </div>
        <!-- 原型 #zenExit：显隐由生成层按 #panel.zen-mode 控制，不传 prop -->
        <button id="zenExit" type="button" class="btn ghost" title="退出 Zen 专注模式（Esc）" @click="emit('zen-exit')">
          <span class="ix">🧘</span> 退出 Zen
        </button>
        <button type="button" class="btn primary" @click="archive">{{ archiveLabel }}</button>
      </div>
    </div>

    <TagManagerModal
      v-if="showTagManager"
      :tags="tags"
      :max-length="5"
      subtitle="当前笔记的标签"
      @save="saveTags"
      @close="closeTagManager"
    />
  </section>
</template>
```

- [ ] **Step 3: `PanelApp.vue` 脚本改动**

（a）第 2 行与第 8–10 行 import 改为：

```ts
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowReactive, watch, type ComponentPublicInstance } from 'vue'
…
import { logger } from '@/service/logger'
import { api, type PanelIntent } from '@/service/tauri'
import { MAX_HOTKEY_SLOTS, resolvePlugins, type PanelPageExpose, type PanelPlugin } from '@/panel-plugins'
```

（b）头注释第 22 行「高度随内容自适应（120~600px）」改为「高度随内容自适应（90~600px）」，并在列表末尾追加两行：

```
 * - Zen 专注模式（spec D15）：窗口铺满工作区、#panel.zen-mode；Esc 只退 Zen；
 * - 回显编辑态（spec D16）：呼出意图带 noteId 时让笔记页 loadNote；收面板前通知各页 onPanelHide。
```

（c）第 41 行 `const modalDepth = ref(0)` 之后插入：

```ts
/** 各插件页实例（按插件 id）：回显 / 聚焦 / 关浮层 / 收面板通知都经它调用，只调用页面实现了的方法。 */
const pageRefs = shallowReactive<Record<string, PanelPageExpose | null>>({})
/** Zen 专注模式：为真时根元素带 .zen-mode，窗口已由后端放大到工作区。 */
const zen = ref(false)
/** 面板窗口是否可见：挂载即可见，hide 完成后 false，panelShown 后 true（Zen 入口显隐用）。 */
const visible = ref(true)

/** 模板 `:ref` 回调：v-for 里的组件实例按插件 id 记账。 */
function setPageRef(id: string, instance: Element | ComponentPublicInstance | null): void {
  pageRefs[id] = instance as unknown as PanelPageExpose | null
}
```

（d）第 80 行 `const plugins = …` 之后插入：

```ts
/** Zen 入口显隐（原型 updateZenToggle）：面板可见 ∧ 未处于 Zen ∧ 笔记页 ∧ 回显编辑态。 */
const zenAvailable = computed(
  () => visible.value && !zen.value && activeId.value === 'note' && (pageRefs.note?.isEditing ?? false),
)

/**
 * 进入 / 退出 Zen（原型 setZenMode）。
 * 进入：先让后端放大窗口再加类，避免 100vh 布局先在小窗口里闪一下；
 * 退出：先去类让内容收回，再让后端按记录的逻辑高度还原窗口。
 */
async function setZen(on: boolean): Promise<void> {
  if (zen.value === on) return
  logger.info('panel', on ? '进入 Zen 专注模式' : '退出 Zen 专注模式')
  if (!on) zen.value = false
  try {
    await api.windows.panelSetZen(on)
  } catch (error) {
    logger.error('panel', 'Zen 窗口切换失败', error)
    return
  }
  zen.value = on
  if (on) {
    // 原型：进入后 60ms 聚焦编辑器
    setTimeout(() => pageRefs[activeId.value]?.focus?.(), 60)
  } else {
    // Zen 期间高度上报被暂停，退出后按内容补报一次
    void nextTick(() => reportHeight())
  }
}
```

（e）第 140–157 行 `hide()` 替换为：

```ts
async function hide(): Promise<void> {
  clearCollapseTimer()
  logger.info('panel', '收起面板')
  // Zen 态先还原窗口（原型 hidePanel 的 setZenMode(false)）；Zen 下生成层 transform: none !important，位移动画无意义。
  if (zen.value) await setZen(false)

  if (panel.value) {
    // 与原型 hidePanel 一致：24px 位移 + 淡出，180ms inQuad。被新的入场打断时不再隐藏窗口。
    const completed = await exit(panel.value, { axis: motionAxis(), distance: motionDistance(24) })
    if (!completed) {
      logger.debug('panel', '收起动画被入场打断，取消隐藏')
      return
    }
  }
  // 收起即丢弃：通知各页（笔记页据此丢弃未保存的回显修改并恢复草稿）。
  // 必须在窗口隐藏之前等它完成——WebView2 在窗口 hide 后挂起，此后的 IPC 回包要等下次显示。
  await Promise.all(Object.values(pageRefs).map((page) => page?.onPanelHide?.()))
  try {
    await api.windows.panelHide()
    visible.value = false
  } catch (error) {
    logger.error('panel', '隐藏面板失败', error)
  }
}
```

（f）第 245–251 行 `onKeydown` 的 Escape 分支替换为（浮层一环在 Task 7 接入）：

```ts
  if (event.key === 'Escape') {
    // 弹窗自己处理 Esc（ModalShell 在捕获阶段拦截），面板不抢。
    if (modalDepth.value > 0) return
    event.preventDefault()
    // 原型 Esc 链首位：Zen 态只退 Zen，不收面板。
    if (zen.value) {
      void setZen(false)
      return
    }
    void hide()
    return
  }
```

（g）第 264–271 行 `reportHeight` 的文档注释里「钳制在 120~600px」改为「钳制在 90~600px」，并在函数体第一行（第 282 行 `if (!panel.value) return` 之前）插入：

```ts
  // Zen 态窗口铺满工作区，内容高度没有意义；退出 Zen 时由 setZen 补报。
  if (zen.value) return
```

（h）`onMounted` 里的 panelShown 订阅（`void onAppEvent(AppEvents.panelShown, …)` 整块，Task 1 已改为 `panelTakeIntent`）替换为：

```ts
  // 后端每次显示面板都会广播：重播入场动画，并取走呼出意图（切页 / 回显）。
  // 隐藏期间的事件会丢，所以意图存在后端、由前端主动拉。
  void onAppEvent(AppEvents.panelShown, () => {
    clearCollapseTimer()
    visible.value = true
    playEnter()
    void consumeIntent()
  })
```

并在 `onKeydown` 之前插入：

```ts
/** 取走后端暂存的呼出意图：切页；带 noteId 则让目标页回显该笔记。 */
async function consumeIntent(): Promise<void> {
  let intent: PanelIntent | null
  try {
    intent = await api.windows.panelTakeIntent()
  } catch (error) {
    logger.error('panel', '读取呼出意图失败', error)
    return
  }
  if (!intent) return
  logger.info('panel', `呼出意图 page=${intent.page} noteId=${intent.noteId ?? '-'}`)
  navigateTo(intent.page)
  if (!intent.noteId) return
  // 切页后等一帧，确保目标页已渲染并挂上实例引用。
  await nextTick()
  const page = pageRefs[intent.page]
  if (!page?.loadNote) {
    logger.warn('panel', `插件页 ${intent.page} 不支持回显，忽略 noteId`)
    return
  }
  await page.loadNote(intent.noteId)
}
```

- [ ] **Step 4: `PanelApp.vue` 模板改动**

根元素 `:class` 改为 `:class="{ 'modal-open': modalDepth > 0, 'zen-mode': zen }"`。

`.panel-nav` 内、`</div>`（`.nav-dots` 结束）与 `<span class="panel-hint">` 之间插入：

```vue
      <!-- 原型 #zenToggle：只在回显编辑态出现 -->
      <button
        v-if="zenAvailable"
        id="zenToggle"
        type="button"
        class="btn ghost tiny"
        title="Zen 专注模式：全屏沉浸编辑（Esc 退出）"
        @click="setZen(true)"
      >
        <span class="ix">🧘</span> Zen
      </button>
```

`<component>` 标签替换为：

```vue
    <component
      :is="plugin.component"
      v-for="plugin in plugins"
      v-show="activeId === plugin.id"
      :key="plugin.id"
      :ref="(el) => setPageRef(plugin.id, el)"
      @modal="onModalToggle"
      @external-editor="onExternalEditorOpen"
      @zen-exit="setZen(false)"
    />
```

- [ ] **Step 5: 主窗口 ✏️ 改为回显到面板**

`src/windows/Main/NotesView.vue`：
- 删除第 5 行 `import NoteEditModal …`；第 13 行改为 `import type { Note } from '@/typings/domain'`；
- 头注释第 23 行改为 `* - 两条编辑入口互不混用：「✏️ 编辑」→ 文本笔记回显到呼出面板（spec D16）/ 导图开独立窗口；标签区 → 标签管理弹窗。`；
- 删除第 34–35 行 `editTarget` 声明与注释；删除第 157–172 行 `saveNote`；
- 在 `openMindmap` 之后插入：

```ts
/** ✏️ 文本笔记 → 回显到呼出面板编辑（spec D16）：后端隐藏主窗口并呼出面板，面板取走意图后载入笔记。 */
function openInPanel(note: Note): void {
  logger.info('notes-view', `回显到面板 id=${note.id}`)
  void api.windows.panelOpenNote(note.id).catch((error) => {
    logger.error('notes-view', '回显到面板失败', error)
    toast('打开面板失败')
  })
}
```

- 模板第 193 行改为 `@edit="note.editor_mode === 'mindmap' ? openMindmap(note.id) : openInPanel(note)"`；删除第 203–204 行（注释与 `<NoteEditModal …/>`）。

`src/windows/Main/DayView.vue`：
- 删除第 9 行 `import NoteEditModal …`；第 18 行改为 `import type { ClipboardEntry, DayDetailItem, Priority, Todo, TodoInput } from '@/typings/domain'`；
- 头注释第 30–31 行「编辑入口按类别分流：文本笔记弹窗、导图开独立窗口」改为「编辑入口按类别分流：文本笔记回显到呼出面板（spec D16）、导图开独立窗口」；
- 删除第 132 行 `const editNote = ref<Note | null>(null)`；第 147 行 `editNote.value = item.note` 改为 `openNoteInPanel(item.note.id)`；删除第 159–170 行 `saveNote`；在 `edit()` 之前插入：

```ts
/** 文本笔记 → 回显到呼出面板编辑；主窗口随之隐藏。 */
function openNoteInPanel(id: string): void {
  logger.info('day-view', `回显到面板 id=${id}`)
  void api.windows.panelOpenNote(id).catch((error) => {
    logger.error('day-view', '回显到面板失败', error)
    toast('打开面板失败')
  })
}
```

- 删除模板第 312 行 `<NoteEditModal …/>`。

删除文件：`git rm src/components/note/NoteEditModal.vue`（目录随之为空）。

- [ ] **Step 6: `window-fit.css` Zen 纵向撑满**

第 68 行 `#panel {…}` 块之后插入：

```css
/* Zen 专注模式：窗口已由 Rust 放大到整个工作区。#panel 在上面被设为 position: static，
   生成层给的 top/left/width:100vw 被覆盖但无害；纵向必须显式撑满，否则编辑区只有内容高度。
   圆角裁剪同步取消——铺满工作区还留 8px 圆角会在四角露出桌面。 */
#panel.zen-mode {
  height: 100vh;
  clip-path: none;
}
```

- [ ] **Step 7: 校验并提交**

```bash
pnpm typecheck && grep -rn "NoteEditModal\|panelTakePage\|plugin\.dot\b" src || echo "无残留引用"
git add -A src/panel-plugins/index.ts src/windows/Panel/PanelApp.vue src/windows/Panel/NotePage.vue src/windows/Main/NotesView.vue src/windows/Main/DayView.vue src/components/note src/styles/window-fit.css
git commit -m "feat(panel): 笔记回显编辑态 + Zen 专注模式，主窗口 ✏️ 改为回显到面板"
```

---

## Task 5: 编辑器底栏文案 / 标签预览 / 归档收起 / 呼出聚焦

**Files:**
- Modify: `src/components/tag/TagList.vue`
- Modify: `src/windows/Panel/NotePage.vue`（Task 4 版本）
- Modify: `src/windows/Panel/PanelApp.vue`

**Interfaces:**
- Consumes: Task 4 的 `pageRefs` / `hide()` / `consumeIntent()`。
- Produces: `TagList` prop `moreAction?: 'expand' | 'open'`（默认 `'expand'`）；`NotePage` emit `archived`；`PanelApp.onArchived()`。

- [ ] **Step 1: `TagList.vue` 的 `moreAction`**

props 里 `shaking?: boolean` 之后加：

```ts
    /**
     * 「+N」的行为：`expand` 就地展开（卡片）；`open` 不拦截点击，冒泡给父级 `.tag-preview` 打开标签管理
     * （面板右下角，原型 renderPanelTags 的 +N 冒泡到 tagPreview）。
     */
    moreAction?: 'expand' | 'open'
```

默认值对象改为 `{ max: 3, deletable: false, shakingTag: null, shaking: false, moreAction: 'expand' }`。

脚本末尾追加：

```ts
/** `expand` 才拦截点击并展开；`open` 让事件冒泡到父级。 */
function onMore(event: MouseEvent): void {
  if (props.moreAction !== 'expand') return
  event.stopPropagation()
  expanded.value = true
}
```

模板 `+N` 元素替换为：

```vue
      <span
        v-if="!showAll && hiddenCount > 0"
        class="tag-more"
        :title="props.moreAction === 'open' ? '查看全部标签' : `展开其余 ${hiddenCount} 个标签`"
        @click="onMore"
      >
        +{{ hiddenCount }}
      </span>
```

- [ ] **Step 2: `NotePage.vue` 暂存三段文案与 `.saving` 时机**

原型：初始「已暂存」；输入中「输入中…」+ `.saving`；500ms 落库后「已暂存 SQLite」。失败态是项目扩展（原型无失败路径，静默显示「已暂存」会误导）。

把

```ts
const saveState = ref<'idle' | 'saving' | 'saved'>('idle')
```

改为

```ts
/** 暂存状态：idle「已暂存」（初始 / 恢复草稿后）· typing「输入中…」· saved「已暂存 SQLite」· failed「暂存失败」。 */
const saveState = ref<'idle' | 'typing' | 'saved' | 'failed'>('idle')
```

把 `saveLabel` 替换为：

```ts
const saveLabel = computed(() => {
  if (saveState.value === 'typing') return '输入中…'
  if (saveState.value === 'saved') return '已暂存 SQLite'
  if (saveState.value === 'failed') return '暂存失败'
  return '已暂存'
})
```

`persistDraft` 里删除 `saveState.value = 'saving'` 这一行；catch 分支 `saveState.value = 'idle'` 改为 `saveState.value = 'failed'`。

`watch([content, tags], …)` 回调里 `saveState.value = 'idle'` 改为 `saveState.value = 'typing'`。

`loadDraft` 成功分支 `saveState.value = 'saved'` 改为 `saveState.value = 'idle'`（恢复草稿后显示初始文案「已暂存」，与原型一致）。

模板 `.save-state` 的 class 改为 `:class="{ saving: saveState === 'typing' }"`。

- [ ] **Step 3: `NotePage.vue` 标签预览与副标题**

脚本里 `archiveLabel` 之后加：

```ts
/** 标签管理弹窗副标题（原型 openTagManager 的 draft / note 两种目标）。 */
const tagSubtitle = computed(() =>
  isEditing.value ? '当前笔记的标签' : '当前正在编写的念头（未归档）的标签',
)
```

模板 `.tag-preview` 块替换为（点击整块打开管理；+N 与 chips 冒泡到这里，不再监听 TagList 的 `open`）：

```vue
        <div class="tag-preview" title="点击管理标签" @click="openTagManager">
          <TagList :tags="tags" :max="3" more-action="open" />
        </div>
```

`TagManagerModal` 的 `subtitle="当前笔记的标签"` 改为 `:subtitle="tagSubtitle"`。

- [ ] **Step 4: 归档后 emit `archived`**

`defineEmits` 里加一行：

```ts
  /** 归档 / 保存修改成功：PanelApp 据此在 250ms 后收面板（原型 archiveNote 的 setTimeout(hidePanel, 250)）。 */
  (e: 'archived'): void
```

`archive()` 里编辑态分支的 `toast('修改已保存 ✔')` 之后、`return` 之前加 `emit('archived')`；新建态分支的 `toast('念头已归档 ✔')` 之后加 `emit('archived')`。

- [ ] **Step 5: `PanelApp.vue` 归档收起与呼出聚焦**

`onExternalEditorOpen` 之后插入：

```ts
/** 笔记归档 / 保存修改成功：原型 250ms 后收面板，让 toast 先被看到。 */
function onArchived(): void {
  logger.debug('panel', '归档完成，250ms 后收起')
  setTimeout(() => void hide(), 250)
}
```

`consumeIntent()` 里，紧接 `try { intent = await api.windows.panelTakeIntent() } catch {…}` 之后、`if (!intent) return` 之前插入两行（定时器回调在 100ms 后才读 `activeId`，此时 `navigateTo` 已切好页，聚焦落在目标页上）：

```ts
  // 原型 showPanel：显示 100ms 后 editor.focus()。当前页非笔记页时 focus 不存在，自然跳过。
  setTimeout(() => pageRefs[activeId.value]?.focus?.(), 100)
```

模板 `<component>` 增加 `@archived="onArchived"`（放在 `@zen-exit` 之前）。

- [ ] **Step 6: 校验并提交**

```bash
pnpm typecheck
git add src/components/tag/TagList.vue src/windows/Panel/NotePage.vue src/windows/Panel/PanelApp.vue
git commit -m "feat(panel): 编辑器底栏文案 / 标签预览 / 归档收起 / 呼出聚焦"
```

---

## Task 6: 粘贴板卡片对齐原型（来源应用 / 类型徽章 / SVG 图标）

**Files:**
- Rewrite: `src/components/card/ClipCard.vue`
- Modify: `src/components/card/ClipArchiveCard.vue:50-55`
- Verify: `src/components/base/Icon.vue`（已含 `paste`，无需改）

**Interfaces:**
- Consumes: `Icon` 的 `close / pin / edit / paste / link`；`ClipboardEntry.source_app`（Task 2）。
- Produces: `ClipCard` props 只剩 `entry` / `confirming?`（删除 `archive`）；emits 不变。

- [ ] **Step 1: 核对图标**

Run: `grep -n "name === 'paste'" src/components/base/Icon.vue`
Expected: 命中一行（阶段二已按 `ICON_PASTE` 补齐，本任务不改 Icon.vue）。

- [ ] **Step 2: `ClipCard.vue` 重写**

```vue
<script setup lang="ts">
import { computed } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import Icon from '@/components/base/Icon.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import ClipTypeBadge from '@/components/clip/ClipTypeBadge.vue'
import type { ClipboardEntry } from '@/typings/domain'
import { formatStamp } from '@/utils/datetime'

/**
 * 面板剪贴板卡片（原型 renderClips 的 .clip-item）。
 *
 * - 右上角 `.card-close`（悬浮显示，二次确认走卡片上方浮层）；
 * - 头部 `.clip-head`：时间（置顶带 📌 前缀）→ 来源应用 `.clip-from`（spec D20，无值不渲染）→ 类型徽章；
 * - 正文最多两行，超出省略；
 * - 右下 `.clip-ops`：粘贴 / 打开链接(仅 link) / 编辑(仅文本类) / 收藏置顶，全部内联 SVG 图标；
 *   「粘贴」是直接粘贴到光标处（面板收起、焦点归还后模拟 Ctrl/Cmd+V）；
 * - 双击 = 粘贴到光标处并置顶；置顶条目金色高亮并优先排序（排序由调用方负责）。
 */
const props = withDefaults(defineProps<{ entry: ClipboardEntry; confirming?: boolean }>(), { confirming: false })

const emit = defineEmits<{
  (e: 'paste'): void
  (e: 'edit'): void
  (e: 'pin'): void
  (e: 'open-link'): void
  (e: 'ask-delete'): void
  (e: 'confirm-delete'): void
  (e: 'cancel-delete'): void
}>()

/** 时间口径：内容被修改过则显示最后修改时间。 */
const stamp = computed(() => formatStamp(props.entry.modified_at || props.entry.copied_at))
/** 仅文本类条目可编辑（图片无正文可改）。 */
const editable = computed(() => props.entry.content_type !== 'image')
const isLink = computed(() => props.entry.content_type === 'link')
</script>

<template>
  <li class="clip-item" :class="{ pinned: props.entry.pinned }" :title="props.entry.preview" @dblclick="emit('paste')">
    <ConfirmPopover
      v-if="props.confirming"
      text="⚠️ 确认删除该剪贴板条目？"
      @confirm="emit('confirm-delete')"
      @cancel="emit('cancel-delete')"
    />

    <button type="button" class="card-close" title="删除该条目" @click="emit('ask-delete')">
      <Icon name="close" />
    </button>

    <div class="clip-head">
      <span class="clip-time"
        ><template v-if="props.entry.pinned"><span class="ix">📌</span> </template>{{ stamp }}</span
      >
      <span v-if="props.entry.source_app" class="clip-from" title="来源应用"
        ><span class="ix">🌐</span> {{ props.entry.source_app }}</span
      >
      <ClipTypeBadge :content-type="props.entry.content_type" />
    </div>

    <div class="clip-text">{{ props.entry.preview || props.entry.content }}</div>

    <div class="clip-ops">
      <IconBtn title="粘贴（直接粘贴到光标处）" @click="emit('paste')"><Icon name="paste" /></IconBtn>
      <IconBtn v-if="isLink" variant="clip-open" title="用默认浏览器打开该链接" @click="emit('open-link')">
        <Icon name="link" />
      </IconBtn>
      <IconBtn v-if="editable" title="编辑内容" @click="emit('edit')"><Icon name="edit" /></IconBtn>
      <IconBtn
        :variant="props.entry.pinned ? 'active-pin' : ''"
        :title="props.entry.pinned ? '取消收藏' : '收藏置顶'"
        @click="emit('pin')"
      >
        <Icon name="pin" />
      </IconBtn>
    </div>
  </li>
</template>
```

- [ ] **Step 3: `ClipArchiveCard.vue` 加来源应用**

第 51 行 `<ClipTypeBadge …/>` 之后、时间 `<span>` 之前插入（原型 renderArchive 的顺序：类型 → 来源 → 时间）：

```vue
      <span v-if="props.entry.source_app" class="clip-from" title="来源应用"
        ><span class="ix">🌐</span> {{ props.entry.source_app }}</span
      >
```

- [ ] **Step 4: 校验并提交**

```bash
pnpm typecheck && grep -rn ":archive" src/windows/Panel/ClipPage.vue || echo "ClipPage 未传 archive"
git add src/components/card/ClipCard.vue src/components/card/ClipArchiveCard.vue
git commit -m "feat(panel): 粘贴板卡片对齐原型（来源应用 / 类型徽章 / SVG 图标）"
```

---

## Task 7: 失焦触发源并存、Esc 链、切页清确认、待办提示行、🔁 接线

**Files:**
- Modify: `src/windows/Panel/PanelApp.vue`（`onPointerEnter / onPointerLeave`、Esc 链、`watch(activeId)`）
- Modify: `src/windows/Panel/ClipPage.vue`
- Modify: `src/components/card/TodoTree.vue`
- Modify: `src/windows/Panel/TodoPage.vue`

**Interfaces:**
- Consumes: `PanelPageExpose.dismissOverlays`；`useConfirmDelete().pendingId / cancel`；`TodoPage.openEditor(mode, todo, parent, focus)`（既有）。
- Produces: `TodoTree` expose `dismissConfirm(): boolean`；`ClipPage` / `TodoPage` expose `dismissOverlays(): boolean`。

- [ ] **Step 1: `PanelApp.vue` 鼠标触发源（spec D17）**

第 301–307 行两个函数替换为：

```ts
/** 鼠标回到面板：取消待执行的收起（原型 panel.mouseenter）。 */
function onPointerEnter(): void {
  pointerInside.value = true
  clearCollapseTimer()
}

/**
 * 鼠标离开面板：按策略计时收起（原型 panel.mouseleave），与窗口 blur 并存（spec D17）。
 * Zen 铺满屏幕，鼠标离开即离开屏幕，不应收起；弹窗 / 独立编辑窗口期间同样忽略。
 */
function onPointerLeave(): void {
  pointerInside.value = false
  if (zen.value || modalDepth.value > 0 || externalEditorOpen.value) return
  scheduleCollapse()
}
```

（监听仍挂在 `document.documentElement`，原因见第 49–53 行注释：弹窗 Teleport 到 body 后不再是 `#panel` 后代。）

- [ ] **Step 2: `PanelApp.vue` Esc 链与切页副作用**

`onKeydown` 的 Escape 分支里，`if (zen.value) {…}` 之后、`void hide()` 之前插入：

```ts
    // 原型 Esc 链：有删除确认 / 浮层先只关它，不收面板。
    if (pageRefs[activeId.value]?.dismissOverlays?.()) {
      logger.debug('panel', 'Esc 关闭当前页浮层')
      return
    }
```

`watch(plugins, …)` 之后插入：

```ts
/** 切页副作用（原型 switchMode 的 clearPendingDelete）：关掉各页的删除确认与浮层——目标列表已隐藏。 */
watch(activeId, (_next, prev) => {
  if (!prev) return
  for (const page of Object.values(pageRefs)) page?.dismissOverlays?.()
})
```

- [ ] **Step 3: `ClipPage.vue` 暴露 `dismissOverlays`**

`remove()` 之后追加：

```ts
/** 关闭本页浮层：待确认删除 → 取消；编辑弹窗 → 关闭。关了任一项返回 true。 */
function dismissOverlays(): boolean {
  let dismissed = false
  if (confirm.pendingId.value) {
    confirm.cancel()
    dismissed = true
  }
  if (editing.value) {
    closeEdit()
    dismissed = true
  }
  return dismissed
}

defineExpose({ dismissOverlays })
```

- [ ] **Step 4: `TodoTree.vue` 暴露 `dismissConfirm`**

`confirmDelete` 之后追加：

```ts
/** 取消待确认的删除（面板 Esc 链 / 切页副作用用）；有待确认项返回 true。 */
function dismissConfirm(): boolean {
  if (!confirm.pendingId.value) return false
  confirm.cancel()
  return true
}

defineExpose({ dismissConfirm })
```

- [ ] **Step 5: `TodoPage.vue` 提示行、🔁 接线、`dismissOverlays`**

脚本：`const priorityFilter = …` 之后加

```ts
const tree = ref<InstanceType<typeof TodoTree> | null>(null)
```

`removeTodo` 之后追加：

```ts
/** 关闭本页浮层：删除确认在 TodoTree 内部，转调它。 */
function dismissOverlays(): boolean {
  return tree.value?.dismissConfirm() ?? false
}

defineExpose({ dismissOverlays })
```

模板：`.todo-input-row` 之后插入原型提示行：

```vue
    <div class="todo-panel-hint">
      搜索 / 优先级过滤当日待办 · 逾期事项自动置顶 · 点击优先级徽章可调整 · 点击 📅 新增待办
    </div>
```

`<TodoTree` 加 `ref="tree"`，并在 `@edit-remind` 之后加一行（🔁 重复菜单本身留阶段四，先接到独立编辑窗的提醒区）：

```vue
      @edit-repeat="openEditor('edit', $event, null, 'remind')"
```

- [ ] **Step 6: 校验并提交**

```bash
pnpm typecheck && pnpm exec vite build 2>&1 | grep -E "built in|error"
git add src/windows/Panel/PanelApp.vue src/windows/Panel/ClipPage.vue src/components/card/TodoTree.vue src/windows/Panel/TodoPage.vue
git commit -m "feat(panel): 失焦触发源并存、Esc 链、切页清确认、待办提示行"
```

---

## Task 8: 实机验收与记录

**Files:**
- Create: `docs/superpowers/plans/2026-09-11-prototype-realign-phase3-acceptance.md`
- Modify: spec §9；必要时 `src/styles/extensions.css`（新开「§10 阶段三验收补丁」分节注明原因）

- [ ] **Step 1: 静态检查全绿**

Run: `pnpm sync:styles --check && pnpm typecheck && pnpm test && cargo fmt --manifest-path src-tauri/Cargo.toml --check && cargo test --manifest-path src-tauri/Cargo.toml && pnpm exec vite build`
Expected: 全部通过；记录 `cargo test` 通过数与 `panel-*.js` 体积。

- [ ] **Step 2: 实机（`pnpm tauri:dev` 后台，沿用 `.tmp/win.ps1` 驱动；打字机 + 深色各一轮）按 spec §6 九项逐条验证并截图**

1. 圆点：三色矢量圆、hover / active 光环、⌃1/2/3 切页、Tab 焦点环、`aria-selected` 随切换。
2. 笔记页：暂存三段文案「已暂存 → 输入中…（金色）→ 已暂存 SQLite」；标签预览无标签 / ≤3 / +N 三形态，+N 打开管理弹窗且副标题为「当前正在编写的念头（未归档）的标签」；归档后 toast「念头已归档 ✔」并 250ms 收面板；呼出后光标在编辑器。
3. 回显：主窗口笔记页 ✏️ → 主窗口隐藏、面板显示、内��与标签回显、按钮「保存修改 ✓」、toast「内容已回显…」、Zen 按钮出现、标签管理副标题「当前笔记的标签」；改内容点保存 → toast「修改已保存 ✔」、主窗口列表更新、面板收起、再呼出显示草稿；再回显后不保存直接 Esc → 修改丢弃、草稿恢复；日期详情页 ✏️ 同样路径；原草稿在回显前已落库（查 `notes` 表 `is_draft=1` 行）。
4. Zen：点 Zen → 面板铺满光标屏工作区（不遮任务栏）、导航 / 标签 / 暂存态隐藏、编辑器 18px、底栏只剩「退出 Zen」+「保存修改 ✓」；Esc 退出还原尺寸与位置（顶部居中、高度为进入前值）；Zen 中鼠标移出屏幕不收起；Zen 中点「退出 Zen」按钮同样还原；多屏时在外接屏呼出再进入 Zen 落在外接屏。
5. 失焦：鼠标移出面板 3 秒收起、3 秒内移回取消；Alt+Tab 切到别的窗口 3 秒收起；设置「立即」/「固定不收起」两档行为；弹窗打开期间移出不收起。
6. Esc 链：粘贴板页 / 待办页有删除确认时 Esc 只关确认，再按 Esc 才收面板；笔记页标签管理开着时 Esc 只关弹窗；切页时上一页的删除确认自动消失。
7. 粘贴板页：从记事本复制一段 → 面板与主窗口卡片显示「🌐 Notepad」；从 VS Code 复制显示「Code」；面板内手动捕获（若有入口）不显示来源；类型徽章、置顶 📌 前缀、四枚 SVG 图标 hover 显示、双击粘贴到记事本光标处。
8. 待办页：提示行文案；🔁 徽章点击打开独立编辑窗且焦点在提醒下拉。
9. `data-window='panel'` 生效后面板内弹窗（标签管理 / 粘贴板编辑）整页化观感：顶对齐、铺满、标题与底栏常显；若观感倒退，删除 `window-fit.css` 第 122–161 行「面板内的编辑弹窗」整组规则并记录。

- [ ] **Step 3: 写验收记录并回填 spec §9**

验收记录按阶段二 `2026-09-10-prototype-realign-phase2-acceptance.md` 的结构：环境 / 方法 → 自动化校验表 → 逐项比对表（打字机、深色各一列）→ 发现的问题与补丁清单 → 未验证项。spec §9 三条「待填」改为：Zen 放大 / 还原结论（含多屏）、来源应用采集命中率（记事本 / VS Code / 浏览器 / 提权进程各试一次）、补丁清单（含 `data-window='panel'` 去留）。

- [ ] **Step 4: 提交**

```bash
git add docs/superpowers/plans/2026-09-11-prototype-realign-phase3-acceptance.md docs/superpowers/specs/2026-09-11-prototype-realign-phase3-panel-design.md src/styles/extensions.css src/styles/window-fit.css
git commit -m "docs(design): 阶段三实机验收记录"
```

---

## 自检记录

- **Spec 覆盖**：§3 差异表 1–24 → Task 3（1, 2）、Task 4（3, 4, 12, 13 的 Zen 部分、22 的注释改 90）、Task 1（12 的后端、4 的后端）、Task 5（7, 8, 9, 10, 11）、Task 6（17, 18）、Task 7（14, 15, 16, 20, 21）、Task 3（23）；5 / 6 / 19 / 24 为「保留」项无任务；§4.1 → Task 3；§4.2 → Task 1 + Task 4；§4.3 → Task 1（后端）+ Task 4（前端 + window-fit）；§4.4 → Task 7；§4.5 → Task 2（后端 + 类型）+ Task 6（渲染）；§4.6 → Task 6；§4.7 → Task 5（TagList / 副标题）、Task 7（TodoPage / dismissOverlays）、Task 3（panel.ts）；§5 测试 → Task 2 三条单测；§6 → Task 8；§7 八个批次 ↔ 八个 Task 一一对应。
- **类型与函数名一致性**：`PanelIntent { page; noteId? }` 在 Task 1 定义、Task 4 `consumeIntent` 使用；`PanelPageExpose` 在 Task 4 Step 1 定义，NotePage（Task 4）/ ClipPage / TodoPage（Task 7）的 expose 与之匹配（`focus / dismissOverlays / loadNote / onPanelHide / isEditing`）；`pageRefs` / `setZen` / `zen` / `visible` 在 Task 4 定义，Task 5（`onArchived` 用 `hide`、聚焦用 `pageRefs`）与 Task 7（`onPointerLeave` 用 `zen`、Esc 链用 `pageRefs`）复用同名；`TodoTree.dismissConfirm` 在 Task 7 Step 4 定义、Step 5 调用；`api.windows.panelSetZen / panelOpenNote / panelTakeIntent`、`api.notes.get` 在 Task 1 定义，Task 4 / 5 使用；Rust `Capture.source_app` 在 Task 2 Step 6 定义，Step 1 测试与 Step 7 watcher 使用同名；`ClipboardEntry::source_app()` getter 由 `dto!` 宏生成，测试按此调用。
- **占位扫描**：无 TBD / 「类似于」/ 「适当处理」；Task 8 按要求只列步骤不含代码。
- **自检发现并就地修正**：
  1. spec §4.2 把 `PanelPageExpose.onPanelHide` 写成 `(): void`，但 NotePage 的 `onPanelHide` 需要 `await loadDraft()`，且 PanelApp 必须在 `panelHide` 前等它完成（隐藏后 WebView2 挂起，IPC 回包延迟）——接口放宽为 `void | Promise<void>` 并在 `hide()` 用 `Promise.all` 等待。
  2. spec §4.3 写 `pageRefs.note?.isEditing?.value`：`defineExpose` 的 ref 经模板 ref 拿到的实例代理会被 `proxyRefs` 自动解包，实际拿到的是 boolean——接口按 `isEditing?: boolean` 声明并在注释里说明，计算属性直接读 `pageRefs.note?.isEditing`。
  3. spec §4.6 说「Icon.vue 若缺 paste 图标则补」：核对 `Icon.vue` 第 23–31 行已含 `paste`（阶段二按 `ICON_PASTE` 补齐），Task 6 改为只核对不改文件。
  4. spec §5 称 `resolvePlugins` 已有单测覆盖：仓库里 `src/panel-plugins/` 下没有 `*.test.ts`，该陈述不成立；本阶段只改 `dot → dotClass` 不动 `resolvePlugins` 逻辑，不补测试（YAGNI），在此记录。
  5. `SettingsView.vue:279` 仍渲染 `plugin.dot`，改名后会类型报错——Task 3 补上该行改动。最初想复用 `.nav-dot.dot-*` 矢量圆做颜色示意，但生成层 `.nav-dot` 是 `display: grid` 的块级盒，内联进 label 会把文字挤到下一行，改为只留文字。
  6. `TodoPage` 自身没有 `useConfirmDelete`，删除确认在 `TodoTree` 内部——Task 7 让 `TodoTree` expose `dismissConfirm()`，`TodoPage.dismissOverlays` 转调；`TodoTree` 同时被主窗口使用，新增 expose 对其无影响。
  7. `TagList` 的 chips 点击会 emit `open`，若面板同时保留 `@open` 与 `.tag-preview` 的 `@click` 会打开两次——Task 5 移除 `@open`，一律靠冒泡到 `.tag-preview`。
  8. `NotePage.loadDraft` 原实现在「没有草稿」时直接 return，退出编辑态后编辑器会残留笔记内容——Task 4 改为无可用草稿时清空。
  9. 程序性写入 `content / tags`（恢复草稿 / 回显 / 归档清空）会触发自动暂存 watch 并把文案打成「输入中…」——Task 4 引入 `suppressAutosave` + `setLocal()`，Task 5 的三段文案建立在其上。
  10. Zen 退出时若只加 `height: 100vh`，`window-fit.css` 的 `#panel { clip-path: inset(0 round 8px) }` 仍会在铺满的窗口四角裁出圆角——Task 4 Step 6 追加 `clip-path: none`（生成层已有 `border-radius: 0`）。
  11. `panel_set_zen` 按用户硬约束改为 `pub async fn`（spec 未标 async），理由写在命令注释里；`note_get` / `panel_take_intent` 无窗口操作保持同步。
  12. `#[cfg]` 属性块内写 `return` 的坑（`models.rs` 注释已记）：`foreground_app_name` 拆成 `foreground_app_path()` 的两个平台版本 + 纯函数 `app_name_from_path`，避免非 Windows 平台出现不可达代码告警，纯函数顺带可测。
