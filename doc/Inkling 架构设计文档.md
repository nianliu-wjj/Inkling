# Inkling 架构设计文档

> **版本**：v2.0（对齐需求规格 v1.1）
> **日期**：2026-08-25
> **技术栈**：Tauri 2 + Rust + Vue 3 + TypeScript + UnoCSS + Vite + SQLite(rusqlite) + GSAP
> **关联文档**：[需求规格说明书](./requirement-spec.md)

---

## 1. 架构总览

### 1.1 架构形态

单进程多窗口 Tauri 应用。**所有窗口随应用启动预创建并常驻隐藏**，呼出只做 `show + focus`，这是「< 1 秒信条」的架构根基——新建 WebView 需 200ms+，而 show 一个已挂载的窗口 < 50ms。

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
│  ┌─────────┐                             │ pinned-N    │     │
│  │  main    │ 历史归档窗(900×600)          │ 置顶浮窗     │ ×N  │
│  │ settings │ 设置窗(680×480)             │ 220×~120    │     │
│  └─────────┘                             └─────────────┘     │
│                                          ┌─────────────┐     │
│                                          │ reminder    │     │
│                                          │ 右上角提醒卡  │ ×N  │
│                                          └─────────────┘     │
└─────────────────────────────────────────────────────────────┘

┌─ Tauri Rust 核心 ─────────────────────────────────────────┐
│  app/      窗口生命周期：tray / shortcut / hotzone / panel  │
│            / pinned / reminder / window_manager / launch   │
│  ipc/      #[tauri::command] 命令层 + 事件常量              │
│  domain/   纯业务逻辑：capture(标签解析) / clipboard(分类    │
│            去重) / todo(状态机) / stats(聚合) ← 可单测      │
│  data/     SQLite: db(连接+迁移) / notes / clipboard /     │
│            todos / settings / stats / file_store(大文件)   │
│  services/ export(md/txt/pdf/png/jpg/html) / reminder     │
└────────────────────────────────────────────────────────────┘
```

### 1.2 数据流

```
Vue 视图 ──invoke──▶ ipc/commands ──▶ domain(纯逻辑) ──▶ data(SQL/文件) ──▶ SQLite / notes/
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
- **归档策略**：点击「归档」按钮时，若内容 ≤ 1MB 则继续存 SQLite；若 > 1MB 则自动写入项目主目录 `/notes` 下的 `.md` 文件，SQLite 仅保留路径引用与元数据。

### ADR-006 剪贴板监听：轮询 + 哈希去重 + 回声抑制 + 30 天清理
- **决策**：`arboard` 每 500ms 轮询系统剪贴板；内容 SHA-256 哈希去重；应用自身写回剪贴板时记录 echo 哈希，下一次轮询命中即跳过（防自记录）。
- **分类**：纯 Rust 启发式（URL 正则 / 代码特征 / 图片 / 富文本 / 纯文本）。
- **保留策略**：tokio 定时任务每 24h 清理超过 30 天的记录。

### ADR-007 前端：单 SPA + window label 视图路由 + ProseMirror 内核
- **决策**：一个 Vite 构建产物；入口按 `getCurrentWindow().label` 分发到 `PanelView / HotzoneView / PinnedView / MainView / SettingsView / ReminderView`。
- **编辑器内核**：采用 **ProseMirror** 实现 Typora 级即时渲染（所见即所得，光标移入展示语法标记，移出渲染最终效果）。
- **组件策略**：panel 窗为极致首屏只引 UnoCSS 自研组件 + GSAP 动画；main/settings 窗引 Naive UI。Vite 按视图分包，Naive UI 不进 panel chunk。

### ADR-008 托盘图标主题自适应
- **macOS**：template image（系统自动控制黑白），`trayIcon.set_icon_as_template(true)`。
- **Windows**：监听 `darkmode` 变化事件切换预置的亮/暗两套 ICO。

### ADR-009 待办提醒：右上角自定义卡片（非系统通知）
- **决策**：到期时创建 `reminder` 无边框窗口（320×200，右上角定位，alwaysOnTop），展示提醒内容 + 操作按钮（关闭/选择下次提醒时间）。
- **理由**：系统通知无法提供「选择下次提醒时间」的交互，且部分平台限制频繁通知，违背「安静待命」但需保证提醒可达。

### ADR-010 使用统计：异步聚合表 + ECharts 渲染
- **决策**：新增 `stats` 聚合表，每次操作（创建笔记/使用粘贴板/完成待办）异步写入计数（避免阻塞主线程）。
- **展示**：前端使用 **ECharts** 渲染日历热力图与每月趋势折线图。

---

## 3. 窗口系统设计

### 3.1 窗口属性矩阵

| label | 尺寸 | transparent | alwaysOnTop | decorations | skipTaskbar | focusable | 生命周期 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `hotzone` | 240×80 | ✓ | ✓ | ✗ | ✓ | ✗ | 常驻显示 |
| `panel` | 480×自适应 | ✓ | ✓ | ✗ | ✓ | NSPanel | 常驻隐藏⇄显示 |
| `pinned-{noteId}` | 220×~120 | ✓ | ✓ | ✗ | ✓ | ✓ | 按需创建/销毁 |
| `main` | 900×600 | ✗ | ✗ | ✓(隐藏标题栏) | ✗ | ✓ | 常驻隐藏⇄显示 |
| `settings` | 680×480 | ✗ | ✗ | ✓ | ✗ | ✓ | 常驻隐藏⇄显示 |
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

- 取**鼠标所在显示器**（多屏场景）`current_monitor` → `size/position`。
- `x = monitor.x + (monitor.width - 480) / 2`；`y = monitor.y`（贴顶，无刘海妥协）。
- hotzone 同理，宽度 240。
- reminder 窗：固定屏幕右上角（`x = monitor.width - 320 - 24, y = 24`）。

---

## 4. 数据层设计（SQLite Schema + 文件存储）

```sql
-- 笔记（念头）
CREATE TABLE IF NOT EXISTS notes (
  id          TEXT PRIMARY KEY,            -- uuid v4
  content     TEXT NOT NULL,               -- Markdown 原文（≤1MB）
  plain_text  TEXT NOT NULL,               -- 渲染后纯文本（搜索用）
  file_path   TEXT,                        -- >1MB 时落盘路径（/notes/*.md）
  is_draft    INTEGER NOT NULL DEFAULT 0,  -- 1=输入中自动保存的草稿
  pinned      INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL,               -- ISO8601
  updated_at  TEXT NOT NULL
);

-- 标签（#tag 解析产物，多对多）
CREATE TABLE IF NOT EXISTS tags (
  id    INTEGER PRIMARY KEY AUTOINCREMENT,
  name  TEXT NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS note_tags (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (note_id, tag_id)
);

-- 剪贴板历史
CREATE TABLE IF NOT EXISTS clipboard_entries (
  id           TEXT PRIMARY KEY,
  content_type TEXT NOT NULL,              -- text/link/code/image/richtext
  content      TEXT NOT NULL,              -- 文本内容或图片 dataURL
  preview      TEXT NOT NULL,              -- 前 50 字摘要
  content_hash TEXT NOT NULL UNIQUE,       -- SHA-256 去重
  pinned       INTEGER NOT NULL DEFAULT 0,
  copied_at    TEXT NOT NULL,              -- 首次复制时间
  modified_at  TEXT NOT NULL,              -- 最后修改时间（内容变化时更新）
  created_at   TEXT NOT NULL
);

-- 待办
CREATE TABLE IF NOT EXISTS todos (
  id          TEXT PRIMARY KEY,
  content     TEXT NOT NULL,
  status      TEXT NOT NULL DEFAULT 'open',  -- open/done/archived
  remind_at   TEXT,                          -- ISO8601 可空
  done_at     TEXT,
  created_at  TEXT NOT NULL
);

-- 使用统计（每日聚合）
CREATE TABLE IF NOT EXISTS stats (
  date        TEXT NOT NULL,               -- YYYY-MM-DD
  note_count  INTEGER NOT NULL DEFAULT 0,
  clip_count  INTEGER NOT NULL DEFAULT 0,
  todo_count  INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (date)
);

-- 设置 KV（主题/快捷键/失焦延迟/保留天数/开机启动…）
CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

**文件存储约定**：
- 超大笔记（>1MB）落盘路径：`{项目主目录}/notes/{noteId}.md`
- SQLite 中 `file_path` 字段存储相对路径，读取时优先读文件，否则读 `content`。

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
  → >1MB: 写入 /notes/{id}.md + SQLite 存路径引用
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
tokio 定时任务（每分钟检查 remind_at <= now）
→ 创建 reminder 窗（右上角 320×200，alwaysOnTop）
→ 用户操作：
  - 关闭 → 关闭窗口 + 清除 remind_at
  - 选择下次提醒时间 → 更新 remind_at → 关闭窗口
```

### 5.6 使用统计写入

```
任意操作完成（创建笔记/使用粘贴板/完成待办）
→ 异步任务 stats.increment(date, type)（不阻塞主线程）
→ 前端统计页查询聚合数据 → ECharts 渲染热力图/趋势图
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
- 超大笔记落盘 `/notes` 目录支持用户自定义路径（可选）。

---

## 9. 测试策略

| 层 | 方式 | 覆盖 |
| --- | --- | --- |
| domain（标签解析/剪贴板分类/待办状态机/统计聚合） | `cargo test` 纯单测 | 全部分支 |
| data（CRUD/迁移/文件落盘） | `cargo test` + 内存 SQLite + 临时目录 | 全表 |
| 前端 stores/composables | vitest | 核心状态流 |
| 端到端手工回归 | 清单制 | 呼出/三态/pin/托盘/失焦/多屏/提醒/统计 |

---

## 10. 里程碑任务拆解

### P0（MVP）
1. 工程脚手架（Tauri2+Vue3+TS+Vite+UnoCSS+GSAP+vitest）
2. SQLite 初始化 + 迁移 + notes CRUD + 混合存储（≤1MB SQLite / >1MB 落盘）
3. panel 窗：预创建/定位/show/hide/GSAP 滑入动画/NSPanel
4. hotzone 感应窗 + hover 防抖联动
5. 全局快捷键注册/改键
6. 托盘（左右键菜单/图标主题/左键弹历史归档页）
7. 笔记模式：ProseMirror 即时渲染 + 500ms 自动保存 + 点击归档按钮 + #tag 解析
8. 失焦收起（立即/3s/固定）
9. 开机自启动（静默启动，macOS LaunchAgent / Windows Run 键）

### P1
10. 粘贴板：轮询监听/分类/去重/搜索/双击置顶粘贴/收藏/修改内容（更新时间戳）/30 天清理
11. 待办：`- [ ]` 解析/提醒时间/完成归档/右上角提醒卡片（关闭/选择下次提醒）
12. 三态圆点切换 + Cmd+1/2/3

### P2
13. 置顶浮窗（创建/拖拽/透明度/双击展开编辑）
14. 历史窗（列表/搜索/标签筛选/导出）
15. 使用统计（每日热力图 + 每月趋势，ECharts 渲染）
16. 导出 md/txt/pdf/png/jpg/html
17. 设置窗（快捷键/失焦延迟/保留天数/开机启动/数据路径）

### P3
18. 主题系统（亮暗 + 毛玻璃材质切换）
19. WebDAV 同步 Provider
20. 插件扩展点（command 注册表）
