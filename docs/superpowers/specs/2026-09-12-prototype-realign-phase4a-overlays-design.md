# 原型对齐 · 阶段四 A「弹窗与浮层」设计

**日期**：2026-09-12
**前置**：阶段一（样式分层 / 动效模块）、阶段二（D10–D14）、阶段三（D15–D21，`2026-09-11-prototype-realign-phase3-panel-design.md`）
**原型**：`docs/index.html` `#cardConfirm`（656–662）、`#todoEditorOverlay`（500–566）、`#repeatMenu`（588–593）、`#tagManagerOverlay`（480–498）、`#clipEditorOverlay`（568–586）、`#toast`（664）；`docs/app.js` `refreshCardConfirm`（141–186）、`openTodoEditor` / `positionTodoEditor`（1026–1128）、`showRepeatMenu`（932–958）、`showToast`（213–220）、标签 / 粘贴板编辑（751–775、2609–2690）；生成层 `src/styles/components.css` 已含 `#cardConfirm` / `.cc-caret`（2420–2457）、`#todoEditorOverlay` / `#todoEditorPanel` / `.te-caret`（3480–3520）、`.repeat-menu` / `.repeat-opt`（3678–3700）、`#toast`（3042）

## 1. 范围

阶段四拆为三个子阶段：**4A 弹窗与浮层**（本文）→ 4B 灵动岛 + 感应区 + 提醒卡片 + 置顶浮窗 + `settings_save` 死锁修复 → 4C 浮窗启动台完整化（笔记 / 待办 / 计算器 / 浏览器历史）。

4A 对齐：删除二次确认浮层（修复阶段三遗留的首张卡片裁切）、待办编辑浮层（锚定卡片右侧、聚焦单区段模式、标题与提示文案）、重复提醒菜单、标签管理与粘贴板编辑弹窗的动效与细节、toast 参数；删除 `extensions.css` §5 全部与 §6 中的 `.card-confirm*` 规则。

**不在 4A**：灵动岛 / 感应区 / 提醒 / 置顶 / 启动台（4B、4C）；优先级菜单形态（保留现状，见 D26）；面板内弹窗承载方式（保留整页化，见 D25）。

## 2. 决策记录（本阶段新增，均已确认）

| # | 问题 | 采用 | 理由 |
|---|---|---|---|
| D22 | 阶段四拆分 | 4A → 4B → 4C 三份 spec，各自 plan → 实现 → 验收 | 约 48 条差异、8 个新设置字段、1 张新表，单份体量过大 |
| D23 | 待办编辑浮层承载 | 面板与主窗口统一走现有铺满工作区的透明 `editor` 窗口，payload 携带锚点卡片的屏幕坐标，窗内按原型 `positionTodoEditor` 定位 `#todoEditorPanel`（右侧 → 左翻 → 居中兜底），带箭头、无遮罩 | 一条代码路径，完全复刻原型定位；主窗口内锚定会被窗口边界裁切 |
| D24 | 灵动岛悬停 | 保留现状（悬停撑高显示详情，点击开面板） | 用户选择；属 4B，此处仅记录 |
| D25 | 面板内标签管理 / 粘贴板编辑 | 保留阶段三已验收的整页化；标签管理保留「取消 / 保存」 | 观感已认可；误删可取消 |
| D26 | 优先级菜单 | 保留三项全显 + 勾选（需求 v1.2 变更 #6） | 既有产品决策，可达性更好 |
| D27 | 删除确认在面板内的兜底位置 | 面板 480px 内卡片左右都放不下时，浮层放在**卡片下方**（箭头朝上，水平对齐卡片左缘），而不是原型的「水平居中盖在卡片上」 | 原型兜底会遮住待删内容；面板内几乎每次都命中兜底 |
| D28 | 待办编辑与删除确认的锚点粒度 | 一律锚定到**卡片**（`[data-id]`），不细分到徽章 | 原型 due / remind / tags / remark 锚到徽章；卡片级锚点少传四个事件参数，视觉差异只在箭头纵向位置 |

## 3. 与原型的差异处理表

| # | 项 | 原型 | 现状 | 做法 |
|---|---|---|---|---|
| **删除确认** | | | | |
| 1 | 结构与层级 | body 级唯一 `#cardConfirm[role=alertdialog]` > `.cc-caret` + `.card-confirm-text` + `.btn.tiny.danger` 删除 + `.btn.tiny.ghost` 取消；fixed | 卡片内 `ConfirmPopover.card-confirm` absolute 于卡片上方，首张被列表容器裁切 | 共享件 `CardConfirm.vue`（Teleport body，原型结构）；删 `ConfirmPopover.vue`、`extensions.css` §5 `confirmIn` / `.card-confirm-text`、§6 `.card-confirm` 与 `.todo-item:has(.card-confirm) .todo-del` |
| 2 | 定位 | 右侧 +14 → 左侧 −14 并 `.flip` → 水平居中；垂直居中卡片；`--caret-y` 对齐卡片中心 | — | 纯函数 `anchorBeside()`（§4.1）；兜底按 D27 |
| 3 | 文案 | 「确认删除该笔记？ / 该条目？ / 该待办事项？ / 该子任务？」 | 「⚠️ 确认删除…」 | 去 ⚠️，四种文案 |
| 4 | 重定位与清除 | 每次重渲染后 `refreshCardConfirm`；卡片消失则清除；`scroll`(capture) / `resize` / Esc / 收面板 / 切视图关闭 | 随卡片 v-if 生命周期 | `CardConfirm` 内 `ResizeObserver` + 容器 `MutationObserver` 合帧重定位；`scroll` capture 与 `resize` 关闭；Esc 由各页 `dismissOverlays` 已覆盖 |
| 5 | 入场 | opacity 0，x ∓10 → 0.15s | `confirmIn` 4px 下移 | `motion` `enter()` 横向 10px，时长 `--dur-fast` |
| 6 | 删除按钮常显 | — | `.todo-item:has(.card-confirm) .todo-del { opacity:1 }` | 改为卡片 `.confirming` 类 + `extensions.css` §3 一条 `.todo-item.confirming .todo-del { opacity:1; pointer-events:auto }` |
| **待办编辑** | | | | |
| 7 | 承载 | 透明 overlay + fixed 340px `#todoEditorPanel` 锚定卡片 | 主窗口 `ModalShell` 居中遮罩；面板走 `editor` 窗居中 520px | D23：主窗口 `TodosView` / `DayView` 也改调 `editorOpen`；`EditorApp` 渲染新 `TodoEditorPanel` |
| 8 | 结构 | `#todoEditorOverlay` > `#todoEditorPanel.glass[data-mode]` > `.te-caret` + `.clip-editor-header`(`#todoEditorTitle`) + `.te-body`(六个 `.te-sect[data-sect]`) + `.clip-editor-footer`(`#todoEditorHint` + 取消 / 保存) | `ModalShell` + `.modal-body` | `TodoEditorModal.vue` 重写为 `TodoEditorPanel.vue`（脱离 `ModalShell`） |
| 9 | 模式 | `create / edit / child` 全字段；`due / remind / tags / remark` 只显示对应区段 | `mode` 三种 + `focus` 只移焦点 | `mode` 扩为七种；`TODO_SECTS` 表决定 `.te-sect` 显隐；`focus` prop 删除 |
| 10 | 标题 | ＋ 新建待办 / ＋ 添加子任务 / ⏰ 设置 / 更改提醒 / 📅 修改完成时间 / ✏️ 编辑待办 / 🏷️ 编辑标签 / 📄 编辑备注 | 三种 | 七种 |
| 11 | 提示 | 按模式七种（remind 模式文案依 D14 改为偏移口径） | 四种 | 见 §4.3 |
| 12 | 提醒区段内容 | 提醒日期 + 时间 | 偏移下拉 + 桌面 / 邮箱渠道（D14） | 保留 D14 内容，放进 `.te-sect[data-sect=remind]` |
| 13 | 定位 | 同差异 2，但**垂直对齐卡片顶部**、无锚点居中；入场 opacity 0 x ∓10 0.18s；退场 opacity 0.12s | 居中 | `anchorBeside()` 的 `align: 'top'`；顶部「＋」按钮以按钮为锚（原型同） |
| 14 | 关闭 | ✕ / 取消 / 点 overlay 空白 / Esc | 同 | 同；editor 窗内 overlay 铺满工作区，点空白关窗 |
| 15 | 校验 | 完成时间必填；子任务 ≤ 父；create / child 不早于当前（历史补录例外）；标签 ≤3·≤10 字 | 已有 | 保留 |
| **重复菜单** | | | | |
| 16 | `#repeatMenu` | `.repeat-menu` > `.repeat-opt[data-repeat=''\|daily\|weekly](.active)`；锚点下方 +6，右缘 ≤ 视口 −180；入场 y −6 0.18s；点外关闭；选中 toast | 无组件；面板 🔁 打开编辑窗，主窗口未接 | 共享件 `RepeatMenu.vue`（复用 `useAnchoredMenu`）；`TodosView` / `DayView` / `TodoPage` 接 `edit-repeat`，选中调 `api.todos.reminder(id, offset, desktop, email, rule)` |
| **标签管理 / 粘贴板编辑** | | | | |
| 17 | 入 / 退场 | scale .94 → 1 0.2s / .96 0.15s | 无 | `ModalShell` 加 `popIn` 入场、`pop` 退场（`motion/presets.ts` 已有） |
| 18 | 粘贴板编辑光标 | 打开时聚焦且光标置末 | 聚焦 | `ClipEditorModal` `setSelectionRange(len, len)` |
| 19 | 粘贴板编辑空内容 | toast「内容为空，未保存」 | 未拦截 | `ClipEditorModal` 内拦截 |
| 20 | 标签管理保存钮 | 无 | 有 | D25 保留（差异接受） |
| 21 | 面板内承载 | 屏幕级遮罩居中 | 整页化 | D25 保留（差异接受） |
| **Toast** | | | | |
| 22 | 参数 | 上浮 20px 0.25s；停 1800ms | 6px；2000ms | `useToast` 默认 1800；`ToastHost` 位移 20px |
| **优先级** | | | | |
| 23 | 阶梯菜单 | 三项全显 | D26 保留（差异接受） | — |

## 4. 设计

### 4.1 锚定纯函数 `src/utils/anchor.ts`（+ `anchor.test.ts`）

```ts
export interface Rect { left: number; top: number; width: number; height: number }
export interface AnchorOptions {
  /** 浮层尺寸 */ size: { width: number; height: number }
  /** 视口尺寸 */ viewport: { width: number; height: number }
  /** 垂直对齐：'center' 删除确认 / 'top' 待办编辑 */ align: 'center' | 'top'
  /** 左右都放不下时：'center' 原型（水平居中盖住） / 'below' D27（卡片下方） */ fallback: 'center' | 'below'
}
export interface AnchorResult { left: number; top: number; placement: 'right' | 'left' | 'center' | 'below'; caretY: number }
export function anchorBeside(anchor: Rect | null, opts: AnchorOptions): AnchorResult
```

算法逐字对应 `app.js:1108–1125`：右侧 `anchor.right + 14 + W <= vw - 10` → `left = right + 14`；否则左侧 `anchor.left - 14 - W >= 10` → `left = left - 14 - W`，`placement = 'left'`；否则兜底：`fallback === 'center'` → `left = max(10, (vw - W) / 2)`；`'below'` → `left = clamp(anchor.left, 10, vw - W - 10)`，`top = anchor.bottom + 8`（超出视口则改为 `anchor.top - H - 8`）。`top`（非 below）：`align === 'top'` → `anchor.top`；`'center'` → `anchor.top + h/2 - H/2`；再钳制 `[10, vh - H - 10]`。`caretY = clamp(anchor.top + h/2 - top, 14, H - 14)`。`anchor` 为 null → 居中，`placement = 'center'`。

测试：右侧放得下 / 左翻 / 两种兜底 / 顶部钳制 / caret 钳制 / 无锚点。

### 4.2 删除确认 `src/components/base/CardConfirm.vue`

- props：`text: string`、`targetId: string | null`、`container: HTMLElement | null`（列表容器，用于 `querySelector('[data-id="…"]')` 与 MutationObserver）、`fallback?: 'center' | 'below'`（默认 `'below'`；主窗口视图传 `'center'`——主窗口宽度足够，实际不会命中兜底）。
- emits：`confirm`、`cancel`。
- 行为：`targetId` 非空时 Teleport 到 body 渲染 `#cardConfirm`；`nextTick` 后取卡片 rect → `anchorBeside(rect, { align: 'center', … })` 写 `left/top/--caret-y` 与 `.flip`（`placement === 'left'`）/ `.below`（新增类，`extensions.css` §3 定义 `.cc-caret` 朝上）；找不到卡片 → emit `cancel`。首次显示 `enter()` 横向 10px；`ResizeObserver(cardConfirm)` + `MutationObserver(container, { childList, subtree })` 合帧重定位；`window` `scroll`(capture) 与 `resize` → emit `cancel`。
- 卡片：`NoteCard` / `ClipCard` / `ClipArchiveCard` / `TodoCard` / `DayView` 卡片加 `:data-id="…"`（DayView 用 `${kind}:${id}`）与 `:class="{ confirming }"`；删除内部 `ConfirmPopover` 用法与 `confirming` 时的浮层，保留 `confirming` prop 只用于类名。
- 接线（六处）：`NotesView`、`ClipsView`、`TodosView`（`TodoTree` 内）、`DayView`、`ClipPage`、`TodoPage`（`TodoTree` 内）各渲染一个 `<CardConfirm :text :target-id="confirm.pendingId" :container="listEl" @confirm @cancel>`。`TodoTree` 因内含确认态，自己渲染 `CardConfirm` 并通过 prop `fallback` 由父级指定；文案由 `TodoTree` 判断父 / 子。
- `useConfirmDelete` 不变。`dismissOverlays` 语义不变。

### 4.3 待办编辑 `src/components/todo/TodoEditorPanel.vue`

- props：`mode: 'create' | 'edit' | 'child' | 'due' | 'remind' | 'tags' | 'remark'`、`todo`、`parent`、`presetDate`、`anchor: Rect | null`（**editor 窗本地 CSS 像素**）。
- `TODO_SECTS`（原型 `app.js` 同名）：`create/edit/child → ['text','tags','remark','due','remind','prio']`；`due → ['due']`；`remind → ['remind']`；`tags → ['tags']`；`remark → ['remark']`。
- 标题：`create` ＋ 新建待办 / `child` ＋ 添加子任务 / `remind` ⏰ 设置 / 更改提醒 / `due` 📅 修改完成时间 / `edit` ✏️ 编辑待办 / `tags` 🏷️ 编辑标签 / `remark` 📄 编辑备注。
- 提示：`child` 「子任务完成时间不能晚于父待办（{父完成时间}）」；`remind` 「提醒在完成时间前 N 分钟触发，可选桌面弹窗 / 邮箱」（D14 口径）；`due` 「完成时间决定列表排序与逾期判定；子任务不能晚于父待办」；`tags` 「标签 ≤3 个、每个 ≤10 字；✕ 需再次点击确认删除」；`remark` 「备注 ≤200 字」；`create` 有 `presetDate` → 「完成时间默认 1 小时后，将归入 {date}（历史日期补录）」，否则「完成时间默认 1 小时后，可修改」；`edit` 「完成时间、任务内容必填」。
- 模板：`<div id="todoEditorOverlay" @click.self="emit('close')"><div id="todoEditorPanel" class="glass" :data-mode :class="{ flip }" :style="{ left, top, '--caret-y' }"><div class="te-caret" />…</div></div>`。定位：`onMounted` → `nextTick` 测 `offsetWidth/Height` → `anchorBeside(anchor, { align: 'top', fallback: 'center' })`；入场 `enter()` 横向 10px `--dur-base`；退场 `exit()` 后 emit `close`。Esc 由自身 `keydown` capture 处理（不再依赖 `ModalShell`）。
- 保存逻辑、校验、标签 / 备注 / 完成时间 / 优先级 / 提醒偏移与渠道字段全部从 `TodoEditorModal.vue` 迁入；聚焦模式只提交对应字段（其余字段取 `todo` 原值组装 `TodoInput`）。
- `TodoEditorModal.vue` 删除。

### 4.4 editor 窗锚点链路

- payload 增 `anchor?: { x: number; y: number; w: number; h: number }`（**物理屏幕像素**）。
- 调用方（`TodoPage` / `TodosView` / `DayView`）：新组合式 `src/composables/useEditorWindow.ts`：`openTodoEditor(opts: { mode; todoId?; parentId?; presetDate?; anchorEl?: HTMLElement | null })`：`anchorEl` 存在时 `const r = anchorEl.getBoundingClientRect(); const pos = await getCurrentWindow().innerPosition(); const s = await getCurrentWindow().scaleFactor(); anchor = { x: pos.x + r.left * s, y: pos.y + r.top * s, w: r.width * s, h: r.height * s }`；然后 `api.windows.editorOpen(JSON.stringify(payload))`。面板调用方在 invoke 前仍 `emit('externalEditor')`。
- `EditorApp.vue`：拿到 payload 后 `innerPosition()` + `scaleFactor()` 把 `anchor` 换算为本窗 CSS 像素 `Rect` 传给 `TodoEditorPanel`；`kind` 仍只有 `todo`。
- 锚点元素：卡片级（D28），调用方用 `listEl.querySelector('[data-id="${todo.id}"]')`；顶部「＋」按钮传 `event.currentTarget`。
- 主窗口 `TodosView` / `DayView`：删除 `TodoEditorModal` 用法与本地 `editor` 状态；`@save` 路径由 `EditorApp` 承担（已有 `saveTodo` → `todos.save`，主窗口经 `todosChanged` 事件刷新）。`DayView` 已监听 `notesChanged`，需同样监听 `todosChanged`（核对现有 `useTodos` 是否已驱动 `load()`；若无则加）。
- 样式：删 `extensions.css` §5 `#todoEditorOverlay` 覆写（生成层已有透明 overlay）；`window-fit.css` 删 `#todoEditorModal.modal-shell` 的 520px 规则（`#clipEditorModal` / `#tagManagerModal` 保留，供将来）。

### 4.5 重复菜单 `src/components/todo/RepeatMenu.vue`

- 复用 `useAnchoredMenu(() => 3)`；模板 `<div ref="menuRef" class="repeat-menu" :style><div v-for="opt in OPTIONS" class="repeat-opt" :class="{ active: opt.value === current }" :data-repeat="opt.value" @click="pick(opt.value)">…</div></div>`；OPTIONS：`''` 🚫 不重复 / `daily` 🔁 每天重复 / `weekly` 🔁 每周重复。
- expose `open(anchor: HTMLElement, current: string | null)`；emit `pick(rule: string | null)`。
- 调用方：`TodosView` / `DayView` / `TodoPage` 接 `edit-repeat(todo, anchor)` → `repeatMenu.open(anchor, todo.repeat_rule)`；`pick` → `api.todos.reminder(todo.id, todo.remind_offset_minutes, todo.remind_desktop, todo.remind_email, rule)` → toast「已设为{每天重复 / 每周重复 / 不重复}」。`TodoCard` 上 🔁 徽章仅在有 `repeat_rule` 时显示（现状），无规则时进入编辑窗 `remind` 模式即可设置——本阶段不加新入口。

### 4.6 ModalShell / ClipEditorModal / Toast

- `ModalShell`：`onMounted` 对 `.modal-shell` 调 `popIn()`；`close` 前 `pop()`（`presets.ts:146`，缩到 .96 淡出，时长 `--dur-fast`）后再 emit。面板整页化下动效同样播放（scale 小幅，可接受）。
- `ClipEditorModal`：`onMounted` → `textarea.focus(); textarea.setSelectionRange(len, len)`；`save()` 时 `draft.trim() === ''` → toast「内容为空，未保存」并 return。
- `useToast` 默认 1800ms；`ToastHost` `translate(-50%, 20px)`，时长 `--dur-base`。

## 5. 测试

- `src/utils/anchor.test.ts`：六个用例（§4.1）。
- 其余为组件与窗口行为，走实机。
- `pnpm typecheck`、`pnpm test`、`pnpm sync:styles --check`、`pnpm format:check`、`vite build`；Rust 无改动（payload 是透传字符串）。

## 6. 实机验收（打字机 + 深色）

1. 删除确认：主窗口笔记 / 粘贴板 / 待办 / 日期详情、面板粘贴板 / 待办，各点 ✕ → 浮层出现在卡片右侧带箭头；面板内落在卡片下方箭头朝上；首张卡片可见；滚动列表 → 浮层消失；Esc 只关浮层；确认删除生效；文案四种。
2. 待办编辑：主窗口 ✏️ / 📅 / ⏰ / 标签 / 备注 / ＋子任务 / 顶部 ＋ → editor 窗内浮层锚定卡片右侧（靠右卡片左翻）、标题与提示对应、聚焦模式只显示单区段；点空白 / Esc 关闭；保存后主窗口列表刷新；面板同路径。
3. 重复菜单：有重复规则的待办点 🔁 → 三项菜单锚在徽章下方，选择后 toast 并刷新徽章。
4. 标签管理 / 粘贴板编辑：入退场缩放；粘贴板编辑打开光标在末尾；清空保存被拦截。
5. Toast 上浮 20px、1.8 秒消失。
6. 回归：面板 Esc 链（删除确认 → 收面板）、切页清确认、Zen 中 Esc。

## 7. 提交批次（中文）

1. `feat(ui): 锚定纯函数 anchorBeside + 单测`
2. `feat(ui): 删除二次确认改为 body 级锚定浮层 CardConfirm，删除 ConfirmPopover 与桥接样式`
3. `feat(todo): 待办编辑浮层对齐原型（锚定卡片右侧 / 七种模式 / 标题与提示），主窗口与面板统一走 editor 窗`
4. `feat(todo): 重复提醒菜单 RepeatMenu 并接入三处待办列表`
5. `feat(ui): 弹窗缩放动效、粘贴板编辑光标与空内容拦截、toast 参数对齐原型`
6. `docs(design): 阶段四A 实机验收记录`

## 8. 风险

| 风险 | 应对 |
|---|---|
| `innerPosition` / `scaleFactor` 在 150% 缩放下与 editor 窗坐标系不一致 | editor 窗按工作区 `position / scale` 创建（`windows.rs:189–203`），两边都用物理像素换算；验收第 2 项在靠右卡片上核对左翻 |
| 主窗口改走 editor 窗后，编辑期间主窗口失焦 | editor 窗是置顶透明层，主窗口仍可见；关闭后焦点回主窗口 |
| `MutationObserver` 在长列表上频繁触发 | 合帧（`requestAnimationFrame` 去抖）只重定位一次 |
| 删除 `TodoEditorModal` 后 `EditorApp` 的 `focus` 语义变化 | payload `focus` 字段删除，改由 `mode` 表达；`TodoPage` 现有 `openEditor(mode, todo, parent, focus)` 调用改为 `mode` 直接传 `'due' / 'remind'` |

## 9. 实机结论（2026-09-12 回填，详见 `../plans/2026-09-12-prototype-realign-phase4a-acceptance.md`）

- 面板内删除确认兜底位置观感：**D27 的「卡片下方」可用**，首张卡不再被裁切；末张卡下方放不下时翻到上方但箭头仍朝上（`anchorBeside` 未区分 `above`，待补）。**主窗口卡片横贯整行同样命中兜底**，当前传 `center` 会盖住卡片正文，建议主窗口也改 `below`（待拍板）。
- editor 窗锚定：主屏 150% 下 ✏️ / 📅 / ＋子任务 / 顶部 ＋ 四种模式浮层均锚定到卡片或按钮右侧、箭头对齐；验收中途外接显示器、主窗口被系统移位后仍正确。多缩放双屏未测。
- 补丁清单：本阶段无 CSS 补丁；审查修复 `f850fe9`（保存在途守卫 / 聚焦模式校验 / 补跑清确认）、`8ac744c`（保存失败 toast / Esc 关确认 / 子任务继承父级 / 编辑窗自毁 / 菜单收口改名与顺序 / 🔁 pointer）。
- 未验证：深色轮、⏰ 🏷️ 📄 聚焦模式、重复菜单（无可用数据）、切页清确认与 Zen 回归，清单见验收记录 §5。
