# Inkling 架构设计文档

> **版本**：v2.1（对齐需求规格 v1.2；已完成文档审查第一轮）
> **日期**：2026-08-29
> **文档状态**：审查中；实现前必须先完成数据模型、提醒语义与窗口边界确认
> **技术栈**：Tauri 2 + Rust + Vue 3 + TypeScript + UnoCSS + Vite + SQLite(rusqlite) + GSAP
> **关联文档**：[Inkling（念头捕手）需求规格说明书](./Inkling（念头捕手）需求规格说明书.md)
> **原型说明**：`doc/index.html` 为静态交互原型，使用原生 HTML/CSS/JavaScript 与 CDN GSAP，不等同于本架构的 Tauri/Vue/Rust 实现。

---

## 1. 架构总览

### 1.1 架构形态

单进程多窗口 Tauri 应用（业务归档/统计/设置仍由一个 `main` 窗承载）。**核心窗口（hotzone、panel、main）随应用启动预创建并按需显示**，呼出只做 `show + focus`，这是「< 1 秒信条」的架构根基。置顶浮窗和提醒浮窗属于瞬态窗口，按需创建、复用或销毁，不能笼统表述为“所有窗口预创建”。

```
┌─────────────────────────────────────────────────────────────┐
│                        屏幕顶部中央                          │
│  ┌──────────────────────┐                                    │
│  │  hotzone 感应窗        │  240×80 透明常驻 (alwaysOnTop)    │
│  └──────────────────────┘                                    │
│  ┌──────────────────────────┐                                │
│  │  panel 主面板窗           │  480×自适应(120-600)           │
│  │  笔记 🔴 / 粘贴板 🟡 / 待办 🟢 │  预创建常驻，hide↔show       │
│  └──────────────────────────┘                                │
│                                          ┌─────────────┐     │
│  ┌─────────────────────────────┐         │ pinned-N    │     │
│  │ main 单窗口 900×600         │         │ 置顶浮窗     │ ×N  │
│  │ 归档 / 统计 / 设置视图      │         │ 220×~120    │     │
│  └─────────────────────────────┘         └─────────────┘     │
│                                          ┌─────────────┐     │
│                                          │ reminder    │     │
│                                          │ 右上角提醒卡  │ ×N  │
│                                          └─────────────┘     │
└─────────────────────────────────────────────────────────────┘

┌─ Tauri Rust 核心 ─────────────────────────────────────────┐
│  app/      窗口生命周期：tray / shortcut / hotzone / panel  │
│            / main(归档/统计/设置视图) / pinned / reminder /  │
│            window_manager / launch                         │
│  ipc/      #[tauri::command] 命令层 + 事件常量              │
│  domain/   纯业务逻辑：capture / clipboard(分类去重)       │
│            / todo(状态机) / stats(聚合) ← 可单测           │
│  data/     SQLite: db(连接+迁移) / notes / clipboard /     │
│            todos / tags / settings / stats / file_store    │
│  services/ export(md/txt/pdf/png/jpg/html) / reminder      │
│            / stats_scheduler / file_store                  │
└────────────────────────────────────────────────────────────┘
```

### 1.2 数据流

### 1.3 当前分支实现状态

本分支从提交 `65bb5fd5fdec81be991a36d9c31d6ab57f5129ba` 创建，正式实现采用 Tauri 2 + Vue 3 + TypeScript + Vite，遵循 Tauri 标准目录：项目根目录放置前端源码与 npm 工程，`src-tauri/` 放置 Rust 后端和 Tauri 配置；原 `doc/` 静态原型仅作为视觉和交互参考。当前已落地的实现边界如下：

| 层次 | 当前实现 | 说明 |
| --- | --- | --- |
| 前端 | 根目录 `src/` Vue 单页应用 | 笔记、剪贴板、待办、统计、设置均在同一窗口内切换；优先级使用锚定式三选一 Popover |
| IPC | `src/api.ts` + Tauri commands | 渲染进程不直接访问 SQLite |
| 数据 | `src-tauri/src/main.rs` + SQLite WAL | 已包含 notes、tags、clipboard_entries、todos、todo_tags、settings 表 |
| 文件 | Tauri `app_data_dir/notes/` | 笔记超过 1MB 时使用临时文件替换并保存相对路径；从外置文件恢复为小文本时清理旧文件 |
| 领域约束 | Rust command 层 | 子任务层级、最多 5 个子任务、已完成项只允许新增子任务、优先级限制和统计逾期日期口径在后端校验 |

当前已接入系统托盘、全局快捷键、文本剪贴板 500ms 轮询和开机启动插件；仍需补齐顶部 hotzone 与多窗口状态机、提醒调度、置顶浮窗、图片/富文本剪贴板、导出、主题完整集合和自动化测试。未完成能力必须在 README 和发布说明中明确标注，不得以原型交互代替验收。


```
Vue 视图 ──invoke──▶ ipc/commands ──▶ domain(纯逻辑) ──▶ data(SQL/文件) ──▶ SQLite / app-data/notes/
   ▲                                                                     │
   └──────────── emit 事件 ◀── domain/data 变更后广播 ◀───────────────────┘
```

铁律：**渲染进程不直接碰 SQL，Rust 不直接碰 DOM**；domain 层零 Tauri 依赖（可脱离框架单测）。

---

## 2. 关键技术决策（ADR）

### ADR-001 顶部感应区：常驻透明感应窗口（非鼠标轮询）
- **决策**：创建 240×80 的透明 `hotzone` 窗口，钉在屏幕顶部中央，`alwaysOnTop + skipTaskbar + decorations(false) + focusable(false)`，监听前端 `mouseenter` 事件触发展开。
- **否决方案**：Rust 端轮询 `mouse_position` crate —— 即便 20ms 轮询也有持续 CPU 占用，违背「安静待命」信条。
- **状态联动**：面板展开期间感应窗 `set_ignore_cursor_events(true)`（防误触）；面板收起后恢复。
- **防误触**：hover 防抖 100ms（需求定义）；鼠标快速划过不展开。

### ADR-002 展开动画：窗口瞬移 + 前端 GSAP 物理动画
- **决策**：Rust 只做 `set_position(最终坐标) + show()`；滑入动效由前端根容器 GSAP `transform: translateY(-12px)→0 + opacity` 完成（200ms ease-out）。
- **理由**：GSAP 支持物理弹性（Spring）与错帧（Stagger），天然 60fps；Rust 逐帧 `set_position` 在 Windows 上有撕裂风险。
- **收起**：前端反向动画 150ms → 结束后 `invoke('panel_hide')` → Rust `hide()`。

### ADR-003 焦点策略：macOS NSPanel，Windows 原生无边框
- **痛点**：macOS 普通 NSWindow `show` 时会激活本应用、挤掉用户当前应用焦点——违背「不切换当前应用」信条。
- **决策**：macOS 经 `tauri-plugin-nspanel` 把 panel 窗转为 `NSPanel`（non-activating panel，Spotlight/Quick Note 同款：键盘焦点可得、应用不激活）。Windows 无边框窗口 `show + set_focus` 行为天然正确。
- **失焦收起**：监听窗口 `blur` 事件 → 按设置（立即/延迟3s/固定）收起。

### ADR-004 毛玻璃：window-vibrancy
- **决策**：`window-vibrancy` crate；macOS `NSVisualEffectMaterial::HudWindow`（深色）/`Popover`（浅色自适应用 `UnderWindowBackground`）；Windows 11 `Mica`，Windows 10 降级 `Acrylic`。
- **前提**：`tauri.conf.json` 窗口 `transparent: true` + Cargo `tauri` feature `macos-private-api`。

### ADR-005 混合存储：SQLite 缓冲 + 大文件自动落盘
- **决策**：`rusqlite`（bundled/chrono/serde_json），单连接驻留 `tauri::State`，`PRAGMA journal_mode=WAL`、`synchronous=NORMAL`。
- **自动保存**：前端输入防抖 500ms → invoke 落盘 SQLite 暂存。
- **归档策略**：点击「归档」按钮时，若内容 ≤ 1MB 则继续存 SQLite；若 > 1MB 则自动写入应用数据目录 `notes/` 下的 `.md` 文件，SQLite 仅保留相对路径引用与元数据。

### ADR-006 剪贴板监听：轮询 + 哈希去重 + 回声抑制 + 30 天清理
- **决策**：`arboard` 每 500ms 轮询系统剪贴板；内容 SHA-256 哈希去重；应用自身写回剪贴板时记录 echo 哈希，下一次轮询命中即跳过（防自记录）。
- **分类**：纯 Rust 启发式（URL 正则 / 代码特征 / 图片 / 富文本 / 纯文本）。
- **保留策略**：tokio 定时任务每 24h 清理超过 30 天的记录。

### ADR-007 前端：单 SPA + window label 视图路由 + ProseMirror 内核
- **决策**：一个 Vite 构建产物；入口按 `getCurrentWindow().label` 分发到 `PanelView / HotzoneView / PinnedView / MainView / ReminderView`。`SettingsView` 与统计视图属于 `MainView` 的内部路由，不再创建独立 settings 窗口。
- **编辑器内核**：采用 **ProseMirror** 实现 Typora 级即时渲染（所见即所得，光标移入展示语法标记，移出渲染最终效果）。
- **组件策略**：panel 窗为极致首屏只引 UnoCSS 自研组件 + GSAP 动画；main 窗内部的归档/统计/设置视图引 Naive UI。Vite 按视图分包，Naive UI 不进 panel chunk。

### ADR-008 托盘图标主题自适应
- **macOS**：template image（系统自动控制黑白），`trayIcon.set_icon_as_template(true)`。
- **Windows**：监听 `darkmode` 变化事件切换预置的亮/暗两套 ICO。

### ADR-009 待办提醒：右上角自定义卡片（非系统通知）
- **决策**：到期时创建 `reminder` 无边框窗口（320×200，右上角定位，alwaysOnTop），展示提醒内容 + 操作按钮（关闭/选择下次提醒时间）。
- **理由**：系统通知无法提供「选择下次提醒时间」的交互，且部分平台限制频繁通知，违背「安静待命」但需保证提醒可达。

### ADR-011 优先级变更：锚定式三选一选择器
- **决策**：待办/子任务卡片使用当前优先级徽章作为触发器，打开固定展示“高/中/低”的锚定式 popover；当前值使用勾选、描边和 `aria-selected` 表示，颜色与文字同时展示。默认向下定位，空间不足整体翻转到上方。
- **交互**：支持鼠标、键盘 Enter/Space 打开、方向键/Home/End 导航、Enter 提交、Esc/外部点击取消；提交后返回触发器焦点。
- **数据边界**：优先级更新是目标条目的独立事务，只写 `priority` 和 `updated_at`；父子不级联，计划时间、提醒、状态、标签和备注保持不变。已完成条目不提供可交互触发器。
- **失败处理**：写入失败时保留旧优先级、恢复菜单状态并提示错误，不允许出现 UI 已变更但数据库未变更的假成功。

### ADR-010 使用统计：异步聚合表 + ECharts 渲染
- **决策**：采用“幂等事件 + 每日聚合”策略：笔记成功归档、剪贴板成功捕获新条目、待办创建/实际完成分别写入可去重的活动事件，再异步聚合；待办总数和逾期数按业务数据查询，不把“置顶”当作统计口径。
- **展示**：前端使用 **ECharts** 渲染日历热力图与每月趋势折线图。

---

## 3. 窗口系统设计

> **单窗口约束**：`main` 窗口内部承载笔记归档、剪贴板归档、待办归档、统计和偏好设置；统计/设置切换不得创建新的业务窗口。`panel` 的 always-on-top 层级高于 `main`，但编辑弹窗的生命周期必须由窗口管理器统一登记，避免 panel 因 blur 误收起。

### 3.1 窗口属性矩阵

| label | 尺寸 | transparent | alwaysOnTop | decorations | skipTaskbar | focusable | 生命周期 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `hotzone` | 240×80 | ✓ | ✓ | ✗ | ✓ | ✗ | 常驻显示 |
| `panel` | 480×自适应 | ✓ | ✓ | ✗ | ✓ | NSPanel | 常驻隐藏⇄显示 |
| `pinned-{noteId}` | 220×~120 | ✓ | ✓ | ✗ | ✓ | ✓ | 按需创建/销毁 |
| `main` | 900×600 | ✗ | ✗ | ✓(隐藏标题栏) | ✗ | ✓ | 常驻隐藏⇄显示 |
| `reminder-{todoId}` | 320×200 | ✓ | ✓ | ✗ | ✓ | ✓ | 按需创建/销毁 |

### 3.2 panel 窗口状态机

```
Hidden ──hotzone hover 100ms / 全局快捷键──▶ Showing(GSAP 滑入 200ms)
  ▲                                            │
  │                                   blur / Esc / 点击归档按钮
  │                                            ▼
  └──── hide() ◀── invoke('panel_hide') ◀── Hiding(GSAP 滑出 150ms)
```

### 3.3 定位计算

- 取**鼠标所在显示器**（多屏场景）`current_monitor` → `position/size/work_area`；快捷键呼出时也取当前鼠标所在显示器，不能固定主屏。
- `x = monitor.x + (monitor.width - 480) / 2`；`y = work_area.y`，并按平台菜单栏、刘海、任务栏和安全边距修正。
- 每个显示器都必须有独立 hotzone，或在显示器变化时动态迁移 hotzone；只创建主屏 hotzone 不满足多屏需求。
- reminder 窗：`x = monitor.x + monitor.width - 320 - 24`，`y = work_area.y + 24`；原公式遗漏 `monitor.x`，在副屏会定位错误。
- 所有计算统一使用物理像素/逻辑像素的明确转换，避免高 DPI 下偏移。

---

## 4. 数据层设计（SQLite Schema + 文件存储）

> 以下模型以需求规格 v1.2 为准。UI 所称“完成时间”在数据层统一命名为 `due_at`（计划完成/截止时间）；用户勾选完成的实际时刻另存为 `completed_at`。所有时间使用带时区的 ISO8601 值，展示时转换为用户当前时区。

```sql
-- 笔记（正文为 Markdown；标签是独立元数据，不从正文解析 #tag）
CREATE TABLE IF NOT EXISTS notes (
  id          TEXT PRIMARY KEY,
  content     TEXT,                       -- <=1MB 时存正文；外部文件时可为空
  plain_text  TEXT NOT NULL DEFAULT '',    -- 搜索索引
  file_path   TEXT,                       -- 相对于应用数据目录的 notes/ 路径
  is_draft    INTEGER NOT NULL DEFAULT 0,
  pinned      INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  name          TEXT NOT NULL,
  normalized    TEXT NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS note_tags (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (note_id, tag_id)
);

-- 剪贴板历史；图片/超长内容应使用文件引用，避免把 dataURL 无限制写入 TEXT
CREATE TABLE IF NOT EXISTS clipboard_entries (
  id           TEXT PRIMARY KEY,
  content_type TEXT NOT NULL CHECK (content_type IN ('text','link','code','image','richtext')),
  content      TEXT,
  file_path    TEXT,                       -- 图片或超大内容相对于应用数据目录的路径
  preview      TEXT NOT NULL DEFAULT '',
  content_hash TEXT NOT NULL,
  pinned       INTEGER NOT NULL DEFAULT 0,
  copied_at    TEXT NOT NULL,
  modified_at  TEXT NOT NULL,
  created_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_clipboard_hash ON clipboard_entries(content_hash);

-- 待办与一级子任务；子任务不可再嵌套由 domain 层校验，最多 5 个子任务也由 domain 层校验
CREATE TABLE IF NOT EXISTS todos (
  id            TEXT PRIMARY KEY,
  content       TEXT NOT NULL,
  due_at        TEXT NOT NULL,                  -- UI 的“完成日期 + 完成时刻”
  completed_at  TEXT,                            -- 实际勾选完成时刻
  status        TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open','done')),
  remind_at    TEXT,                             -- 下一次提醒时间
  repeat_rule  TEXT CHECK (repeat_rule IN ('daily','weekly') OR repeat_rule IS NULL),
  priority      TEXT NOT NULL DEFAULT 'medium'
                CHECK (priority IN ('high','medium','low')),
  remark        TEXT NOT NULL DEFAULT '',
  parent_id     TEXT REFERENCES todos(id) ON DELETE CASCADE,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_todos_due ON todos(due_at, status);
CREATE INDEX IF NOT EXISTS idx_todos_parent ON todos(parent_id);
CREATE TABLE IF NOT EXISTS todo_tags (
  todo_id TEXT NOT NULL REFERENCES todos(id) ON DELETE CASCADE,
  tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (todo_id, tag_id)
);

-- 可重放/去重的活动事件；daily 聚合可由事件重建
CREATE TABLE IF NOT EXISTS activity_events (
  id          TEXT PRIMARY KEY,              -- 幂等事件 ID
  event_type  TEXT NOT NULL,                 -- note_archived/clipboard_captured/todo_created/todo_completed
  entity_id   TEXT NOT NULL,
  occurred_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS stats_daily (
  date                    TEXT PRIMARY KEY,
  note_archived_count     INTEGER NOT NULL DEFAULT 0,
  clipboard_captured_count INTEGER NOT NULL DEFAULT 0,
  todo_created_count      INTEGER NOT NULL DEFAULT 0,
  todo_completed_count    INTEGER NOT NULL DEFAULT 0,
  updated_at              TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

**数据约束与文件存储约定**：
- 标签新增前统一按 Unicode 规范化并去重；笔记标签最多 3 个、每个最多 5 字，待办标签最多 3 个、每个最多 10 字。
- 剪贴板去重必须在事务内完成；编辑内容后重新计算哈希，遇到已有同哈希条目时按产品策略合并或拒绝，不能留下错误的唯一索引。
- `notes/` 和剪贴板附件位于应用数据目录，默认不写项目安装目录；文件写入采用临时文件 + 原子替换，数据库记录与文件记录失败时可恢复。
- 草稿自动保存使用稳定 `id`；`note_commit` 需要把草稿提升为正式笔记并清理草稿标记，数据库与外部文件引用必须可回滚。
- 待办总量、已完成数、逾期数由 `todos` 的 `due_at/status/completed_at` 查询得出；它们不是只靠 `stats_daily` 的计数列推断。
- 优先级变更只更新目标待办/子任务的 `priority` 与 `updated_at`，不级联父子层级、不改变 `due_at`、`remind_at`、状态或其他元数据；事务成功后发送 `todos:changed`，不写入活跃度统计事件。
- 已完成父待办新增子任务必须在同一事务内完成：插入未完成子任务、将父待办 `status` 改为 `open`、清空父待办 `completed_at`，并将父待办 `due_at` 更新为 `max(原父级 due_at, 新子任务 due_at)`；若原父级计划时间较晚则不提前。

---

## 5. 核心交互时序

### 5.1 快捷键呼出 → 落屏（预算分解 < 1s）

```
用户按键                                         t=0
tauri-plugin-global-shortcut 回调                 t+5ms
panel.show() + focus()（窗口常驻，无 WebView 重建） t+50ms
前端已挂载 → GSAP 滑入动画 + 输入框 autofocus      t+80ms
用户可打字                                        t+100ms ✔
输入停止 500ms → invoke('note_autosave') 草稿落盘 SQLite
点击「归档」按钮 → invoke('note_commit')
  → ≤1MB: 直接写 SQLite
  → >1MB: 写入应用数据目录 `notes/{id}.md` + SQLite 存相对路径引用
  → 写入与数据库更新成功后清理草稿标记（失败可恢复）
→ emit('notes:changed') → GSAP 滑出收起
```

### 5.2 鼠标触顶展开

```
hotzone mouseenter → 100ms 防抖计时 → invoke('panel_show')
→ panel.show() + hotzone.set_ignore_cursor_events(true)
→ 面板失焦/收起后 → hotzone.set_ignore_cursor_events(false)
```

### 5.3 剪贴板捕获

```
tokio 500ms 轮询 → arboard.get() → SHA-256
→ 命中 echo 哈希 → 跳过（防自记录）
→ 命中 DB 已有哈希 → 跳过
→ 否则 classify() 分类 → 入库（copied_at=now, modified_at=now）
→ emit('clipboard:changed')
```

### 5.4 粘贴板内容修改

```
用户修改内容 → invoke('clipboard_update', { id, content })
→ 更新 content + modified_at=now（copied_at 保持不变）
→ emit('clipboard:changed')
```

### 5.5 待办提醒触发

```
tokio 定时任务（每分钟检查 `remind_at <= now`，按带时区时间比较）
→ 在事务内抢占/标记提醒实例（幂等，防重启或轮询重复弹出）
→ 创建或复用 reminder 窗（右上角 320×200，alwaysOnTop）
→ 用户操作：
  - 完成/关闭 → 更新待办状态或清除 `remind_at`，停止后续提醒
  - 选择下次提醒时间 → 只更新 `remind_at`，保留 `repeat_rule`，关闭窗口
→ 无明确提醒时间时由 due_at 计算默认的前30分钟、前5分钟、到点三次提醒
```

### 5.6 已完成父待办新增子任务

```
用户点击已完成父待办的「＋子任务」
→ 允许打开新增弹窗；新子任务默认未完成，due_at=当前时间+1小时
→ 校验任务内容、时间和提醒字段；已完成父级豁免“子任务 due_at 不得晚于父级”
→ 开启数据库事务
   → 插入新子任务
   → 父待办 status=open，completed_at=NULL
   → 父待办 due_at=max(原父级 due_at, 新子任务 due_at)
→ 事务提交成功后发送 todos:changed
→ 任一步失败则整体回滚，父待办保持已完成状态且不留下孤立子任务
```

### 5.7 使用统计写入

```

### 5.8 待办优先级变更

```
用户聚焦未完成待办/子任务的优先级徽章
→ 打开锚定式三选一 popover（高/中/低，当前值勾选）
→ 鼠标点击或键盘 Enter 提交；Esc/外部点击取消
→ 开启事务，仅更新目标记录 priority + updated_at
→ 提交成功后 emit('todos:changed')，前端按 due_at → priority → created_at 稳定重排
→ 写入失败则回滚并恢复原优先级，toast 提示错误
```
业务事务成功提交
→ 写入带幂等 ID 的 `activity_events`（归档笔记/捕获新剪贴板/创建待办/实际完成待办）
→ 异步聚合 `stats_daily`（不阻塞主线程，可由事件重建）
→ 前端查询业务数据 + 聚合数据 → ECharts 渲染热力图/趋势图
```

---

## 6. 平台适配层

| 能力 | macOS | Windows |
| --- | --- | --- |
| 状态栏 | tray template 图标（自动黑白） | System Tray + 亮暗双 ICO 手动切 |
| 左键交互 | 单击 → 弹出「历史归档」页 | 单击 → 弹出「历史归档」页 |
| 面板焦点 | `tauri-plugin-nspanel`（不激活当前应用） | 原生无边框 `show+focus` |
| 毛玻璃 | `NSVisualEffectMaterial::UnderWindowBackground` | Win11 `Mica` / Win10 `Acrylic` 降级 |
| 贴顶定位 | 主屏 `visible_frame` 顶部（菜单栏下沿即屏顶） | 工作区顶部 |
| 全局快捷键 | `Cmd+Shift+Space`（冲突时引导用户改键） | `Ctrl+Shift+Space` |
| 开机启动 | LaunchAgent（静默，无 Dock 图标） | 注册表 Run 键（静默启动） |

平台差异收敛在 `app/platform.rs`（条件编译 `cfg!(target_os)`），业务层无感知。

---

## 6.1 原型与正式实现边界（审查结论）

- `doc/index.html` 当前是原生 HTML/CSS/JavaScript 静态原型，使用 CDN 加载 GSAP；它没有实现 Tauri IPC、SQLite、本地文件、真实系统剪贴板、系统托盘、全局快捷键、跨屏窗口或开机启动。需求文档不得再表述为“已同步实现全部交互”。
- 原型的 CDN 依赖与正式实现的 `default-src 'self'` CSP 冲突。正式实现必须把 GSAP 等依赖打包到应用资源中，或调整并审查 CSP；原型仅用于视觉/交互走查。
- 原型模拟了部分数据和“退出”提示，不能作为持久化、权限、失败恢复、通知去重和性能指标的验收依据。

---

## 7. 性能预算与验证

| 指标 | 预算 | 验证方式 |
| --- | --- | --- |
| 冷启动至托盘就绪 | < 400ms | 启动日志计时 |
| 呼出至可输入 | < 300ms（目标 100ms） | DevTools Performance |
| 展开动画（GSAP） | 稳定 60fps | Performance 帧率 |
| 常驻内存 | < 80MB | 活动监视器 / 任务管理器 |
| 安装包体积 | < 15MB | 构建产物 |
| SQLite 查询响应 | < 10ms | 日志埋点 |
| 大文件落盘（1MB） | < 50ms | 日志埋点 |

---

## 8. 安全与隐私

- CSP：`default-src 'self'`，无远程资源、无遥测上报。
- 数据库落盘于应用数据目录，用户可在设置中一键打开/备份。
- 剪贴板条目支持「保留天数」自动清理（默认 30 天，tokio time 调度）。
- 超大笔记落盘到应用数据目录 `notes/`；用户自定义路径必须显式授权，路径不可写时回退到应用数据目录，并以原子写入保证恢复。

---

## 9. 测试策略

| 层 | 方式 | 覆盖 |
| --- | --- | --- |
| domain（独立标签约束/剪贴板分类/待办状态机/优先级变更/稳定排序/统计聚合） | `cargo test` 纯单测 | 全部分支 |
| data（CRUD/迁移/文件落盘） | `cargo test` + 内存 SQLite + 临时目录 | 全表 |
| 前端 stores/composables | vitest | 核心状态流 |
| 端到端手工回归 | 清单制 | 呼出/三态/pin/托盘/失焦/多屏/提醒/统计/优先级弹出菜单（鼠标键盘、翻转定位、失败恢复） |

---

## 10. 里程碑任务拆解

### P0（MVP）
1. 工程脚手架（Tauri2+Vue3+TS+Vite+UnoCSS+GSAP+vitest）
2. SQLite 初始化 + 迁移 + notes CRUD + 混合存储（≤1MB SQLite / >1MB 落盘）
3. panel 窗：预创建/定位/show/hide/GSAP 滑入动画/NSPanel
4. hotzone 感应窗 + hover 防抖联动
5. 全局快捷键注册/改键
6. 托盘（左右键菜单/图标主题/左键弹历史归档页）
7. 笔记模式：ProseMirror 即时渲染 + 500ms 自动保存 + 点击归档按钮 + 独立标签元数据维护（不解析正文 `#tag`）
8. 失焦收起（立即/3s/固定）
9. 开机自启动（静默启动，macOS LaunchAgent / Windows Run 键）

### P1
10. 粘贴板：轮询监听/分类/去重/搜索/双击置顶粘贴/收藏/修改内容（更新时间戳）/30 天清理
11. 待办：`- [ ]` 解析/提醒时间/完成归档/右上角提醒卡片（关闭/选择下次提醒）
12. 三态圆点切换 + Cmd+1/2/3

### P2
13. 置顶浮窗（创建/拖拽/透明度/双击展开编辑）
14. 历史窗（列表/搜索/标签筛选/导出）
15. 使用统计（活动事件幂等写入 + 每日热力图 + 每月趋势，ECharts 渲染）
16. 导出 md/txt/pdf/png/jpg/html
17. 单窗口设置视图（快捷键/失焦延迟/保留天数/开机启动/数据路径）

### P3
18. 主题系统（亮暗 + 毛玻璃材质切换）
19. WebDAV 同步 Provider
20. 插件扩展点（command 注册表）
