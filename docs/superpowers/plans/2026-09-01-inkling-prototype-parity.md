# Inkling 原型对齐与 P0–P2 功能补全 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `doc/index.html` + `doc/styles.css` + `doc/app.js` 这套已完成的交互原型，落地为真实的 Tauri 2 + Vue 3 + TypeScript 桌面应用，交付需求规格 v1.2 的 P0~P2 全部能力。

**Architecture:** 后端（Rust / SQLite / 40 个 IPC 命令）已基本就绪，本计划以**前端重建**为主。采用「多入口 + 共享层」结构：5 个 vite 入口各挂载独立 Vue app（main / panel / pinned / reminder / hotzone），共用 `components/` `composables/` `editor/` `service/`。样式以原型 `doc/styles.css` 为**唯一样式源**，按职责切分移植进 `src/styles/`，不重写设计语言。笔记编辑器用 ProseMirror 做所见即所得，代码块内嵌 CodeMirror 6。

**Tech Stack:** Tauri 2.11 · Vue 3.5 (`<script setup>`) · TypeScript 5.9 · Vite 7 · Pinia 4 · ProseMirror · CodeMirror 6 · ECharts 6 · GSAP 3 · markdown-it · window-vibrancy 0.6

---

## Global Constraints

以下约束适用于**每一个任务**，不再逐条重复。

**样式**
- `doc/styles.css` 是唯一样式源。移植时**保留原选择器名、令牌名、数值**，只做文件切分与 Vue scoped 适配。不得自创设计语言，不得引入 Tailwind / naive-ui 主题覆盖原型视觉。
- 主题令牌共 30 套：默认 `dark`（`:root`）+ 29 套 `:root[data-theme="..."]`。完整清单：`light cupcake bumblebee emerald business neon retro romance halloween fantasy oled luxury dracula autumn businessgray night coffee winter dim sunset abyss aqua aurora latte lemon pastel print psychedelic wireframe`。
- 主题切换 = 在 `document.documentElement` 上设置 `data-theme`；`dark` 主题不设该属性。
- **禁止重复定义令牌**：现有 `src/styles/index.css` 与 `src/styles/themes.css` 各存了一份 29 套主题，必须去重，令牌只在 `tokens.css` + `themes.css` 各出现一次。

**编码规范（用户全局规则）**
- 关键代码必须有详细中文注释；关键节点（方法入口/出口、重要分支、IPC 调用前后、异常处理）必须有日志。前端日志统一走 `src/service/logger.ts`，禁止裸 `console.log`。
- Vue：一律 `<script setup lang="ts">`；props/emits 显式类型化；禁止 `any`。
- Rust：`cargo check` 零 error；新增 `pub` 项必须有 `///` 文档注释。
- CSS：仅使用令牌变量表达颜色，禁止硬编码十六进制色值（原型移植内容除外，原样保留）。

**验证方式（用户已选定）**
- 每个任务结束运行：`pnpm typecheck` 与（若改动 Rust）`cargo check --manifest-path src-tauri/Cargo.toml`，必须零 error。
- 前端**无单元测试框架**，因此前端任务以「静态检查 + 实机运行截图比对原型」验收。Rust 领域逻辑（`domain/todo.rs`、`domain/clipboard.rs`）已有 `#[cfg(test)]`，凡改动该层必须先写失败测试再实现（TDD），用 `cargo test --manifest-path src-tauri/Cargo.toml` 验证。
- 实机验证命令：`pnpm tauri:dev`，按任务的「实机验收」小节逐条比对。

**范围边界**
- 交付 P0~P2。**不做** P3 的 WebDAV 云同步与插件化能力。
- 原型中的演示脚手架 —— `#desktop` `#menubar` `#fakeApp` `#demoBar` `#onboarding` —— **不移植**，它们只为浏览器演示存在。托盘图标与右键主菜单走真实 Tauri 托盘（`src-tauri/src/app/tray.rs` 已实现）。

**时间约定**
- 后端一律 RFC3339 UTC 字符串。前端展示与「归属日期」计算一律转用户本地时区，禁止用 `slice(0,10)` 截断 UTC 字符串当本地日期。统一走 `src/utils/format.ts` 的日期函数。

---

## File Structure

```
src/
├─ styles/
│   ├─ tokens.css          默认 dark 令牌 + --radius + .glass + .btn/.hidden/kbd（移植 styles.css:1-72）
│   ├─ base.css            滚动条、全局 select 主题跟随、动画 keyframes（移植 :483-490, :686-689, 各 @keyframes）
│   ├─ components.css      卡片/标签/树/热力图/弹窗等全部组件样式（移植 :130-937，剔除演示脚手架段）
│   └─ themes.css          29 套 [data-theme] 覆盖（移植 :938-末尾）
├─ service/
│   ├─ tauri.ts            IPC 封装（已存在，需补全）
│   ├─ events.ts           跨窗口事件订阅，常量对齐 src-tauri/src/events.rs
│   └─ logger.ts           分级日志
├─ composables/
│   ├─ useHoverActions.ts  卡片悬浮显隐 + 父子层级隔离
│   ├─ useConfirmDelete.ts 上方悬浮二次确认框状态机
│   ├─ useAnchoredMenu.ts  锚定弹出菜单（定位/翻转/键盘/外部点击）
│   ├─ useShakeConfirm.ts  标签 ✕ 抖动二次确认（0.7s/次，3s 超时）
│   ├─ useTheme.ts         主题读写 + data-theme 应用
│   ├─ useSettings.ts      偏好设置 store 绑定
│   ├─ useNotes.ts / useClips.ts / useTodos.ts   数据 store + 事件刷新
│   └─ useToast.ts
├─ components/
│   ├─ base/     BaseBtn IconBtn SearchInput ModalShell ConfirmPopover ToastHost
│   ├─ tag/      TagChip TagList TagManagerModal
│   ├─ card/     NoteCard ClipCard TodoCard TodoTree DayPinCard
│   ├─ todo/     PriorityBadge PriorityMenu RepeatMenu DueBadge RemindBadge
│   │            TodoEditorModal DueEditModal
│   ├─ clip/     ClipEditorModal ClipTypeBadge
│   └─ stats/    HeatmapCalendar MiniHeatmap TrendChart HeatTip
├─ editor/
│   ├─ schema.ts           ProseMirror schema（markdown 子集）
│   ├─ inputrules.ts       ** _ ` # - > 等即时渲染输入规则
│   ├─ markdown.ts         ProseMirror ↔ Markdown 序列化
│   ├─ codeblock.ts        CodeMirror 6 NodeView
│   └─ NoteEditor.vue
├─ windows/
│   ├─ Main/     MainApp.vue Sidebar.vue NotesView.vue ClipsView.vue TodosView.vue
│   │            StatsView.vue SettingsView.vue DayView.vue
│   ├─ Panel/    PanelApp.vue NotePage.vue ClipPage.vue TodoPage.vue
│   ├─ Pinned/   PinnedApp.vue
│   └─ Reminder/ ReminderApp.vue
└─ (入口) main.ts panel.ts pinned.ts reminder.ts hotzone.ts
```

**删除**：`src/App.vue`（973 行，旧设计语言）、`src/styles/index.css`、`src/styles/windows.css`、`src/windows/Panel.vue`、`src/windows/Pinned.vue`。

---

## Phase 0 · 基础设施

### Task 1: 移植原型样式，消除令牌重复

**Files:**
- Create: `src/styles/tokens.css`, `src/styles/base.css`, `src/styles/components.css`
- Rewrite: `src/styles/themes.css`
- Delete: `src/styles/index.css`, `src/styles/windows.css`
- Source: `doc/styles.css`（唯一样式源）

**Interfaces:**
- Produces: 全局 CSS 类名契约，后续所有组件直接套用原型类名 —— `.glass .btn .btn.primary .btn.tiny .btn.ghost .icon-btn .search-input .pin-card .todo-item .tag-chip .confirm-pop .heat-cell` 等。
- Produces: 令牌变量 —— `--radius --accent --accent-rgb --wsa --text --text-dim --text-strong --glass-bg --glass-border --glass-shadow --c-note --c-clip --c-todo --trend-note --trend-clip --trend-todo --hm-base --option-bg --confirm-bg --tip-bg --menu-bg --body-bg --bg-deep --scheme --select-arrow`。

- [ ] **Step 1: 切出 tokens.css**

把 `doc/styles.css` 第 1–72 行原样复制到 `src/styles/tokens.css`。三处改动：
1. 删除 `* { ... user-select: none; }` 中的 `user-select: none`，改为在 `base.css` 里对可编辑区域放开（编辑器与输入框必须可选中）。
2. `body { height: 100vh; overflow: hidden; }` 保留 —— Tauri 窗口需要。
3. 文件顶部加注释块说明来源与「唯一样式源」约定。

- [ ] **Step 2: 切出 themes.css**

把 `doc/styles.css` 第 938 行至文件末尾复制到 `src/styles/themes.css`，**整体替换**现有内容。其中第 1102–1131 行的「主题下拉选择器（设置页）」属于组件样式，移到 `components.css`。

- [ ] **Step 3: 切出 components.css**

复制 `doc/styles.css` 第 130–937 行到 `src/styles/components.css`，**剔除**以下演示脚手架段落：`#desktop` `#menubar` `.menubar-*` `.tray-icon` `#fakeApp` `.fake-*` `.dot` `#onboarding` `.onboard-*` `#demoBar` `.demo-*`（对应 :73–129）。保留 `.context-menu .menu-row .menu-sep .menu-title` —— 优先级/重复菜单复用它们。

- [ ] **Step 4: 写 base.css**

```css
/* 全局基础：滚动条、下拉框主题跟随、可选中区域、通用动画 */
/* 输入区域恢复可选中（tokens.css 全局关闭了 user-select） */
input, textarea, [contenteditable="true"], .ProseMirror, .cm-editor {
  user-select: text;
  -webkit-user-select: text;
}
/* 拖拽区域：无边框窗口的标题栏 */
[data-tauri-drag-region] { app-region: drag; }
[data-tauri-drag-region] button { app-region: no-drag; }
```
再把 `doc/styles.css` 的滚动条段（:686-689）与下拉框段（:483-490）并入本文件。

- [ ] **Step 5: 建立统一入口并删除旧样式**

创建 `src/styles/index.ts`：
```ts
/** 全局样式入口：顺序不可调换 —— 令牌 → 基础 → 组件 → 主题覆盖。 */
import './tokens.css'
import './base.css'
import './components.css'
import './themes.css'
```
删除 `src/styles/index.css` 与 `src/styles/windows.css`。

- [ ] **Step 6: 验证无重复令牌**

Run: `grep -c 'data-theme=' src/styles/themes.css`
Expected: `29`

Run: `grep -rn 'data-theme' src/styles/tokens.css src/styles/components.css | wc -l`
Expected: `0`

- [ ] **Step 7: Commit**

```bash
git add src/styles docs/superpowers/plans
git commit -m "refactor(styles): 以原型 styles.css 为唯一样式源重建样式层，消除令牌重复"
```

---

### Task 2: 服务层与日志

**Files:**
- Create: `src/service/logger.ts`, `src/service/events.ts`
- Modify: `src/service/tauri.ts`, `src/typings/domain.ts`

**Interfaces:**
- Produces: `logger.debug/info/warn/error(scope: string, msg: string, ...args: unknown[])`
- Produces: `onAppEvent(name: AppEvent, handler: (payload: unknown) => void): Promise<UnlistenFn>`，`AppEvent` 联合类型对齐 `src-tauri/src/events.rs` 的 11 个常量。
- Produces: `api.*` 全量 IPC 封装（含 `api.export.items`、`api.system.dataDir`、`api.stats.day`）。

- [ ] **Step 1: 写 logger.ts**

```ts
/** 分级日志：统一前缀与时间戳，便于在 WebView 控制台按窗口过滤。 */
type Level = 'debug' | 'info' | 'warn' | 'error'
const WINDOW = new URLSearchParams(location.search).get('w') ?? location.pathname
function emit(level: Level, scope: string, msg: string, args: unknown[]): void {
  const line = `[${new Date().toISOString()}][${WINDOW}][${scope}] ${msg}`
  // eslint-disable-next-line no-console
  console[level === 'debug' ? 'log' : level](line, ...args)
}
export const logger = {
  debug: (s: string, m: string, ...a: unknown[]) => emit('debug', s, m, a),
  info: (s: string, m: string, ...a: unknown[]) => emit('info', s, m, a),
  warn: (s: string, m: string, ...a: unknown[]) => emit('warn', s, m, a),
  error: (s: string, m: string, ...a: unknown[]) => emit('error', s, m, a),
}
```

- [ ] **Step 2: 写 events.ts**

```ts
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
/** 事件名常量，必须与 src-tauri/src/events.rs 逐字对齐。 */
export const AppEvents = {
  navigate: 'inkling://navigate',
  panelShown: 'inkling://panel-shown',
  panelHidden: 'inkling://panel-hidden',
  notesChanged: 'inkling://notes-changed',
  clipboardChanged: 'inkling://clipboard-changed',
  todosChanged: 'inkling://todos-changed',
  settingsChanged: 'inkling://settings-changed',
  statsChanged: 'inkling://stats-changed',
  pinUpdated: 'inkling://pin-updated',
  reminderFired: 'inkling://reminder-fired',
} as const
export type AppEvent = (typeof AppEvents)[keyof typeof AppEvents]
export function onAppEvent<T = unknown>(name: AppEvent, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (e) => handler(e.payload))
}
```

- [ ] **Step 3: 校对 tauri.ts 与后端命令签名**

逐条比对 `src-tauri/src/ipc.rs` 的 40 个 `#[tauri::command]`，补齐 `tauri.ts` 中缺失的封装（至少 `stats.day`、`export.items`、`system.dataDir`）。参数名必须是 camelCase（Tauri 自动转换 snake_case 形参）。

- [ ] **Step 4: 校对 domain.ts 与 Rust 模型**

`src/typings/domain.ts` 的接口字段必须与 `src-tauri/src/domain/models.rs` 逐字段对齐：`Note`(9 字段) `ClipboardEntry`(9) `Todo`(15) `Settings`(6) `DayActivity`(6) `MonthTrend`(5) `StatsSummary`(5) `DayDetailItem`(5)。字段名保持 snake_case（serde 未加 rename_all）。

- [ ] **Step 5: 验证**

Run: `pnpm typecheck`
Expected: 零 error

- [ ] **Step 6: Commit**

```bash
git add src/service src/typings
git commit -m "feat(service): 补齐 IPC 封装、跨窗口事件与分级日志"
```

---

### Task 3: 共享 composables

**Files:**
- Create: `src/composables/useHoverActions.ts` `useConfirmDelete.ts` `useAnchoredMenu.ts` `useShakeConfirm.ts` `useToast.ts` `useTheme.ts`

**Interfaces:**
- Produces: `useHoverActions()` → `{ hoveredId: Ref<string|null>, bind(id: string): { onMouseenter, onMouseleave } }`。**层级隔离**：子任务 `mouseenter` 必须 `stopPropagation`，使父级 hover 态失效。
- Produces: `useConfirmDelete()` → `{ pendingId, ask(id), cancel(), confirm(): string|null }`
- Produces: `useAnchoredMenu()` → `{ open(anchor: HTMLElement), close(), style: ComputedRef<CSSProperties>, visible: Ref<boolean> }`；默认锚点下方左对齐，下方空间不足翻转到上方，超出视口向内收缩；Esc / 外部点击关闭；↑↓/Home/End 移动，Enter 提交，关闭后焦点回锚点。
- Produces: `useShakeConfirm()` → `{ shakingId, arm(id), disarm(), isArmed(id) }`；3 秒无操作自动 `disarm`。
- Produces: `useToast()` → `{ toast(msg: string, ms?: number) }`
- Produces: `useTheme()` → `{ theme: Ref<string>, applyTheme(name: string) }`

- [ ] **Step 1: useHoverActions —— 父子层级隔离是关键**

```ts
import { ref, type Ref } from 'vue'
/**
 * 卡片悬浮显隐。同一时刻只有一个卡片 id 处于 hover 态。
 * 待办树中子任务必须阻止事件冒泡，否则父级会同时点亮（需求 2.2「悬浮层级隔离」）。
 */
export function useHoverActions(): {
  hoveredId: Ref<string | null>
  bind: (id: string) => { onMouseenter: (e: MouseEvent) => void; onMouseleave: (e: MouseEvent) => void }
} {
  const hoveredId = ref<string | null>(null)
  const bind = (id: string) => ({
    onMouseenter: (e: MouseEvent) => { e.stopPropagation(); hoveredId.value = id },
    onMouseleave: (e: MouseEvent) => { e.stopPropagation(); if (hoveredId.value === id) hoveredId.value = null },
  })
  return { hoveredId, bind }
}
```

- [ ] **Step 2: useConfirmDelete**

```ts
import { ref, type Ref } from 'vue'
import { logger } from '@/service/logger'
/** 删除二次确认：确认框悬浮于卡片上方，同一时刻只允许一个待确认项。 */
export function useConfirmDelete(): {
  pendingId: Ref<string | null>
  ask: (id: string) => void
  cancel: () => void
  confirm: () => string | null
} {
  const pendingId = ref<string | null>(null)
  return {
    pendingId,
    ask: (id) => { logger.debug('confirm-delete', `进入确认态 id=${id}`); pendingId.value = id },
    cancel: () => { pendingId.value = null },
    confirm: () => { const id = pendingId.value; pendingId.value = null; return id },
  }
}
```

- [ ] **Step 3: useAnchoredMenu**

实现锚定定位：`getBoundingClientRect()` 取锚点位置，菜单固定定位；若 `rect.bottom + menuH > innerHeight` 则翻转到 `rect.top - menuH`；左边缘对齐 `rect.left`，若 `rect.left + menuW > innerWidth` 则收缩为 `innerWidth - menuW - 8`。挂 `document` 的 `click`（capture）与 `keydown` 监听，`onUnmounted` 清理。

- [ ] **Step 4: useShakeConfirm / useToast / useTheme**

`useTheme.applyTheme` 逻辑：`name === 'dark' ? root.removeAttribute('data-theme') : root.setAttribute('data-theme', name)`，并调 `api.settings.save` 持久化。

- [ ] **Step 5: 验证 + Commit**

Run: `pnpm typecheck` → 零 error
```bash
git add src/composables && git commit -m "feat(composables): 抽出悬浮显隐/二次确认/锚定菜单/抖动确认共享逻辑"
```

---

## Phase 1 · 共享组件

### Task 4: 基础组件与 Toast

**Files:** Create `src/components/base/{BaseBtn,IconBtn,SearchInput,ModalShell,ConfirmPopover,ToastHost}.vue`

**Interfaces:**
- Consumes: Task 1 类名 `.btn .icon-btn .search-input .glass .confirm-pop`；Task 3 的 `useToast`
- Produces: `<ConfirmPopover :text="string" @confirm @cancel />` —— 绝对定位于卡片上方，`position:absolute; bottom:100%`，不推挤布局
- Produces: `<ModalShell :title="string" @close>` —— `.glass` 弹窗外壳 + 遮罩 + Esc 关闭 + `<slot>`/`<slot name="footer">`

- [ ] **Step 1–6:** 逐个实现，样式全部套用 Task 1 已移植的原型类名，组件内**不写新样式**（仅写布局必需的 scoped 微调）。`ConfirmPopover` 必须复用 `.confirm-pop` 与 `confirmIn` 动画。
- [ ] **Step 7:** `pnpm typecheck` → 零 error；Commit `feat(components): 基础组件与二次确认浮层`

---

### Task 5: 标签体系

**Files:** Create `src/components/tag/{TagChip,TagList,TagManagerModal}.vue`

**Interfaces:**
- Consumes: `useShakeConfirm`, `ModalShell`
- Produces: `<TagList :tags="string[]" :max="number" :limit="number" @click-empty />` —— 超出 `max` 聚合为「+N」，点击展开；无标签时渲染置灰「无标签」占位（**无 ✕**，点击触发 `click-empty`）
- Produces: `<TagManagerModal :tags :maxCount :maxLen @save @close />` —— 流式布局；回车新增、点击文字就地编辑、✕ 抖动二次确认删除

- [ ] **Step 1:** `TagChip` —— ✕ 仅 hover 显示；进入抖动态时套 `.shake` 类（0.7s/次，复用原型 `tagShake` keyframes）；`✕:hover` 时暂停抖动（原型 :611-616 已有规则）
- [ ] **Step 2:** `TagList` —— 笔记场景 `max=3, limit=5`，待办场景 `max=3, limit=10`
- [ ] **Step 3:** `TagManagerModal` —— 笔记标签上限 5 字；去重校验；管理页 ✕ 常显
- [ ] **Step 4:** `pnpm typecheck`；Commit `feat(tag): 标签 chip、列表与管理弹窗`

---

### Task 6: 待办卡片与树

**Files:** Create `src/components/todo/{PriorityBadge,PriorityMenu,RepeatMenu,DueBadge,RemindBadge,RemarkDisplay}.vue`, `src/components/card/{TodoCard,TodoTree}.vue`

**Interfaces:**
- Consumes: `useAnchoredMenu` `useHoverActions` `useConfirmDelete` `TagList` `ConfirmPopover`
- Produces: `<TodoTree :todos="Todo[]" :hovered @edit @delete @toggle @add-sub @priority @due @remind />`
- Produces: `sortTodos(todos: Todo[]): Todo[]` in `src/utils/todo.ts` —— 完成时间升序 → 优先级高在前 → 创建时间升序，已完成沉底
- Produces: `partitionOverdue(todos: Todo[], now: Date): { overdue: Todo[]; normal: Todo[] }` —— 父待办有逾期子任务时整棵树入 overdue

- [ ] **Step 1:** `PriorityMenu` —— 三项常驻，当前项带勾选 + `aria-selected`；颜色圆点 + 文字双通道；点击区 ≥32×32px；键盘全支持。**不再用卡片本身充当菜单项**（v1.2 变更 #6）
- [ ] **Step 2:** `DueBadge` —— 「📅 今天 HH:mm」/「📅 M/D HH:mm」，逾期红色，常显，点击 emit `due`
- [ ] **Step 3:** `RemindBadge` —— 已设 → 「⏰ 日期 时间」（同日仅时间）；未设 → 淡色 ⏰ 占位 + tooltip 说明默认提醒计划（前 30 分 / 前 5 分 / 到点）
- [ ] **Step 4:** `RemarkDisplay` —— 三模式：`icon` / `line` / `mixed`（≤100 字文本行，>100 字图标），读 `settings.remark_style`
- [ ] **Step 5:** `TodoCard` —— 右上角 ✕（hover 显示）、底部左侧标签+完成时间徽章（常显）、底部右侧 ⏰/＋子任务/✏️（hover 显示）；已完成项全部交互拦截并 toast
- [ ] **Step 6:** `TodoTree` —— 树连接线（复用原型 :800-846 的 `│ ├─ └─` 绘制）、折叠箭头 ▸ 旋转、层级渐变；**不显示 n/m 进度计数**
- [ ] **Step 7:** `sortTodos` / `partitionOverdue` 纯函数
- [ ] **Step 8:** `pnpm typecheck`；Commit `feat(todo): 待办卡片、子任务树与优先级锚定菜单`

---

### Task 7: 笔记卡片与剪贴板卡片

**Files:** Create `src/components/card/{NoteCard,ClipCard}.vue`, `src/components/clip/{ClipTypeBadge,ClipEditorModal}.vue`, `src/utils/markdown.ts`

**Interfaces:**
- Produces: `renderMarkdown(src: string): string` —— markdown-it 实例，`html:false`（防注入），供归档卡片轻量渲染
- Produces: `<NoteCard :note @edit @delete @pin />` —— 元数据行「时间 + 标签 chips」同一行，标签紧跟时间之后
- Produces: `<ClipCard :entry :compact @paste @edit @pin @delete @open-link />` —— 内容最多两行省略（`-webkit-line-clamp:2`）；`compact` 用于面板

- [ ] **Step 1:** `renderMarkdown` —— 禁用 HTML，开启 linkify
- [ ] **Step 2:** `NoteCard` —— 正文按 Markdown 渲染，不显示源码标记
- [ ] **Step 3:** `ClipTypeBadge` —— text/link/code/image/richtext 五色徽章
- [ ] **Step 4:** `ClipCard` —— 左上时间、右上 ✕、右下操作组（粘贴/打开链接(仅 link)/编辑(仅文本类)/收藏）；双击 = 粘贴并置顶；置顶金色高亮优先排序
- [ ] **Step 5:** `ClipEditorModal` —— 回显、⌃/⌘+Enter 保存
- [ ] **Step 6:** `pnpm typecheck`；Commit `feat(card): 笔记与剪贴板卡片`

---

## Phase 2 · 笔记编辑器

### Task 8: ProseMirror 所见即所得编辑器

**Files:** Create `src/editor/{schema.ts,inputrules.ts,markdown.ts,NoteEditor.vue}`

**Interfaces:**
- Produces: `<NoteEditor v-model="string" @save />`，暴露 `getMarkdown(): string`
- Produces: schema 节点 —— `doc paragraph heading(1-3) blockquote code_block bullet_list ordered_list list_item hard_break text`；marks —— `strong em code link strikethrough`

- [ ] **Step 1:** `schema.ts` —— 基于 `prosemirror-markdown` 的 `schema`，加 `strikethrough` mark
- [ ] **Step 2:** `inputrules.ts` —— `**粗体**` `*斜体*` `` `代码` `` `# 标题` `- 列表` `> 引用` ` ``` 代码块` 的即时输入规则；配合 `prosemirror-history` `prosemirror-keymap` `prosemirror-dropcursor` `prosemirror-gapcursor`
- [ ] **Step 3:** `markdown.ts` —— `defaultMarkdownParser` / `defaultMarkdownSerializer` 包装，扩展 strikethrough
- [ ] **Step 4:** `NoteEditor.vue` —— 挂载 EditorView，套原型 `.editor` 类名与 placeholder；输入停止 500ms 触发 `note_save`（草稿暂存）
- [ ] **Step 5:** `pnpm typecheck`；Commit `feat(editor): ProseMirror 所见即所得笔记编辑器`

---

### Task 9: 代码块内嵌 CodeMirror

**Files:** Create `src/editor/codeblock.ts`; Modify `src/editor/NoteEditor.vue`

**Interfaces:**
- Consumes: Task 8 的 schema `code_block` 节点
- Produces: `CodeBlockView` —— ProseMirror NodeView，内部持有 CodeMirror 6 EditorView

- [ ] **Step 1:** 实现 NodeView 三件套 —— `update()` 同步外部变更、`setSelection()`、`stopEvent()` 返回 true 让 CM 独占事件
- [ ] **Step 2:** 光标越界处理 —— CM 首行 ↑ / 末行 ↓ 时把焦点交还 ProseMirror
- [ ] **Step 3:** CM 主题跟随 —— 用 CSS 变量而非 CM 内置主题，保证 30 套主题一致
- [ ] **Step 4:** `pnpm typecheck`；Commit `feat(editor): 代码块内嵌 CodeMirror 6`

---

## Phase 3 · 窗口实现

### Task 10: 呼出面板（三态合一）

**Files:** Create `src/windows/Panel/{PanelApp,NotePage,ClipPage,TodoPage}.vue`; Rewrite `src/panel.ts`; Delete `src/windows/Panel.vue`

- [ ] **Step 1:** `PanelApp` —— 三态圆点导航（🔴🟡🟢）、`⌃1/2/3` 切换、Esc 收起、GSAP 滑入 200ms / 滑出 150ms 弹性过渡
- [ ] **Step 2:** **弹窗失焦保护** —— 任一弹窗打开时禁止失焦收起；全部关闭后若鼠标不在面板内，按 `collapse_policy` 重新计时
- [ ] **Step 3:** `NotePage` —— NoteEditor + 右下角标签区（TagList，点击弹 TagManagerModal）+ 归档按钮 + 「已暂存」状态
- [ ] **Step 4:** `ClipPage` —— 搜索 + ClipCard 列表（compact），双击粘贴并置顶
- [ ] **Step 5:** `TodoPage` —— 搜索 + 优先级过滤 + 📅 新增；**仅当日，无日期切换**；逾期置顶分区
- [ ] **Step 6:** 高度自适应 —— 内容变化时调 `api.windows.panelResize`，钳制 120~600px
- [ ] **Step 7:** `pnpm typecheck`；实机验收：呼出面板毛玻璃、三态切换、Esc 收起；Commit

---

### Task 11: 主窗口骨架与侧边栏

**Files:** Create `src/windows/Main/{MainApp,Sidebar}.vue`; Rewrite `src/main.ts`; Delete `src/App.vue`

- [ ] **Step 1:** `MainApp` —— 无边框标题栏（`data-tauri-drag-region`）+ 左右结构；响应 `inkling://navigate` 事件切视图
- [ ] **Step 2:** `Sidebar` —— 三页签（笔记蓝/粘贴板金/待办绿）+ 计数徽章 + 选中指示条；底部左 ⚙️ 右 📊
- [ ] **Step 3:** 拖宽 —— 分隔条拖动实时调宽 110~280px；低于阈值自动折叠为 52px 图标窄栏；« / » 切换；宽度存 localStorage
- [ ] **Step 4:** `MiniHeatmap` —— 当月热力图，悬浮明细，点击进日期详情
- [ ] **Step 5:** `pnpm typecheck`；实机验收：侧边栏拖宽/折叠动画顺滑；Commit

---

### Task 12: 笔记页与粘贴板页

**Files:** Create `src/windows/Main/{NotesView,ClipsView}.vue`

- [ ] **Step 1:** `NotesView` —— 搜索（正文 + 标签）+ NoteCard 列表 + 置顶优先
- [ ] **Step 2:** `ClipsView` —— 搜索 + ClipCard（完整态，带类型徽章、打开链接按钮）+ 置顶金色高亮优先
- [ ] **Step 3:** `pnpm typecheck`；Commit

---

### Task 13: 待办归档页

**Files:** Create `src/windows/Main/TodosView.vue`, `src/components/todo/{TodoEditorModal,DueEditModal}.vue`

- [ ] **Step 1:** 日期切换条 —— ‹ / 日期 / › / 今天 + 搜索框 + ＋新增
- [ ] **Step 2:** `TodoEditorModal` —— 内容/标签/备注(200 字计数)/完成日期+时刻/提醒日期+时间/优先级；提醒模式下其余字段只读
- [ ] **Step 3:** 创建约束 —— 完成时间不早于当前（日期下限锁今天），默认当前 +1 小时；历史日期补录提示
- [ ] **Step 4:** 子任务约束 —— 完成时间不晚于父级；最多 5 个；不可再嵌套
- [ ] **Step 5:** 已完成父待办 ＋子任务 → 父级同事务恢复未完成、清 `completed_at`、`due_at` 顺延
- [ ] **Step 6:** 逾期置顶分区「⚠️ 逾期事项（n）」
- [ ] **Step 7:** 跨日期搜索 —— 含子任务与已完成项，结果显示所属日期徽章，命中高亮
- [ ] **Step 8:** `DueEditModal` —— 仅完成日期/时刻可编辑
- [ ] **Step 9:** `pnpm typecheck`；实机验收：逾期标红、排序稳定、优先级菜单翻转；Commit

---

### Task 14: 统计页与日期详情页

**Files:** Create `src/windows/Main/{StatsView,DayView}.vue`, `src/components/stats/{HeatmapCalendar,TrendChart,HeatTip}.vue`

- [ ] **Step 1:** `HeatmapCalendar` —— 列=周、行=星期的 GitHub 风格日历；顶部月份范围标签与周列对齐；左侧「一/四/日」标签
- [ ] **Step 2:** 悬浮 tooltip —— 日期 + 笔记数 + 复制项数 + 待办数（含已完成/逾期）；存在逾期的格子红色边框
- [ ] **Step 3:** `TrendChart` —— ECharts 折线图，近 6 个月，三条线用 `--trend-note/--trend-clip/--trend-todo`
- [ ] **Step 4:** 统计页支持滚动
- [ ] **Step 5:** `DayView` —— 该日全部 pin 混排按时间排序（待办取完成时间）+ 类别筛选 + 搜索 + 悬浮编辑/删除
- [ ] **Step 6:** `pnpm typecheck`；实机验收：热力图与折线渲染正确；Commit

---

### Task 15: 偏好设置页（含毛玻璃开关）

**Files:** Create `src/windows/Main/SettingsView.vue`; Modify `src-tauri/src/ipc.rs`, `src-tauri/src/app/windows.rs`, `src-tauri/tauri.conf.json`

**Interfaces:**
- Produces: 新 IPC 命令 `set_main_acrylic(app: AppHandle, enabled: bool) -> Result<(), String>`

- [ ] **Step 1:** `tauri.conf.json` main 窗口改 `"transparent": true`
- [ ] **Step 2:** 新增 `set_main_acrylic` —— `enabled` 时调 `window_vibrancy::apply_acrylic`，否则 `clear_acrylic`；注册进 `invoke_handler`
- [ ] **Step 3:** `Settings` 增字段 `main_acrylic: bool`（默认 true），同步 `data/settings.rs` 与 `typings/domain.ts`
- [ ] **Step 4:** 前端开关 —— 调 IPC + 在根元素挂 `data-acrylic="off"`，该属性下把 `--glass-bg` 覆盖为不透明实色（取 `--bg-deep`），关闭时 `.glass` 自动退化为实心卡片，无需改任何组件
- [ ] **Step 5:** 其余设置项 —— 失焦策略/保留天数/开机自启/快捷键录制/备注展示样式/主题下拉（30 套）
- [ ] **Step 6:** `cargo check` + `pnpm typecheck`；实机验收：开关即时生效不丢窗口状态；Commit

---

### Task 16: 置顶浮窗、提醒卡片与感应区

**Files:** Create `src/windows/Pinned/PinnedApp.vue`, `src/windows/Reminder/ReminderApp.vue`; Rewrite `src/windows/hotzone.ts`, `src/pinned.ts`, `src/reminder.ts`; Delete `src/windows/Pinned.vue`

- [ ] **Step 1:** `PinnedApp` —— 头部/正文/透明度滑杆；双击展开编辑（调 `pin_set_editing`）
- [ ] **Step 2:** `ReminderApp` —— 右上角卡片；关闭 = 稍后不再提醒；下拉选择下次提醒（10/30/60/180 分钟、明天 9:00）
- [ ] **Step 3:** `hotzone` —— 顶部中央 ±120px、高 80px 感应区，悬停 100ms 触发 `panel_show`
- [ ] **Step 4:** `pnpm typecheck`；实机验收：提醒到点弹出、置顶窗透明度可调；Commit

---

## Phase 4 · 收尾

### Task 17: 导出功能

**Files:** Modify `src/components/card/{NoteCard,ClipCard,TodoCard}.vue`, `src/windows/Main/*View.vue`

- [ ] **Step 1:** 卡片悬浮功能区加「⤓ 导出」按钮，调 `api.export.items`（单条）
- [ ] **Step 2:** 各归档页头部加「批量导出」入口，走 `plugin-dialog` 选保存目录
- [ ] **Step 3:** 导出格式 Markdown / JSON（后端 `services/export.rs` 已实现）
- [ ] **Step 4:** `pnpm typecheck`；Commit

---

### Task 18: 动画打磨与全量验收

- [ ] **Step 1:** GSAP 过渡统一 —— 面板滑入 200ms `power2.out` / 滑出 150ms；卡片进入 `fadeIn`；侧边栏宽度过渡；菜单 `confirmIn`
- [ ] **Step 2:** 清理 Rust 的 11 个 warning
- [ ] **Step 3:** `pnpm typecheck` + `cargo check` + `cargo test` 全绿
- [ ] **Step 4:** `pnpm tauri:dev` 实机逐屏截图，与 `doc/index.html`（浏览器打开）逐项比对：面板三态 / 归档四视图 / 统计 / 设置 / 日期详情 / 置顶窗 / 提醒卡
- [ ] **Step 5:** Commit `feat: 完成 P0~P2 原型对齐`

---

## 需求覆盖对照

| 需求条目 | 覆盖任务 |
| :-- | :-- |
| 2.1 悬浮展开 / 动效 / 层级 / 失焦（含弹窗保护） | Task 10, 16 |
| 2.2-1 笔记：即时渲染 / 归档 / 标签体系 / 卡片悬浮显隐 | Task 5, 7, 8, 9, 10, 12 |
| 2.2-2 粘贴板：监听 / 搜索 / 双击粘贴 / 两行省略 / 类型徽章 / 收藏 | Task 7, 10, 12 |
| 2.2-3 待办：完成时间模型 / 提醒 / 标签 / 备注 / 优先级阶梯 / 逾期置顶 / 子任务树 | Task 6, 13, 16 |
| 2.3 状态栏常驻 / 开机自启 | 后端已实现；Task 15 设置项 |
| 2.4 热力图 / 折线趋势 | Task 14 |
| 2.5 桌面置顶浮窗 | Task 16 |
| 2.6 持久化 / 导出 | 后端已实现；Task 17 |
| 2.7 偏好设置（含 30 套主题、毛玻璃开关） | Task 1, 15 |
| v1.2 #4 归档单窗口 / #5 侧边栏折叠拖宽 / #13 搜索 | Task 11, 12, 13 |
