# 原型对齐 · 阶段四 A「弹窗与浮层」实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把删除二次确认、待办编辑浮层、重复提醒菜单、标签管理 / 粘贴板编辑弹窗动效与 toast 参数对齐原型 `docs/`：删除确认改为 body 级唯一 `#cardConfirm` 锚定卡片右侧（面板内兜底落到卡片下方，D27）；待办编辑改为锚定卡片右侧的 `#todoEditorPanel`（七种模式、单区段聚焦），主窗口与面板统一走透明 `editor` 窗并通过物理像素锚点定位（D23）；新增 `RepeatMenu`；`ModalShell` 缩放入退场；toast 1800ms / 20px。

**Architecture:** 一个纯函数 `anchorBeside()` 同时服务删除确认与待办编辑的定位（可单测）。删除确认由各列表页 / `TodoTree` 各自渲染一个 Teleport 到 body 的 `CardConfirm`，用 `[data-id]` 反查卡片、Observer 合帧重定位；`useConfirmDelete` 不变。待办编辑不再有 `ModalShell` 版本：`TodoEditorPanel` 只在 `editor` 窗内渲染，调用方通过新组合式 `useEditorWindow` 把锚点卡片换算为物理屏幕像素塞进 payload，`EditorApp` 再换算回本窗 CSS 像素。Rust 本阶段零改动（payload 透传字符串）。

**Tech Stack:** Vue 3.5 `<script setup lang="ts">` · TypeScript 5.9 · Vite 7 · animejs 4.5 · `@tauri-apps/api` 2.11（`getCurrentWindow().innerPosition() / scaleFactor()`）· Node 24（`node:test`）· Tauri 2 / Rust（不改）

**设计文档：** `docs/superpowers/specs/2026-09-12-prototype-realign-phase4a-overlays-design.md`（下文简称 spec；决策 §2 D22–D28、差异表 §3、设计 §4、验收 §6、提交批次 §7）

## Global Constraints

- 唯一参考原型 `docs/index.html` + `docs/styles.css` + `docs/app.js`；生成层样式（`tokens.css` / `base.css` / `components.css` / `themes.css`）**不改**，`pnpm sync:styles --check` 必须始终通过；项目自有样式只进 `src/styles/extensions.css`（本阶段：§3 补充两条规则；**删除 §5 整节**与 §6 中的 `.card-confirm` / `.todo-item:has(.card-confirm) .todo-del` / `#todoEditorModal*` / `.te-field-date|time|priority`）与 `window-fit.css`。
- 组件模板只用原型类名（`#cardConfirm` / `.cc-caret` / `.card-confirm-text` / `#todoEditorOverlay` / `#todoEditorPanel` / `.te-caret` / `.te-body` / `.te-sect[data-sect]` / `.repeat-menu` / `.repeat-opt`）；不新增设计语言；颜色只用令牌。
- 已拍定决策（spec §2）：D23 待办编辑统一走 `editor` 窗 + 物理像素锚点；D25 面板内标签管理 / 粘贴板编辑保留整页化；D26 优先级菜单保留三项全显；D27 面板内删除确认兜底落卡片下方（箭头朝上、左缘对齐）；D28 锚点一律卡片级（`[data-id]`）。
- 编码规范：Vue 一律 `<script setup lang="ts">`，禁止 `any`；关键节点走 `src/service/logger.ts`；每个文件头部有职责说明注释（SFC 放在 `<script setup>` 顶部的块注释）。
- 动效硬约束沿用阶段一：只动 transform / opacity；`.glass` 静止态无 transform（入退场由 `src/motion` 预设驱动并在结束时清除内联样式）；JS 驱动、可重播。
- **Rust 本阶段不改。** 若实施中发现必须改 Rust（例如 `editor_open` 需要新参数），不要自行加，把该项标记为 **BLOCKED** 写进任务末尾并停下等拍板。
- 验证：每任务结束 `pnpm typecheck` 零错误；改动纯函数（Task 1）时 `pnpm test:unit`；改动样式时 `pnpm sync:styles --check`；最后 `pnpm exec vite build`。
- 提交：中文提交信息，按 spec §7 的 6 个批次，每任务一次提交（Task 6 为验收记录提交）。

---

## File Structure

```
src/
├─ utils/
│  ├─ anchor.ts                          新：anchorBeside() 纯函数（Rect / AnchorOptions / AnchorResult）
│  └─ anchor.test.ts                     新：六个用例
├─ motion/presets.ts                     SlideOptions 增 duration 档位（enter / exit 可用 fast / base）；新增 popOut()
├─ motion/index.ts                       导出 popOut
├─ components/base/
│  ├─ CardConfirm.vue                    新：body 级删除确认浮层（Teleport / anchorBeside / Observer 重定位）
│  ├─ ConfirmPopover.vue                 删除
│  ├─ ModalShell.vue                     popIn 入场 / popOut 退场
│  └─ ToastHost.vue                      位移 20px
├─ components/card/
│  ├─ NoteCard.vue                       去 ConfirmPopover；data-id + .confirming；删 confirm/cancel-delete 事件
│  ├─ ClipCard.vue                       同上
│  ├─ ClipArchiveCard.vue                同上
│  ├─ TodoCard.vue                       同上；新增 edit-remark 事件
│  └─ TodoTree.vue                       内置 CardConfirm；prop confirmFallback；expose cardOf(id)；透传 edit-remark
├─ components/todo/
│  ├─ TodoEditorModal.vue                删除
│  ├─ TodoEditorPanel.vue                新：锚定浮层（七种模式 / TODO_SECTS / 标题与提示 / 自定位 / Esc / 入退场）
│  └─ RepeatMenu.vue                     新：重复提醒菜单（useAnchoredMenu）
├─ components/clip/ClipEditorModal.vue   光标置末 / 空内容拦截
├─ constants/todoEditor.ts               新：TodoEditorMode / TodoEditorSect / TODO_SECTS / TODO_EDITOR_TITLES
├─ composables/
│  ├─ useConfirmDelete.ts                仅更新头注释（行为不变）
│  ├─ useEditorWindow.ts                 新：物理像素锚点换算 + editorOpen（TodoEditorPayload 类型）
│  └─ useToast.ts                        默认 1800ms
├─ windows/Editor/EditorApp.vue          渲染 TodoEditorPanel；anchor 换算回本窗 CSS 像素；parent 兼容 todo.parent_id
├─ windows/Main/
│  ├─ NotesView.vue                      CardConfirm 接线
│  ├─ ClipsView.vue                      CardConfirm 接线
│  ├─ TodosView.vue                      删 TodoEditorModal；useEditorWindow；RepeatMenu；confirm-fallback="center"
│  └─ DayView.vue                        CardConfirm 接线（`${kind}:${id}`）；useEditorWindow；RepeatMenu；监听 todosChanged
├─ windows/Panel/
│  ├─ ClipPage.vue                       CardConfirm 接线
│  └─ TodoPage.vue                       useEditorWindow（七种 mode）；RepeatMenu
└─ styles/
   ├─ extensions.css                     §3 追加 .todo-item.confirming 与 #cardConfirm.below；删 §5 整节；删 §6 四组规则
   └─ window-fit.css                     editor 窗 #todoEditorOverlay 接管鼠标；删 #todoEditorModal 选择器
docs/superpowers/plans/2026-09-12-prototype-realign-phase4a-acceptance.md   验收记录（Task 6）
```

---

## Task 1: 锚定纯函数 `anchorBeside()` + 单测（TDD）

**Files:**
- Create: `src/utils/anchor.test.ts`
- Create: `src/utils/anchor.ts`

**Interfaces:**
- Produces（spec §4.1）：

```ts
export interface Rect { left: number; top: number; width: number; height: number }
export interface AnchorOptions {
  size: { width: number; height: number }
  viewport: { width: number; height: number }
  align: 'center' | 'top'
  fallback: 'center' | 'below'
}
export interface AnchorResult { left: number; top: number; placement: 'right' | 'left' | 'center' | 'below'; caretY: number }
export function anchorBeside(anchor: Rect | null, opts: AnchorOptions): AnchorResult
```

- 常量（与 `docs/app.js:1108–1125`、`:174–184` 逐字对应）：卡片与浮层间距 14、视口留白 10、下方兜底间距 8、箭头距边缘最小 14。

- [ ] **Step 1: 写六个失败用例**

`src/utils/anchor.test.ts`：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import { anchorBeside, type AnchorOptions } from './anchor'

/** 1280×800 视口、200×60 浮层：绝大多数用例共用。 */
function opts(partial: Partial<AnchorOptions> = {}): AnchorOptions {
  return {
    size: { width: 200, height: 60 },
    viewport: { width: 1280, height: 800 },
    align: 'center',
    fallback: 'center',
    ...partial,
  }
}

test('anchorBeside：右侧放得下 → 卡片右缘 +14，垂直居中，箭头对齐卡片中心', () => {
  const result = anchorBeside({ left: 100, top: 300, width: 400, height: 100 }, opts())
  assert.equal(result.placement, 'right')
  assert.equal(result.left, 514) // 100 + 400 + 14
  assert.equal(result.top, 320) // 300 + 50 - 30
  assert.equal(result.caretY, 30) // 350 - 320
})

test('anchorBeside：右侧不够 → 翻到左侧（卡片左缘 −14 − W）', () => {
  const result = anchorBeside({ left: 1000, top: 300, width: 200, height: 100 }, opts())
  // 右侧 1200 + 14 + 200 = 1414 > 1270；左侧 1000 − 14 − 200 = 786 ≥ 10
  assert.equal(result.placement, 'left')
  assert.equal(result.left, 786)
  assert.equal(result.top, 320)
})

test('anchorBeside：两侧都不够，fallback=center → 水平居中盖住卡片（原型）', () => {
  const result = anchorBeside(
    { left: 20, top: 100, width: 440, height: 80 },
    opts({ viewport: { width: 480, height: 600 } }),
  )
  // 右侧 460 + 14 + 200 = 674 > 470；左侧 20 − 214 < 10
  assert.equal(result.placement, 'center')
  assert.equal(result.left, 140) // (480 − 200) / 2
  assert.equal(result.top, 110) // 100 + 40 − 30
})

test('anchorBeside：两侧都不够，fallback=below → 卡片下方 +8、左缘对齐；下方放不下则改到上方', () => {
  const below = anchorBeside(
    { left: 20, top: 100, width: 440, height: 80 },
    opts({ viewport: { width: 480, height: 600 }, fallback: 'below' }),
  )
  assert.equal(below.placement, 'below')
  assert.equal(below.left, 20)
  assert.equal(below.top, 188) // 100 + 80 + 8
  // 视口极窄时左缘超出可放范围 → left 钳制到 vw − W − 10
  const clamped = anchorBeside(
    { left: 60, top: 100, width: 100, height: 80 },
    opts({ viewport: { width: 230, height: 600 }, fallback: 'below' }),
  )
  // 右侧 160 + 214 > 220；左侧 60 − 214 < 10；下方 left = clamp(60, 10, 20)
  assert.equal(clamped.placement, 'below')
  assert.equal(clamped.left, 20)
  // 卡片贴底：下方 + 8 放不下 → 放到卡片上方 −8
  const above = anchorBeside(
    { left: 20, top: 520, width: 440, height: 70 },
    opts({ viewport: { width: 480, height: 600 }, fallback: 'below' }),
  )
  assert.equal(above.placement, 'below')
  assert.equal(above.top, 452) // 520 − 60 − 8
})

test('anchorBeside：align=top 对齐卡片顶部，且 top 钳制在 [10, vh − H − 10]', () => {
  const top = anchorBeside({ left: 100, top: 300, width: 400, height: 100 }, opts({ align: 'top' }))
  assert.equal(top.top, 300)
  // 卡片在视口顶部以上 → 钳到 10
  const clampedTop = anchorBeside({ left: 100, top: -40, width: 400, height: 100 }, opts({ align: 'top' }))
  assert.equal(clampedTop.top, 10)
  // 卡片贴底 → 钳到 800 − 60 − 10
  const clampedBottom = anchorBeside({ left: 100, top: 790, width: 400, height: 100 }, opts({ align: 'top' }))
  assert.equal(clampedBottom.top, 730)
})

test('anchorBeside：caretY 钳制在 [14, H − 14]；无锚点则居中且 placement=center', () => {
  // 卡片贴顶（top 钳到 10），卡片中心 15 → 箭头 5 → 钳到 14
  const low = anchorBeside({ left: 100, top: -20, width: 400, height: 70 }, opts())
  assert.equal(low.caretY, 14)
  // 卡片贴底（top 钳到 730），卡片中心 795 → 箭头 65 → 钳到 46
  const high = anchorBeside({ left: 100, top: 760, width: 400, height: 70 }, opts())
  assert.equal(high.caretY, 46)
  const centered = anchorBeside(null, opts())
  assert.deepEqual(centered, { left: 540, top: 370, placement: 'center', caretY: 30 })
})
```

- [ ] **Step 2: 运行，确认失败**

Run: `pnpm test:unit 2>&1 | grep -E "anchor|error TS|Cannot find" | head`
Expected: `tsc -p tsconfig.test.json` 报 `Cannot find module './anchor'`（`tsconfig.test.json` 的 include 已含 `src/utils/**/*.ts`，无需改）。

- [ ] **Step 3: 实现 `src/utils/anchor.ts`**

```ts
/**
 * 浮层锚定纯函数：把浮层摆到锚点卡片旁边（原型 app.js `positionTodoEditor` / `refreshCardConfirm` 的定位段）。
 *
 * 优先卡片右侧（+14）→ 右侧不够翻到左侧（−14 − W，placement = 'left'，箭头转到右缘）→ 两侧都不够时按
 * `fallback` 兜底：'center' 是原型的水平居中盖住卡片；'below' 是 spec D27 的卡片下方（箭头朝上、左缘对齐，
 * 下方也放不下时改到卡片上方）。纵向 `align` 'center' 对齐卡片中心（删除确认）、'top' 对齐卡片顶部（待办编辑），
 * 再钳制在视口留白内；`caretY` 是箭头相对浮层顶部的纵向位置，钳制在 [14, H − 14]。
 *
 * 不碰 DOM，输入输出都是数字，便于单测；调用方负责 getBoundingClientRect / offsetWidth 的测量。
 */

export interface Rect {
  left: number
  top: number
  width: number
  height: number
}

export interface AnchorOptions {
  /** 浮层尺寸。 */
  size: { width: number; height: number }
  /** 视口尺寸。 */
  viewport: { width: number; height: number }
  /** 垂直对齐：'center' 删除确认 / 'top' 待办编辑。 */
  align: 'center' | 'top'
  /** 左右都放不下时：'center' 原型（水平居中盖住） / 'below' D27（卡片下方）。 */
  fallback: 'center' | 'below'
}

export interface AnchorResult {
  left: number
  top: number
  placement: 'right' | 'left' | 'center' | 'below'
  /** 箭头相对浮层顶部的纵向位置（px）；placement = 'below' 时箭头改为朝上，此值不再使用。 */
  caretY: number
}

/** 卡片与浮层之间的水平间距（原型 `r.right + 14`）。 */
const SIDE_GAP = 14
/** 视口四周留白（原型 `innerWidth - 10` / `Math.max(10, …)`）。 */
const VIEWPORT_MARGIN = 10
/** D27 兜底：卡片与浮层之间的垂直间距。 */
const BELOW_GAP = 8
/** 箭头距浮层上下边缘的最小距离（原型 `Math.max(14, …)`）。 */
const CARET_MIN = 14

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(value, max))
}

export function anchorBeside(anchor: Rect | null, opts: AnchorOptions): AnchorResult {
  const { width: W, height: H } = opts.size
  const { width: vw, height: vh } = opts.viewport
  const maxLeft = vw - W - VIEWPORT_MARGIN
  const maxTop = vh - H - VIEWPORT_MARGIN

  // 无锚点（如顶部新增按钮未传锚点）：居中。
  if (!anchor) {
    return {
      left: Math.max(VIEWPORT_MARGIN, (vw - W) / 2),
      top: Math.max(VIEWPORT_MARGIN, (vh - H) / 2),
      placement: 'center',
      caretY: clamp(H / 2, CARET_MIN, H - CARET_MIN),
    }
  }

  const right = anchor.left + anchor.width
  const bottom = anchor.top + anchor.height
  const centerY = anchor.top + anchor.height / 2

  let left: number
  let placement: AnchorResult['placement']
  if (right + SIDE_GAP + W <= vw - VIEWPORT_MARGIN) {
    left = right + SIDE_GAP
    placement = 'right'
  } else if (anchor.left - SIDE_GAP - W >= VIEWPORT_MARGIN) {
    left = anchor.left - SIDE_GAP - W
    placement = 'left'
  } else if (opts.fallback === 'center') {
    left = Math.max(VIEWPORT_MARGIN, (vw - W) / 2)
    placement = 'center'
  } else {
    // D27：卡片下方、左缘对齐；下方放不下则改到卡片上方。
    left = clamp(anchor.left, VIEWPORT_MARGIN, maxLeft)
    let top = bottom + BELOW_GAP
    if (top > maxTop) top = anchor.top - H - BELOW_GAP
    top = clamp(top, VIEWPORT_MARGIN, maxTop)
    return { left, top, placement: 'below', caretY: clamp(centerY - top, CARET_MIN, H - CARET_MIN) }
  }

  const rawTop = opts.align === 'top' ? anchor.top : centerY - H / 2
  const top = clamp(rawTop, VIEWPORT_MARGIN, maxTop)
  const caretY = clamp(centerY - top, CARET_MIN, H - CARET_MIN)
  return { left, top, placement, caretY }
}
```

- [ ] **Step 4: 运行测试通过**

Run: `pnpm test:unit 2>&1 | grep -E "anchorBeside|^# (pass|fail)"`
Expected: 六个 `ok`，`# fail 0`。

- [ ] **Step 5: 校验并提交**

```bash
pnpm typecheck && pnpm format:check
git add src/utils/anchor.ts src/utils/anchor.test.ts
git commit -m "feat(ui): 锚定纯函数 anchorBeside + 单测"
```

---

## Task 2: 删除二次确认改为 body 级锚定浮层 `CardConfirm`，删除 `ConfirmPopover` 与桥接样式

**Files:**
- Modify: `src/motion/presets.ts:18-24,74-114`（`SlideOptions.duration`）
- Create: `src/components/base/CardConfirm.vue`
- Delete: `src/components/base/ConfirmPopover.vue`
- Modify: `src/components/card/NoteCard.vue`、`ClipCard.vue`、`ClipArchiveCard.vue`、`TodoCard.vue`、`TodoTree.vue`
- Modify: `src/windows/Main/NotesView.vue`、`ClipsView.vue`、`TodosView.vue`、`DayView.vue`、`src/windows/Panel/ClipPage.vue`
- Modify: `src/composables/useConfirmDelete.ts:4-10`（仅头注释）
- Modify: `src/styles/extensions.css:79-103`（§3 追加）、`:111-137`（§5 缩减为只剩 `#todoEditorOverlay`）、`:424-447`（§6 删除）

**Interfaces:**
- Consumes: Task 1 `anchorBeside / Rect`；`useConfirmDelete().pendingId / isPending / ask / cancel / confirm`（不变）。
- Produces:
  - `SlideOptions.duration?: 'fast' | 'base' | 'slow'`（默认 `enter` 用 `slow`、`exit` 用 `base`，与现状一致；非默认档位时 `enter` 改用 `--ease-out` 而非 outBack）。
  - `CardConfirm` props：`text: string`、`targetId: string | null`、`container: HTMLElement | null`、`fallback?: 'center' | 'below'`（默认 `'below'`）；emits `confirm` / `cancel`。
  - 卡片组件：`NoteCard` / `ClipCard` / `ClipArchiveCard` / `TodoCard` 根元素带 `:data-id` 与 `.confirming`；emits **删除** `confirm-delete` / `cancel-delete`（保留 `ask-delete`）。
  - `TodoTree` 新 prop `confirmFallback?: 'center' | 'below'`（默认 `'below'`）；expose 增 `cardOf(id: string): HTMLElement | null`（Task 3 的锚点查找用）。

- [ ] **Step 1: `presets.ts` 时长档位**

第 18–24 行 `SlideOptions` 替换为：

```ts
export interface SlideOptions {
  axis: Axis
  /** 位移距离（px），正负即方向：入场从 distance 滑到 0，收起从 0 滑到 distance。 */
  distance: number
  /** 入场起始缩放（如 0.97）；不传则不缩放。 */
  scale?: number
  /**
   * 时长档位：enter 默认 'slow'（面板入场，outBack 回弹），exit 默认 'base'。
   * 小浮层（删除确认 / 待办编辑）按原型用 .15s / .18s，传 'fast' / 'base'；非默认档位的 enter 改用 --ease-out，不回弹。
   */
  duration?: 'fast' | 'base' | 'slow'
}
```

`enter()`（第 75–91 行）替换为：

```ts
/** 入场：位移 + 淡入（+ 可选缩放）。默认 --dur-slow + outBack(1.6)（原型 showPanel）；指定档位时 --ease-out。 */
export function enter(el: HTMLElement, opts: SlideOptions): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) {
    cancelRunning(el)
    clearInline(el)
    return Promise.resolve(true)
  }
  const prop = opts.axis === 'x' ? 'translateX' : 'translateY'
  const tier = opts.duration ?? 'slow'
  logger.debug('motion', `enter ${prop} ${opts.distance}px → 0 (${tier})`)
  return run(el, [el], {
    [prop]: [opts.distance, 0],
    opacity: [0, 1],
    ...(opts.scale === undefined ? {} : { scale: [opts.scale, 1] }),
    duration: tokens[tier],
    ease: tier === 'slow' ? 'outBack(1.6)' : tokens.easeOut,
  })
}
```

`exit()` 里 `duration: tokens.base,` 改为 `duration: tokens[opts.duration ?? 'base'],`。

- [ ] **Step 2: 新建 `src/components/base/CardConfirm.vue`**

```vue
<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch, type CSSProperties } from 'vue'
import { enter } from '@/motion'
import { logger } from '@/service/logger'
import { anchorBeside, type AnchorResult } from '@/utils/anchor'

/**
 * 删除二次确认浮层（原型 body 级唯一 `#cardConfirm`，spec §4.2）。
 *
 * - `targetId` 非空时 Teleport 到 body 渲染，按 `container.querySelector('[data-id="…"]')` 反查卡片，
 *   用 anchorBeside() 摆到卡片右侧（右侧不够翻左侧 `.flip`；两侧都不够按 `fallback`：主窗口 'center'、
 *   面板 'below' = 卡片下方箭头朝上，spec D27）；
 * - 首次显示横向 10px 滑入（--dur-fast）；浮层自身尺寸变化（ResizeObserver）与列表重渲染
 *   （容器 MutationObserver，childList + subtree）合帧重定位；卡片消失 → emit cancel；
 * - 任意滚动（capture）与窗口 resize → emit cancel（原型 app.js:209–210）；Esc 由各页 dismissOverlays 负责。
 *
 * 一个列表页 / TodoTree 各渲染一个实例；同一窗口内同一时刻只会有一个 targetId 非空（v-if 互斥的视图）。
 */
const props = withDefaults(
  defineProps<{
    /** 文案，四种：确认删除该笔记？ / 该条目？ / 该待办事项？ / 该子任务？ */
    text: string
    /** 待确认卡片的 data-id；null = 隐藏。 */
    targetId: string | null
    /** 列表容器：反查卡片与观察重渲染。 */
    container: HTMLElement | null
    /** 左右都放不下时的兜底：'center' 原型 / 'below' D27。 */
    fallback?: 'center' | 'below'
  }>(),
  { fallback: 'below' },
)

const emit = defineEmits<{ (e: 'confirm'): void; (e: 'cancel'): void }>()

const popRef = ref<HTMLElement | null>(null)
const position = ref<AnchorResult>({ left: 0, top: 0, placement: 'right', caretY: 28 })

const style = computed<CSSProperties>(() => ({
  left: `${position.value.left}px`,
  top: `${position.value.top}px`,
  '--caret-y': `${position.value.caretY}px`,
}))

let resizeObserver: ResizeObserver | null = null
let mutationObserver: MutationObserver | null = null
let frame = 0

function findCard(): HTMLElement | null {
  if (!props.container || !props.targetId) return null
  return props.container.querySelector<HTMLElement>(`[data-id="${CSS.escape(props.targetId)}"]`)
}

/** 反查卡片并定位；找不到卡片（已被删除 / 列表已切换）则取消确认态。 */
function reposition(first: boolean): void {
  const pop = popRef.value
  if (!pop || !props.targetId) return
  const card = findCard()
  if (!card) {
    logger.debug('card-confirm', `卡片 ${props.targetId} 已不在列表，清除确认态`)
    emit('cancel')
    return
  }
  const rect = card.getBoundingClientRect()
  position.value = anchorBeside(
    { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
    {
      size: { width: pop.offsetWidth, height: pop.offsetHeight },
      viewport: { width: window.innerWidth, height: window.innerHeight },
      align: 'center',
      fallback: props.fallback,
    },
  )
  if (first) {
    logger.debug('card-confirm', `显示 id=${props.targetId} placement=${position.value.placement}`)
    // 原型：opacity 0、x ∓10 → 0，.15s
    void enter(pop, { axis: 'x', distance: position.value.placement === 'left' ? 10 : -10, duration: 'fast' })
  }
}

/** Observer 回调合帧：一轮重渲染可能触发多次变更，只定位一次。 */
function scheduleReposition(): void {
  if (frame) return
  frame = requestAnimationFrame(() => {
    frame = 0
    reposition(false)
  })
}

function onScroll(): void {
  emit('cancel')
}

function onResize(): void {
  emit('cancel')
}

function attach(): void {
  detach()
  if (popRef.value) {
    resizeObserver = new ResizeObserver(scheduleReposition)
    resizeObserver.observe(popRef.value)
  }
  if (props.container) {
    mutationObserver = new MutationObserver(scheduleReposition)
    mutationObserver.observe(props.container, { childList: true, subtree: true })
  }
  window.addEventListener('scroll', onScroll, true)
  window.addEventListener('resize', onResize)
}

function detach(): void {
  resizeObserver?.disconnect()
  resizeObserver = null
  mutationObserver?.disconnect()
  mutationObserver = null
  window.removeEventListener('scroll', onScroll, true)
  window.removeEventListener('resize', onResize)
  if (frame) {
    cancelAnimationFrame(frame)
    frame = 0
  }
}

watch(
  () => [props.targetId, props.container] as const,
  async ([id]) => {
    if (!id) {
      detach()
      return
    }
    // 等 Teleport 内容挂载出真实尺寸再定位，否则 offsetWidth 为 0。
    await nextTick()
    reposition(true)
    attach()
  },
  { immediate: true, flush: 'post' },
)

onBeforeUnmount(detach)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.targetId"
      id="cardConfirm"
      ref="popRef"
      role="alertdialog"
      aria-live="polite"
      :class="{ flip: position.placement === 'left', below: position.placement === 'below' }"
      :style="style"
    >
      <div class="cc-caret" />
      <span class="card-confirm-text">{{ props.text }}</span>
      <button type="button" class="btn tiny danger" @click.stop="emit('confirm')">删除</button>
      <button type="button" class="btn tiny ghost" @click.stop="emit('cancel')">取消</button>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 3: 四张卡片去 `ConfirmPopover`，加 `data-id` / `.confirming`**

`src/components/card/NoteCard.vue`：
- 删除第 3 行 `import ConfirmPopover …`；
- 头注释第 15 行改为 `* - 右上角 ✕（悬浮显示；二次确认浮层由列表页的 CardConfirm 按 data-id 锚定到卡片右侧）；正文按 Markdown 渲染；思维导图笔记正文前置「🧠 思维导图」徽章，`；
- props 第 24 行 `confirming?: boolean` 上方加注释 `/** 删除确认态：只用于卡片类名 .confirming，浮层本体在 CardConfirm。 */`；
- emits（第 33–41 行）删除 `(e: 'confirm-delete'): void` 与 `(e: 'cancel-delete'): void` 两行；
- 模板第 56–62 行替换为：

```vue
  <div
    class="archive-item"
    :class="{ pinned: props.note.pinned, shaking: props.shaking, confirming: props.confirming }"
    :data-id="props.note.id"
  >
```

`src/components/card/ClipCard.vue`：
- 删除第 3 行 import；头注释第 13 行改为 `* - 右上角 .card-close（悬浮显示；二次确认浮层由 ClipPage 的 CardConfirm 按 data-id 锚定）；`；
- emits 删除 `confirm-delete` / `cancel-delete` 两行；
- 模板第 40–46 行替换为：

```vue
  <li
    class="clip-item"
    :class="{ pinned: props.entry.pinned, confirming: props.confirming }"
    :data-id="props.entry.id"
    :title="props.entry.preview"
    @dblclick="emit('paste')"
  >
```

`src/components/card/ClipArchiveCard.vue`：
- 删除第 3 行 import；emits 删除 `confirm-delete` / `cancel-delete`；
- 模板第 36–42 行替换为：

```vue
  <div class="archive-item clip-arch" :class="{ pinned: props.entry.pinned, confirming: props.confirming }" :data-id="props.entry.id">
```

`src/components/card/TodoCard.vue`：
- 删除第 3 行 import；
- 头注释第 19–20 行改为两行：`* - 右上角 ✕：仅未完成项渲染，仅悬浮**本卡片自身内容区**时显示（CSS .todo-item > .todo-body:hover 负责父子层级隔离）；` 与 `*   删除确认浮层由 TodoTree 的 CardConfirm 按 data-id 锚定，确认态下卡片带 .confirming 让 ✕ 常显；`；
- emits 第 62–75 行替换为（删两个确认事件，新增 `edit-remark`，Task 3 接线）：

```ts
const emit = defineEmits<{
  (e: 'toggle-done'): void
  (e: 'toggle-collapse'): void
  (e: 'open-priority', anchor: HTMLElement): void
  (e: 'edit-due'): void
  (e: 'edit-remind'): void
  (e: 'edit-repeat', anchor: HTMLElement): void
  (e: 'edit'): void
  /** 点击备注徽章 / 备注文本行：只编辑备注（原型 data-todoact="remark"）。 */
  (e: 'edit-remark'): void
  (e: 'add-sub'): void
  (e: 'ask-delete'): void
  (e: 'open-tags'): void
}>()
```

- 模板第 117–135 行替换为：

```vue
  <li
    class="todo-item"
    :class="{
      done,
      overdue,
      collapsed: props.collapsed,
      'has-children': props.hasChildren,
      [`depth-${props.depth}`]: props.depth > 0,
      'search-hit': props.searchHit,
      confirming: props.confirming,
    }"
    :data-id="props.todo.id"
  >
    <!-- todo-body 是悬浮判定范围：CSS 用直接子代选择器实现父子按钮隔离 -->
    <div class="todo-body">
```

- 两处 `RemarkDisplay` 的 `@edit="emit('edit')"`（第 181、187 行）改为 `@edit="emit('edit-remark')"`。

- [ ] **Step 4: `TodoTree.vue` 内置 `CardConfirm`**

第 4 行 import 之后加 `import CardConfirm from '@/components/base/CardConfirm.vue'`。

props（第 20–37 行）在 `archive?: boolean` 之后加：

```ts
    /** 删除确认浮层左右都放不下时的兜底：面板 'below'（D27）、主窗口 'center'。 */
    confirmFallback?: 'center' | 'below'
```

默认值对象追加 `confirmFallback: 'below'`。

emits（第 39–49 行）在 `(e: 'edit', todo: Todo): void` 之后加 `(e: 'edit-remark', todo: Todo): void`。

第 51 行 `const confirm = …` 之后加：

```ts
/** 列表根元素：CardConfirm 反查卡片、Task 3 的编辑浮层锚点都从这里查。 */
const listEl = ref<HTMLElement | null>(null)

/** 待确认项的文案：子任务与父待办措辞不同（原型 pendingDeleteText）。 */
const confirmText = computed(() => {
  const id = confirm.pendingId.value
  const target = id ? props.todos.find((todo) => todo.id === id) : undefined
  return target?.parent_id ? '确认删除该子任务？' : '确认删除该待办事项？'
})

/** 按待办 id 取卡片元素（编辑浮层锚点，spec D28）。 */
function cardOf(id: string): HTMLElement | null {
  return listEl.value?.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`) ?? null
}
```

`confirmDelete`（第 97–99 行）替换为：

```ts
/** CardConfirm 确认：pendingId 就是要删的 id，从当前列表找回对象派发。 */
function confirmDelete(): void {
  const id = confirm.confirm()
  const target = id ? props.todos.find((todo) => todo.id === id) : undefined
  if (target) emit('delete', target)
}
```

第 108 行改为 `defineExpose({ dismissConfirm, cardOf })`。

模板：第 112 行改为 `<ul ref="listEl" v-stagger-list class="todo-list" :class="{ 'todo-arch-list': props.archive }">`；父卡片（第 131 行 `@edit` 之后）加 `@edit-remark="emit('edit-remark', node.todo)"`，删除第 138–139 行 `@confirm-delete` / `@cancel-delete`；子卡片同样加 `@edit-remark="emit('edit-remark', child)"`，删除 `@confirm-delete` / `@cancel-delete`；`<PriorityMenu …/>` 之后加：

```vue
    <CardConfirm
      :text="confirmText"
      :target-id="confirm.pendingId.value"
      :container="listEl"
      :fallback="props.confirmFallback"
      @confirm="confirmDelete"
      @cancel="confirm.cancel()"
    />
```

- [ ] **Step 5: 五个列表页接线**

`src/windows/Main/NotesView.vue`：
- 第 4 行之后加 `import CardConfirm from '@/components/base/CardConfirm.vue'`；
- 第 30 行 `const keyword = ref('')` 之后加 `const listEl = ref<HTMLElement | null>(null)`；
- `remove()`（第 90–99 行）替换为：

```ts
/** CardConfirm 确认：按 pendingId 删除。 */
async function remove(): Promise<void> {
  const id = confirm.confirm()
  if (!id) return
  try {
    await api.notes.remove(id)
    toast('已删除')
  } catch (error) {
    logger.error('notes-view', '删除失败', error)
    toast('删除失败')
  }
}
```

- 模板第 173 行改为 `<div ref="listEl" v-stagger-list>`；删除第 186–187 行 `@confirm-delete="remove(note)"` / `@cancel-delete="confirm.cancel()"`；列表容器 `</div>`（第 190 行）之后插入：

```vue
    <CardConfirm
      text="确认删除该笔记？"
      :target-id="confirm.pendingId.value"
      :container="listEl"
      fallback="center"
      @confirm="remove"
      @cancel="confirm.cancel()"
    />
```

`src/windows/Main/ClipsView.vue`：同款——第 4 行后加 import、`keyword` 后加 `listEl`；`remove()` 改为无参：

```ts
/** CardConfirm 确认：按 pendingId 删除。 */
async function remove(): Promise<void> {
  const id = confirm.confirm()
  if (!id) return
  try {
    await api.clipboard.remove(id)
    toast('已删除')
  } catch (error) {
    logger.error('clips-view', '删除失败', error)
    toast('删除失败')
  }
}
```

模板 `<div ref="listEl" v-stagger-list>`；删除 `@confirm-delete` / `@cancel-delete`；列表容器之后插入：

```vue
    <CardConfirm
      text="确认删除该条目？"
      :target-id="confirm.pendingId.value"
      :container="listEl"
      fallback="center"
      @confirm="remove"
      @cancel="confirm.cancel()"
    />
```

`src/windows/Main/TodosView.vue`：`<TodoTree` 增加 `confirm-fallback="center"`（主窗口宽度足够，实际不会命中兜底）。

`src/windows/Panel/ClipPage.vue`：第 5 行后加 import、`keyword` 后加 `listEl`；`remove()` 改为与 ClipsView 相同的无参版本（scope `'panel-clip'`）；模板 `<ul ref="listEl" v-stagger-list class="clip-list">`；删除 `@confirm-delete` / `@cancel-delete`；`</ul>` 之后插入（默认 `fallback='below'`）：

```vue
    <CardConfirm
      text="确认删除该条目？"
      :target-id="confirm.pendingId.value"
      :container="listEl"
      @confirm="remove"
      @cancel="confirm.cancel()"
    />
```

`src/windows/Main/DayView.vue`：
- 第 4 行 `import ConfirmPopover …` 改为 `import CardConfirm from '@/components/base/CardConfirm.vue'`；
- 第 41 行 `const keyword = ref('')` 之后加 `const listEl = ref<HTMLElement | null>(null)`；
- `idOf()` 之后加：

```ts
/** 确认文案按类别（原型 pendingDeleteText）：笔记 / 条目 / 待办 / 子任务。 */
const confirmText = computed(() => {
  const id = confirm.pendingId.value
  if (!id) return ''
  if (id.startsWith('note:')) return '确认删除该笔记？'
  if (id.startsWith('clip:')) return '确认删除该条目？'
  const todo = items.value.find((item) => idOf(item) === id)?.todo
  return todo?.parent_id ? '确认删除该子任务？' : '确认删除该待办事项？'
})
```

- `remove()`（第 197–209 行）替换为：

```ts
/** CardConfirm 确认：pendingId 形如 `${kind}:${id}`，从当前条目里找回对象。 */
async function remove(): Promise<void> {
  const id = confirm.confirm()
  const item = id ? items.value.find((entry) => idOf(entry) === id) : undefined
  if (!item) return
  try {
    if (item.kind === 'note' && item.note) await api.notes.remove(item.note.id)
    else if (item.kind === 'clip' && item.clip) await api.clipboard.remove(item.clip.id)
    else if (item.kind === 'todo' && item.todo) await api.todos.remove(item.todo.id)
    await load()
    toast('已删除')
  } catch (error) {
    logger.error('day-view', '删除失败', error)
    toast(String(error))
  }
}
```

- 模板第 239–252 行替换为：

```vue
    <div ref="listEl" v-stagger-list>
      <div
        v-for="item in visible"
        :key="idOf(item)"
        class="day-item"
        :class="[item.kind, { done: item.todo?.status === 'done', confirming: confirm.isPending(idOf(item)) }]"
        :data-kind="item.kind"
        :data-id="idOf(item)"
      >
```

- 列表容器 `</div>`（原第 306 行）之后插入：

```vue
    <CardConfirm
      :text="confirmText"
      :target-id="confirm.pendingId.value"
      :container="listEl"
      fallback="center"
      @confirm="remove"
      @cancel="confirm.cancel()"
    />
```

- [ ] **Step 6: 删除 `ConfirmPopover.vue`，更新 `useConfirmDelete` 头注释**

`git rm src/components/base/ConfirmPopover.vue`。

`src/composables/useConfirmDelete.ts` 第 4–10 行替换为：

```ts
/**
 * 删除二次确认态。
 *
 * 点击卡片右上角 ✕ 不直接删除，而是进入确认态：pendingId 记录待删卡片，
 * 浮层本体是 body 级锚定的 CardConfirm（原型 #cardConfirm，锚到卡片右侧带箭头）。
 * 同一时刻只允许一个待确认项，避免出现多个确认框。
 */
```

- [ ] **Step 7: 样式**

`src/styles/extensions.css`：

（a）§3 末尾（第 102 行 `.nav-dots {…}` 之后、§4 之前）追加：

```css
/* 删除确认态：该待办自身的 ✕ 常显（原型无此态；浮层在 body 级 CardConfirm，卡片只留 .confirming 类）。
   用直接子代限定到本卡片的 todo-body，父待办确认态不会把子任务的 ✕ 也点亮。 */
.todo-item.confirming > .todo-body > .todo-del {
  opacity: 1;
  pointer-events: auto;
}
/* D27：面板 480px 内卡片左右都放不下时，浮层落在卡片下方——箭头改为朝上贴顶缘、水平靠左。
   生成层 .cc-caret 默认在左缘（top: --caret-y），这里覆写为顶缘并把可见边改成上 + 左两条。 */
#cardConfirm.below .cc-caret {
  top: calc(-1 * var(--sp-sm));
  left: var(--sp-xl);
  right: auto;
  border: 1px solid rgba(255, 95, 87, 0.5);
  border-right: none;
  border-bottom: none;
  transform: rotate(45deg);
}
```

（b）§5（第 111–137 行）整段替换为（只剩 Task 3 要删的最后一条）：

```css
/* ═══ 5. 过渡桥接：旧原型规则（阶段四 A Task 3 迁移后整节删除） ═══
 * TodoEditorModal 仍按旧原型居中遮罩弹窗实现；新原型的 #todoEditorOverlay 是透明无遮罩的锚定浮层容器。 */
#todoEditorOverlay {
  background: rgba(8, 10, 18, 0.45);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
}
```

（c）删除 §6 第 424–447 行（`/* ═══ 删除二次确认浮层（悬浮于卡片上方…） ═══ */` 注释、`.card-confirm {…}`、`/* 确认态下该条目删除按钮保持常显 */`、`.todo-item:has(.card-confirm) .todo-del {…}`）。

- [ ] **Step 8: 校验并提交**

```bash
pnpm sync:styles --check && pnpm typecheck && pnpm format:check && grep -rn "ConfirmPopover\|confirm-delete\|cancel-delete" src || echo "无残留引用"
git add -A src/motion/presets.ts src/components/base src/components/card src/composables/useConfirmDelete.ts src/windows/Main/NotesView.vue src/windows/Main/ClipsView.vue src/windows/Main/TodosView.vue src/windows/Main/DayView.vue src/windows/Panel/ClipPage.vue src/styles/extensions.css
git commit -m "feat(ui): 删除二次确认改为 body 级锚定浮层 CardConfirm，删除 ConfirmPopover 与桥接样式"
```

---

## Task 3: 待办编辑浮层 `TodoEditorPanel` 对齐原型，主窗口与面板统一走 `editor` 窗

**Files:**
- Create: `src/constants/todoEditor.ts`
- Create: `src/composables/useEditorWindow.ts`
- Create: `src/components/todo/TodoEditorPanel.vue`
- Delete: `src/components/todo/TodoEditorModal.vue`
- Modify: `src/windows/Editor/EditorApp.vue`
- Modify: `src/windows/editor.ts:1-7`（头注释）
- Modify: `src/windows/Panel/TodoPage.vue`、`src/windows/Main/TodosView.vue`、`src/windows/Main/DayView.vue`
- Modify: `src/styles/extensions.css`（删 §5 整节；删 §6 `#todoEditorModal` / `#todoEditorModal .search-input` / `.te-field-date` / `.te-field-time` / `.te-field-priority`）
- Modify: `src/styles/window-fit.css:91-117`

**Interfaces:**
- Consumes: Task 1 `anchorBeside / Rect`；Task 2 `enter / exit` 的 `duration` 档位、`TodoTree.cardOf(id)`、`TodoCard` 的 `edit-remark`；`api.windows.editorOpen / editorPayload / editorReady / editorClose`（既有，不改 Rust）；`@tauri-apps/api/window` 的 `getCurrentWindow().innerPosition() / scaleFactor()`。
- Produces:
  - `src/constants/todoEditor.ts`：`type TodoEditorMode = 'create' | 'edit' | 'child' | 'due' | 'remind' | 'tags' | 'remark'`；`type TodoEditorSect = 'text' | 'tags' | 'remark' | 'due' | 'remind' | 'prio'`；`TODO_SECTS: Record<TodoEditorMode, readonly TodoEditorSect[]>`；`TODO_EDITOR_TITLES: Record<TodoEditorMode, { icon: string; text: string }>`。
  - `src/composables/useEditorWindow.ts`：`interface EditorAnchor { x: number; y: number; w: number; h: number }`（物理像素）；`interface TodoEditorPayload { kind: 'todo'; mode: TodoEditorMode; todoId?: string | null; parentId?: string | null; presetDate?: string; anchor?: EditorAnchor }`（**删除 `focus`**）；`interface OpenTodoEditorOptions { mode; todoId?; parentId?; presetDate?; anchorEl?: HTMLElement | null }`；`physicalAnchorOf(el: HTMLElement): Promise<EditorAnchor>`；`useEditorWindow(scope: string): { openTodoEditor(opts: OpenTodoEditorOptions): Promise<void> }`。
  - `TodoEditorPanel` props：`mode: TodoEditorMode`、`todo?: Todo | null`、`parent?: Todo | null`、`presetDate?: string`、`anchor?: Rect | null`（editor 窗本地 CSS 像素）；emits `save(input: TodoInput)` / `close`。
  - `TodoTree` emit `edit-remark`（Task 2 已加）由三处调用方接到 `mode: 'remark'`。

- [ ] **Step 1: `src/constants/todoEditor.ts`**

```ts
/**
 * 待办编辑浮层的模式表（原型 app.js `TODO_SECTS` / `openTodoEditor` 的 titles）。
 *
 * 七种模式：create / edit / child 是全字段；due / remind / tags / remark 是聚焦单区段
 * （只显示对应 .te-sect，其余字段取待办原值原样提交）。抽成常量便于 TodoEditorPanel 与调用方共用类型。
 */

export type TodoEditorMode = 'create' | 'edit' | 'child' | 'due' | 'remind' | 'tags' | 'remark'

/** 面板内的六个区段，对应模板 `.te-sect[data-sect]`。 */
export type TodoEditorSect = 'text' | 'tags' | 'remark' | 'due' | 'remind' | 'prio'

const FULL: readonly TodoEditorSect[] = ['text', 'tags', 'remark', 'due', 'remind', 'prio']

/** 各模式显示的区段（原型 TODO_SECTS 逐字）。 */
export const TODO_SECTS: Record<TodoEditorMode, readonly TodoEditorSect[]> = {
  create: FULL,
  edit: FULL,
  child: FULL,
  due: ['due'],
  remind: ['remind'],
  tags: ['tags'],
  remark: ['remark'],
}

/** 标题（原型 titles）：emoji 走 .ix 单独渲染，文字部分在这里。 */
export const TODO_EDITOR_TITLES: Record<TodoEditorMode, { icon: string; text: string }> = {
  create: { icon: '＋', text: '新建待办' },
  child: { icon: '＋', text: '添加子任务' },
  remind: { icon: '⏰', text: '设置 / 更改提醒' },
  due: { icon: '📅', text: '修改完成时间' },
  edit: { icon: '✏️', text: '编辑待办' },
  tags: { icon: '🏷️', text: '编辑标签' },
  remark: { icon: '📄', text: '编辑备注' },
}
```

- [ ] **Step 2: `src/composables/useEditorWindow.ts`**

```ts
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { TodoEditorMode } from '@/constants/todoEditor'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 打开独立 editor 窗口编辑待办（spec D23 / §4.4）。
 *
 * 主窗口与面板统一走这一条路径：editor 窗是铺满工作区的透明置顶窗，浮层在窗内按原型
 * positionTodoEditor 锚定到触发卡片右侧。锚点跨窗口传递必须用**物理屏幕像素**：
 * 调用窗口的 `innerPosition()`（物理）+ 卡片 `getBoundingClientRect()`（CSS 像素）× `scaleFactor()`；
 * EditorApp 再用自己的 innerPosition / scaleFactor 换算回本窗 CSS 像素。两边都走物理像素，
 * 150% 缩放与多屏下各自的 scale 才能对上。
 *
 * payload 是 Rust 透传的 JSON 字符串（`editor_open` 不解析），字段增删不需要改后端。
 */

/** 锚点卡片的屏幕矩形（物理像素）。 */
export interface EditorAnchor {
  x: number
  y: number
  w: number
  h: number
}

/** editor 窗打开参数；与 EditorApp 解析的类型一一对应。 */
export interface TodoEditorPayload {
  kind: 'todo'
  mode: TodoEditorMode
  todoId?: string | null
  parentId?: string | null
  /** 历史日期补录时预填的 YYYY-MM-DD。 */
  presetDate?: string
  /** 缺省 = 无锚点，浮层居中。 */
  anchor?: EditorAnchor
}

export interface OpenTodoEditorOptions {
  mode: TodoEditorMode
  todoId?: string | null
  parentId?: string | null
  presetDate?: string
  /** 锚点元素：卡片（spec D28）或顶部「＋」按钮；null 则居中。 */
  anchorEl?: HTMLElement | null
}

/** 把本窗口内的元素矩形换算为物理屏幕像素。 */
export async function physicalAnchorOf(el: HTMLElement): Promise<EditorAnchor> {
  const rect = el.getBoundingClientRect()
  const win = getCurrentWindow()
  const [position, scale] = await Promise.all([win.innerPosition(), win.scaleFactor()])
  return {
    x: Math.round(position.x + rect.left * scale),
    y: Math.round(position.y + rect.top * scale),
    w: Math.round(rect.width * scale),
    h: Math.round(rect.height * scale),
  }
}

export function useEditorWindow(scope: string): {
  openTodoEditor: (opts: OpenTodoEditorOptions) => Promise<void>
} {
  const { toast } = useToast()

  /**
   * 换算锚点并打开 editor 窗。锚点换算失败（极端情况：元素已卸载）不阻断打开，浮层退化为居中。
   * 面板调用方须在调用前 emit('externalEditor')（编辑窗一拿到焦点面板就会 blur）。
   */
  async function openTodoEditor(opts: OpenTodoEditorOptions): Promise<void> {
    let anchor: EditorAnchor | undefined
    if (opts.anchorEl) {
      try {
        anchor = await physicalAnchorOf(opts.anchorEl)
      } catch (error) {
        logger.warn(scope, '锚点换算失败，编辑浮层将居中', error)
      }
    }
    const payload: TodoEditorPayload = {
      kind: 'todo',
      mode: opts.mode,
      todoId: opts.todoId ?? null,
      parentId: opts.parentId ?? null,
      presetDate: opts.presetDate ?? '',
      anchor,
    }
    logger.info(scope, `打开编辑窗口 mode=${opts.mode}`, payload)
    try {
      await api.windows.editorOpen(JSON.stringify(payload))
    } catch (error) {
      logger.error(scope, '打开编辑窗口失败', error)
      toast(String(error))
    }
  }

  return { openTodoEditor }
}
```

- [ ] **Step 3: 新建 `src/components/todo/TodoEditorPanel.vue`**

```vue
<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, type CSSProperties } from 'vue'
import TagChip from '@/components/tag/TagChip.vue'
import { useShakeConfirm } from '@/composables/useShakeConfirm'
import { useToast } from '@/composables/useToast'
import { DEFAULT_REMIND_OFFSET, REMIND_OPTIONS, remindOffsetLabel } from '@/constants/reminder'
import { TODO_EDITOR_TITLES, TODO_SECTS, type TodoEditorMode, type TodoEditorSect } from '@/constants/todoEditor'
import { enter, exit } from '@/motion'
import { logger } from '@/service/logger'
import type { Priority, Todo, TodoInput } from '@/typings/domain'
import { anchorBeside, type Rect } from '@/utils/anchor'
import { formatDueLabel, fromDateAndTimeInputs, toDateAndTimeInputs, todayKey } from '@/utils/datetime'

/**
 * 待办编辑浮层（原型 #todoEditorOverlay > #todoEditorPanel，spec §4.3）。只在 editor 窗内渲染（EditorApp）。
 *
 * - 七种模式：create / edit / child 全字段；due / remind / tags / remark 只显示对应区段（TODO_SECTS），
 *   其余字段取待办原值原样提交；标题与底栏提示按模式切换；
 * - 定位：挂载后测自身尺寸，anchorBeside(anchor, align 'top', fallback 'center') 锚到卡片右侧（右侧不够翻左 .flip），
 *   箭头 --caret-y 对齐卡片中心；无锚点居中；入场横向 10px 滑入（--dur-base），退场淡出（--dur-fast）后 emit close；
 * - 关闭：✕ / 取消 / 点 overlay 空白 / Esc（自身 keydown capture）；
 * - 校验（需求 2.2）：内容与完成时间必填；create / child 且非历史补录时不早于当前；子任务不晚于未完成父待办；
 *   标签 ≤3 · ≤10 字；备注 ≤200 字；选了提醒偏移必须至少勾一种渠道；已完成事项只允许改备注。
 */
const props = withDefaults(
  defineProps<{
    mode: TodoEditorMode
    /** 编辑既有事项时传入；创建时为空。 */
    todo?: Todo | null
    /** 子任务的父待办（新建子任务 / 编辑子任务），用于完成时间上限校验与提示。 */
    parent?: Todo | null
    /** 归档页在历史日期新增时预填该日期（YYYY-MM-DD）。 */
    presetDate?: string
    /** 锚点卡片矩形（本窗 CSS 像素）；null 居中。 */
    anchor?: Rect | null
  }>(),
  { todo: null, parent: null, presetDate: '', anchor: null },
)

const emit = defineEmits<{
  (e: 'save', input: TodoInput): void
  (e: 'close'): void
}>()

const { toast } = useToast()
const shake = useShakeConfirm()

// ── 区段与文案 ──

const sects = computed<readonly TodoEditorSect[]>(() => TODO_SECTS[props.mode])
function has(sect: TodoEditorSect): boolean {
  return sects.value.includes(sect)
}

const title = computed(() => TODO_EDITOR_TITLES[props.mode])
const isNew = computed(() => props.mode === 'create' || props.mode === 'child')

// ── 表单状态：全部从 todo 原值初始化，聚焦模式下未显示的字段保持原值提交 ──

/** 默认完成时间 = 当前 + 1 小时（需求指定）；历史补录时日期归入所选日。 */
function defaultDue(): { date: string; time: string } {
  const value = toDateAndTimeInputs(new Date(Date.now() + 3_600_000).toISOString())
  return props.presetDate ? { date: props.presetDate, time: value.time } : value
}

const initialDue = props.todo ? toDateAndTimeInputs(props.todo.due_at) : defaultDue()

const content = ref(props.todo?.content ?? '')
const tags = ref<string[]>([...(props.todo?.tags ?? [])])
const tagInput = ref('')
const remark = ref(props.todo?.remark ?? '')
const dueDate = ref(initialDue.date)
const dueTime = ref(initialDue.time)
const remindOffset = ref<number | null>(props.todo ? props.todo.remind_offset_minutes : DEFAULT_REMIND_OFFSET)
const remindDesktop = ref(props.todo ? props.todo.remind_desktop : true)
const remindEmail = ref(props.todo ? props.todo.remind_email : false)
const priority = ref<Priority>((props.todo?.priority as Priority) ?? 'medium')

const contentInput = ref<HTMLInputElement | null>(null)
const tagInputEl = ref<HTMLInputElement | null>(null)
const remarkInput = ref<HTMLTextAreaElement | null>(null)
const dueTimeInput = ref<HTMLInputElement | null>(null)
const remindSelect = ref<HTMLSelectElement | null>(null)

/** 已完成事项只允许编辑备注，约束由前后端共同执行。 */
const completedOnly = computed(() => props.todo?.status === 'done')
const fieldsDisabled = computed(() => completedOnly.value)
/** 选「不提醒」时渠道勾选无意义，禁用并置灰。 */
const channelsDisabled = computed(() => fieldsDisabled.value || remindOffset.value === null)
/** 新建（含子任务）且非历史补录时锁定日期下限为今天。 */
const minDate = computed(() => (isNew.value && !props.presetDate ? todayKey() : ''))
/** 子任务日期上限锁到未完成父待办的完成日（原型 dateEl.max）。 */
const maxDate = computed(() =>
  props.parent && props.parent.status === 'open' ? toDateAndTimeInputs(props.parent.due_at).date : '',
)

/** 底栏提示（原型 todoEditorHint，remind 文案按 D14 偏移口径）。 */
const hint = computed(() => {
  if (completedOnly.value) return '已完成待办仅允许修改备注'
  switch (props.mode) {
    case 'child':
      return `子任务完成时间不能晚于父待办（${props.parent ? formatDueLabel(props.parent.due_at) : '—'}）`
    case 'remind':
      return remindOffset.value === null
        ? '当前不提醒；选择偏移后可勾选桌面弹窗 / 邮箱'
        : `提醒在完成时间${remindOffsetLabel(remindOffset.value)}触发，可选桌面弹窗 / 邮箱`
    case 'due':
      return '完成时间决定列表排序与逾期判定；子任务不能晚于父待办'
    case 'tags':
      return '标签 ≤3 个、每个 ≤10 字；✕ 需再次点击确认删除'
    case 'remark':
      return '备注 ≤200 字'
    case 'create':
      return props.presetDate
        ? `完成时间默认 1 小时后，将归入 ${props.presetDate}${props.presetDate < todayKey() ? '（历史日期补录）' : ''}`
        : '完成时间默认 1 小时后，可修改'
    default:
      return '完成时间、任务内容必填'
  }
})

// ── 定位与入退场 ──

const panelRef = ref<HTMLElement | null>(null)
const left = ref(0)
const top = ref(0)
const caretY = ref(28)
const flip = ref(false)

const panelStyle = computed<CSSProperties>(() => ({
  left: `${left.value}px`,
  top: `${top.value}px`,
  '--caret-y': `${caretY.value}px`,
}))

/** 原型 positionTodoEditor：右侧 → 左翻 → 居中兜底，垂直对齐卡片顶部并防出屏。 */
function position(): void {
  const panel = panelRef.value
  if (!panel) return
  const result = anchorBeside(props.anchor, {
    size: { width: panel.offsetWidth || 340, height: panel.offsetHeight || 300 },
    viewport: { width: window.innerWidth, height: window.innerHeight },
    align: 'top',
    fallback: 'center',
  })
  left.value = result.left
  top.value = result.top
  caretY.value = result.caretY
  flip.value = result.placement === 'left'
  logger.debug('todo-editor', `定位 placement=${result.placement} left=${result.left} top=${result.top}`)
}

/** 按模式把焦点放到对应控件（原型 focusEl 表）。 */
function focusByMode(): void {
  const target: HTMLElement | null =
    props.mode === 'remind'
      ? remindSelect.value
      : props.mode === 'due'
        ? dueTimeInput.value
        : props.mode === 'tags'
          ? tagInputEl.value
          : props.mode === 'remark'
            ? remarkInput.value
            : contentInput.value
  target?.focus()
}

let closing = false

/** 退场淡出（原型 .12s）后再通知调用方关窗；重复触发只走一次。 */
async function close(): Promise<void> {
  if (closing) return
  closing = true
  logger.debug('todo-editor', '关闭浮层')
  if (panelRef.value) await exit(panelRef.value, { axis: 'x', distance: 0, duration: 'fast' })
  emit('close')
}

/** Esc 关闭；capture 保证优先于内部控件（如 select）的键盘处理。 */
function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.stopPropagation()
    void close()
  }
}

onMounted(async () => {
  document.addEventListener('keydown', onKeydown, true)
  const panel = panelRef.value
  if (panel) panel.style.opacity = '0' // 定位前不闪一帧在 (0,0)
  await nextTick()
  position()
  if (panel) void enter(panel, { axis: 'x', distance: flip.value ? 10 : -10, duration: 'base' })
  focusByMode()
})

onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown, true))

// ── 标签 ──

function addTag(): void {
  const name = tagInput.value.trim()
  if (!name) return
  if (tags.value.length >= 3) {
    toast('最多只能添加 3 个标签')
    return
  }
  if (name.length > 10) {
    toast('标签最多 10 个字')
    return
  }
  if (tags.value.includes(name)) {
    toast('该标签已存在')
    return
  }
  tags.value.push(name)
  tagInput.value = ''
}

function removeTag(tag: string): void {
  if (!shake.press(tag)) return
  tags.value = tags.value.filter((t) => t !== tag)
}

// ── 保存 ──

function save(): void {
  if (!content.value.trim()) {
    toast('待办内容不能为空')
    return
  }

  const dueAt = fromDateAndTimeInputs(dueDate.value, dueTime.value)
  if (!dueAt) {
    toast('请填写完整的完成日期与时刻')
    return
  }

  // 新建时完成时间不得早于当前（编辑既有事项不设此下限；历史补录例外）。
  if (isNew.value && !props.presetDate && new Date(dueAt).getTime() < Date.now()) {
    toast('完成时间不能早于当前时刻')
    return
  }

  // 子任务不得晚于父待办（父级已完成时后端会豁免，此处只拦未完成父级）。
  if (props.parent && props.parent.status === 'open') {
    if (new Date(dueAt).getTime() > new Date(props.parent.due_at).getTime()) {
      toast('子任务的完成时间不能晚于父待办')
      return
    }
  }

  // 选了提醒时间却一个渠道都没勾，等于不会提醒，提前拦下避免误以为已生效。
  if (remindOffset.value !== null && !remindDesktop.value && !remindEmail.value) {
    toast('请至少选择一种提醒方式')
    return
  }

  const input: TodoInput = {
    id: props.todo?.id,
    content: content.value.trim(),
    dueAt,
    remindOffsetMinutes: remindOffset.value,
    remindDesktop: remindDesktop.value,
    remindEmail: remindEmail.value,
    repeatRule: props.todo?.repeat_rule ?? null,
    priority: priority.value,
    remark: remark.value,
    tags: [...tags.value],
    parentId: props.parent?.id ?? props.todo?.parent_id ?? null,
    // 补录历史日期时放行后端的「不得早于当前」校验。
    allowPast: Boolean(props.presetDate) || !isNew.value,
  }

  logger.info('todo-editor', `保存待办 mode=${props.mode}`, input)
  emit('save', input)
}
</script>

<template>
  <!-- 透明 overlay 铺满 editor 窗：点空白关闭 -->
  <div id="todoEditorOverlay" @click.self="close">
    <div
      id="todoEditorPanel"
      ref="panelRef"
      class="glass"
      :class="{ flip }"
      :data-mode="props.mode"
      :style="panelStyle"
      role="dialog"
      aria-modal="true"
      :aria-label="title.text"
    >
      <div v-if="props.anchor" class="te-caret" />

      <div class="clip-editor-header">
        <span class="clip-editor-title"><span class="ix">{{ title.icon }}</span> {{ title.text }}</span>
        <button type="button" class="icon-btn" title="关闭（Esc）" @click="close">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
        </button>
      </div>

      <div class="te-body">
        <!-- 任务内容（全字段模式） -->
        <div v-if="has('text')" class="te-sect" data-sect="text">
          <input
            ref="contentInput"
            v-model="content"
            class="search-input"
            placeholder="待办内容…"
            :disabled="fieldsDisabled"
            @keydown.enter.prevent="save"
          />
        </div>

        <!-- 标签（全字段 或 聚焦标签） -->
        <div v-if="has('tags')" class="te-sect" data-sect="tags">
          <div class="te-tags-row">
            <span class="te-label">标签</span>
            <div class="te-tags">
              <TagChip
                v-for="tag in tags"
                :key="tag"
                :label="tag"
                :shaking="shake.isArmed(tag)"
                :deletable="!fieldsDisabled"
                @remove="removeTag(tag)"
              />
              <span v-if="!tags.length" class="te-tags-empty">暂无标签</span>
            </div>
          </div>
          <input
            ref="tagInputEl"
            v-model="tagInput"
            class="search-input te-tag-input"
            placeholder="输入标签回车添加（最多 3 个 · 每个 10 字）"
            maxlength="10"
            :disabled="fieldsDisabled"
            @keydown.enter.prevent="addTag"
          />
        </div>

        <!-- 备注（全字段 或 聚焦备注） -->
        <div v-if="has('remark')" class="te-sect" data-sect="remark">
          <div class="te-remark-wrap">
            <span class="te-label">备注</span>
            <textarea
              id="todoEditorRemark"
              ref="remarkInput"
              v-model="remark"
              maxlength="200"
              rows="2"
              placeholder="补充说明…（选填，最多 200 字）"
            />
            <span class="te-remark-count">{{ remark.length }}/200</span>
          </div>
        </div>

        <!-- 完成时间（全字段 或 聚焦完成时间） -->
        <div v-if="has('due')" class="te-sect" data-sect="due">
          <div class="todo-editor-grid">
            <label class="te-field">
              完成日期
              <input v-model="dueDate" type="date" :min="minDate" :max="maxDate" :disabled="fieldsDisabled" />
            </label>
            <label class="te-field">
              完成时间
              <input ref="dueTimeInput" v-model="dueTime" type="time" :disabled="fieldsDisabled" />
            </label>
          </div>
        </div>

        <!-- 提醒（全字段 或 聚焦提醒）：D14 偏移下拉 + 渠道勾选 -->
        <div v-if="has('remind')" class="te-sect" data-sect="remind">
          <div class="todo-editor-grid">
            <label class="te-field">
              提醒时间
              <select ref="remindSelect" v-model="remindOffset" :disabled="fieldsDisabled">
                <option v-for="option in REMIND_OPTIONS" :key="String(option.value)" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </label>
            <div class="te-field te-remind-channels">
              提醒方式
              <div class="te-channel-row">
                <label class="te-channel">
                  <input v-model="remindDesktop" type="checkbox" :disabled="channelsDisabled" />
                  桌面弹窗
                </label>
                <label class="te-channel">
                  <input v-model="remindEmail" type="checkbox" :disabled="channelsDisabled" />
                  邮箱
                </label>
              </div>
            </div>
          </div>
        </div>

        <!-- 优先级（仅全字段） -->
        <div v-if="has('prio')" class="te-sect" data-sect="prio">
          <div class="todo-editor-grid">
            <label class="te-field">
              优先级
              <select v-model="priority" :disabled="fieldsDisabled">
                <option value="high">🔴 高</option>
                <option value="medium">🟡 中</option>
                <option value="low">🟢 低</option>
              </select>
            </label>
          </div>
        </div>
      </div>

      <div class="clip-editor-footer">
        <span class="clip-editor-hint">{{ hint }}</span>
        <div class="clip-editor-actions">
          <button type="button" class="btn ghost" @click="close">取消</button>
          <button type="button" class="btn primary" @click="save">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 4: `EditorApp.vue` 换渲染 `TodoEditorPanel`，锚点换算回本窗**

（a）第 2–10 行 import 替换为：

```ts
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import ToastHost from '@/components/base/ToastHost.vue'
import TodoEditorPanel from '@/components/todo/TodoEditorPanel.vue'
import { useSettings, useTodos } from '@/composables/useData'
import type { EditorAnchor, TodoEditorPayload } from '@/composables/useEditorWindow'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Todo, TodoInput } from '@/typings/domain'
import type { Rect } from '@/utils/anchor'
```

（b）头注释第 13–16 行改为：

```
 * 独立编辑窗口：铺满工作区的透明置顶窗，内含锚定卡片右侧的待办编辑浮层（spec D23）。
 *
 * 面板只有 480px 宽、主窗口内锚定会被窗口边界裁切，因此主窗口与面板都走这里；
 * payload 带锚点卡片的**物理屏幕像素**矩形，本窗按自己的 innerPosition / scaleFactor 换算回 CSS 像素后
 * 交给 TodoEditorPanel 按原型 positionTodoEditor 定位（右侧 → 左翻 → 居中兜底）。
```

（c）第 34–42 行 `EditorPayload` 类型删除（改用 `TodoEditorPayload`）；第 49–53 行替换为：

```ts
/** 本次打开参数，挂载后由后端拉取填入。 */
const payload = ref<TodoEditorPayload | null>(null)
/** 锚点换算结果（本窗 CSS 像素）；null = 无锚点居中。 */
const anchorRect = ref<Rect | null>(null)
/** 锚点是否已换算完成（无锚点时立即为 true）；未完成前不渲染，避免浮层先居中再跳到卡片旁。 */
const anchorResolved = ref(false)

/** 是否已请求显示窗口，避免 ready 反复变化时重复 invoke。 */
const shown = ref(false)
```

（d）`parent` 计算（第 75–80 行）替换为（编辑子任务时也带上父级，让「子任务 ≤ 父」校验在 edit / due 模式同样生效）：

```ts
/** 父待办：新建子任务取 payload.parentId；编辑既有子任务取 todo.parent_id（完成时间上限校验用）。 */
const parentId = computed<string | null>(() => payload.value?.parentId ?? todo.value?.parent_id ?? null)
const parent = computed<Todo | null>(() => {
  const id = parentId.value
  if (!id) return null
  return todos.value.find((item) => item.id === id) ?? null
})
```

`ready`（第 82–89 行）替换为：

```ts
/** 参数、锚点与目标事项都就绪才渲染，避免闪出一张空表单或位置跳变。 */
const ready = computed(() => {
  const value = payload.value
  if (!value || !anchorResolved.value) return false
  if (value.todoId && !todo.value) return false
  if (parentId.value && !parent.value) return false
  return true
})
```

（e）`saveTodo` 之前插入：

```ts
/** 物理屏幕像素 → 本窗 CSS 像素（与 useEditorWindow.physicalAnchorOf 互逆）。 */
async function resolveAnchor(anchor: EditorAnchor | undefined): Promise<void> {
  if (!anchor) {
    anchorResolved.value = true
    return
  }
  try {
    const win = getCurrentWindow()
    const [position, scale] = await Promise.all([win.innerPosition(), win.scaleFactor()])
    anchorRect.value = {
      left: (anchor.x - position.x) / scale,
      top: (anchor.y - position.y) / scale,
      width: anchor.w / scale,
      height: anchor.h / scale,
    }
    logger.info('editor', '锚点换算完成', anchorRect.value)
  } catch (error) {
    logger.warn('editor', '锚点换算失败，浮层居中', error)
  }
  anchorResolved.value = true
}
```

（f）参数拉取（第 128–139 行）替换为：

```ts
void api.windows
  .editorPayload()
  .then((raw) => {
    if (!raw) throw new Error('后端未提供打开参数')
    payload.value = JSON.parse(raw) as TodoEditorPayload
    logger.info('editor', '打开参数', payload.value)
    return resolveAnchor(payload.value.anchor)
  })
  .catch((error) => {
    // 参数缺失或损坏时不能留一个吞掉整屏点击的透明窗口，直接自毁。
    logger.error('editor', '获取打开参数失败，关闭窗口', error)
    void close()
  })
```

（g）模板替换为：

```vue
<template>
  <TodoEditorPanel
    v-if="ready && payload?.kind === 'todo'"
    :mode="payload.mode"
    :todo="todo"
    :parent="parent"
    :preset-date="payload.presetDate ?? ''"
    :anchor="anchorRect"
    @save="saveTodo"
    @close="close"
  />

  <ToastHost />
</template>
```

（h）`src/windows/editor.ts` 第 1–7 行头注释改为：

```ts
/**
 * 独立编辑窗口入口。
 *
 * 铺满显示器工作区的透明置顶窗；待办编辑浮层在窗内锚定到触发卡片右侧（spec D23），
 * 面板（480px）与主窗口都借这个窗口摆浮层，不被各自的窗口边界裁切。
 */
```

- [ ] **Step 5: `TodoPage.vue` 改调 `useEditorWindow`（七种 mode）**

第 3 行 `import TodoTree …` 之后加两行：`import { useEditorWindow } from '@/composables/useEditorWindow'` 与 `import type { TodoEditorMode } from '@/constants/todoEditor'`（其余 import 不动）。

头注释第 20–22 行改为：

```
 * 新增 / 编辑走**独立编辑窗口**（editor，spec D23）：面板只有 480px 宽，锚定浮层会被窗口边界裁切，
 * 因此由后端打开铺满工作区的透明窗，浮层在窗内锚定到卡片右侧（锚点经 useEditorWindow 换算为物理像素）；
 * 保存后由 todosChanged 事件驱动刷新。
```

第 29 行 `const { toast } = useToast()` 之后加 `const { openTodoEditor } = useEditorWindow('panel-todo')`。

`openEditor`（第 87–113 行）替换为：

```ts
/**
 * 打开独立编辑窗口。只传 ID 不传整个对象：编辑窗口按 ID 从自己那份最新列表中取，
 * 避免面板持有的旧快照覆盖掉别处刚改过的字段。锚点是卡片（spec D28）或顶部按钮。
 */
function openEditor(
  mode: TodoEditorMode,
  todo: Todo | null = null,
  parent: Todo | null = null,
  anchorEl: HTMLElement | null = null,
): void {
  // 先上报再 invoke：编辑窗口一拿到焦点面板就会 blur，晚于 blur 上报会来不及阻止收起。
  emit('externalEditor')
  void openTodoEditor({ mode, todoId: todo?.id ?? null, parentId: parent?.id ?? null, anchorEl })
}

/** 卡片级锚点：从待办树反查 [data-id]。 */
function cardOf(todo: Todo): HTMLElement | null {
  return tree.value?.cardOf(todo.id) ?? null
}
```

模板：第 175 行「📅」按钮的 `@click` 改为 `@click="openEditor('create', null, null, $event.currentTarget as HTMLElement)"`；`<TodoTree …>` 的事件（第 189–194 行）替换为：

```vue
      @edit="openEditor('edit', $event, null, cardOf($event))"
      @edit-due="openEditor('due', $event, null, cardOf($event))"
      @edit-remind="openEditor('remind', $event, null, cardOf($event))"
      @edit-remark="openEditor('remark', $event, null, cardOf($event))"
      @edit-repeat="(todo) => openEditor('remind', todo, null, cardOf(todo))"
      @add-sub="openEditor('child', null, $event, cardOf($event))"
      @open-tags="openEditor('tags', $event, null, cardOf($event))"
```

（`@edit-repeat` 在 Task 4 换成 RepeatMenu。）

- [ ] **Step 6: `TodosView.vue` 删弹窗、改调 `useEditorWindow`**

- 删除第 4 行 `import TodoEditorModal …`；第 3 行之后加 `import { useEditorWindow } from '@/composables/useEditorWindow'`、`import type { TodoEditorMode } from '@/constants/todoEditor'`；第 9 行改为 `import type { Priority, Todo } from '@/typings/domain'`（不再用 `TodoInput`）；
- 头注释末尾加一行 `* - 新增 / 编辑走 editor 窗内的锚定浮层（spec D23）：锚点为卡片或顶部「＋」按钮，保存后经 todosChanged 刷新。`；
- 第 24 行之后加 `const { openTodoEditor } = useEditorWindow('todos-view')`；
- 删除第 29–34 行 `editor` 状态；第 27 行 `keyword` 之后加 `const tree = ref<InstanceType<typeof TodoTree> | null>(null)`；
- `openEditor`（第 81–88 行）替换为：

```ts
/** 打开 editor 窗内的锚定浮层；历史日期视图下新增走补录（presetDate）。 */
function openEditor(
  mode: TodoEditorMode,
  todo: Todo | null = null,
  parent: Todo | null = null,
  anchorEl: HTMLElement | null = null,
): void {
  void openTodoEditor({
    mode,
    todoId: todo?.id ?? null,
    parentId: parent?.id ?? null,
    presetDate: mode === 'create' && !isToday.value ? currentDate.value : '',
    anchorEl,
  })
}

/** 卡片级锚点：从待办树反查 [data-id]。 */
function cardOf(todo: Todo): HTMLElement | null {
  return tree.value?.cardOf(todo.id) ?? null
}
```

- 删除 `saveTodo`（第 98–108 行）；
- 模板：第 166 行「＋」按钮 `@click="openEditor('create')"` 改为 `@click="openEditor('create', null, null, $event.currentTarget as HTMLElement)"`；`<TodoTree` 加 `ref="tree"`，事件（第 182–186 行）替换为：

```vue
      @edit="openEditor('edit', $event, null, cardOf($event))"
      @edit-due="openEditor('due', $event, null, cardOf($event))"
      @edit-remind="openEditor('remind', $event, null, cardOf($event))"
      @edit-remark="openEditor('remark', $event, null, cardOf($event))"
      @add-sub="openEditor('child', null, $event, cardOf($event))"
      @open-tags="openEditor('tags', $event, null, cardOf($event))"
```

- 删除模板第 195–204 行 `<TodoEditorModal …/>`。

- [ ] **Step 7: `DayView.vue` 删弹窗、改调 `useEditorWindow`、监听 `todosChanged`**

- 删除第 10 行 `import TodoEditorModal …`；第 11 行之后加 `import { useEditorWindow } from '@/composables/useEditorWindow'`；第 18 行改为 `import type { ClipboardEntry, DayDetailItem, Priority, Todo } from '@/typings/domain'`；
- 头注释第 30–31 行「粘贴板弹窗、待办弹窗（已完成待办拦截）」改为「粘贴板弹窗、待办走 editor 窗锚定浮层（spec D23；已完成待办拦截）」；
- 第 37 行之后加 `const { openTodoEditor } = useEditorWindow('day-view')`；
- 第 122–124 行之后加：

```ts
// 待办在 editor 窗内保存 / 别处完成或删除后，日期详情同样要回流刷新。
void onAppEvent(AppEvents.todosChanged, () => void load())
```

- 删除第 135 行 `const editTodo = ref<Todo | null>(null)`；`edit()` 里第 165 行 `editTodo.value = item.todo` 改为：

```ts
    void openTodoEditor({
      mode: 'edit',
      todoId: item.todo.id,
      anchorEl: listEl.value?.querySelector<HTMLElement>(`[data-id="${CSS.escape(idOf(item))}"]`) ?? null,
    })
```

- 删除 `saveTodo`（第 184–195 行）；删除模板第 310 行 `<TodoEditorModal …/>`。

- [ ] **Step 8: 删除 `TodoEditorModal.vue`；样式**

`git rm src/components/todo/TodoEditorModal.vue`。

`src/styles/extensions.css`：
- 删除 §5 整节（Task 2 缩减后的 `/* ═══ 5. 过渡桥接 … ═══ */` 注释 + `#todoEditorOverlay {…}`）；
- 删除 §6 中 `#todoEditorModal {…}`、`#todoEditorModal .search-input {…}`、`.te-field-date {…}`、`.te-field-time {…}`、`.te-field-priority {…}` 五个规则块（原第 530–547 行；`.te-remind-channels` / `.te-channel*` 保留，新浮层的提醒区段仍用）。

`src/styles/window-fit.css`：
- 第 91–95 行注释改为：

```css
/* ═══ 独立编辑窗口：铺满工作区的透明窗 + 锚定浮层 ═══
 * 窗口以 transparent 创建，若不把 body 背景清成透明，浮层之外会是一整块实色。
 * #todoEditorOverlay 沿用生成层（fixed + inset: 0 + 透明底），在这个窗口里正好铺满整窗，点空白关窗。 */
```

- 第 105–107 行替换为：

```css
:root[data-window='editor'] .modal-overlay,
:root[data-window='editor'] #todoEditorOverlay {
  pointer-events: auto;
}
```

- 第 108–117 行替换为（去掉 `#todoEditorModal`，其余两个保留供将来）：

```css
/* 屏幕比面板宽得多，对话框可以给到更舒展的尺寸与留白。
   选择器必须带上 ID：弹窗宽度原本挂在 ID 上（优先级 100），
   只写 `:root[data-window] .modal-shell` 会被它压过去，width 不生效。 */
:root[data-window='editor'] #clipEditorModal.modal-shell,
:root[data-window='editor'] #tagManagerModal.modal-shell {
  width: 520px;
  max-height: 86vh;
  padding: 16px 20px 18px;
}
```

- [ ] **Step 9: 校验并提交**

```bash
pnpm sync:styles --check && pnpm typecheck && pnpm format:check && grep -rn "TodoEditorModal\|focus: 'content'\|te-field-date\|todoEditorModal" src || echo "无残留引用"
pnpm exec vite build 2>&1 | grep -E "built in|error"
git add -A src/constants/todoEditor.ts src/composables/useEditorWindow.ts src/components/todo src/windows/Editor/EditorApp.vue src/windows/editor.ts src/windows/Panel/TodoPage.vue src/windows/Main/TodosView.vue src/windows/Main/DayView.vue src/styles/extensions.css src/styles/window-fit.css
git commit -m "feat(todo): 待办编辑浮层对齐原型（锚定卡片右侧 / 七种模式 / 标题与提示），主窗口与面板统一走 editor 窗"
```

---

## Task 4: 重复提醒菜单 `RepeatMenu` 并接入三处待办列表

**Files:**
- Create: `src/components/todo/RepeatMenu.vue`
- Modify: `src/windows/Main/TodosView.vue`、`src/windows/Main/DayView.vue`、`src/windows/Panel/TodoPage.vue`

**Interfaces:**
- Consumes: `useAnchoredMenu(optionCount)`（既有：`visible / activeIndex / menuRef / style / open / close / onKeydown`）；`enter()` 的 `duration` 档位（Task 2）；`api.todos.reminder(id, offsetMinutes, desktop, email, repeatRule)`（既有签名 `src/service/tauri.ts:133-134`）；`TodoTree` emit `edit-repeat(todo, anchor)`（既有）。
- Produces: `RepeatMenu` expose `open(anchor: HTMLElement, current: string | null): Promise<void>`、`close(): void`；emit `pick(rule: string | null)`（`null` = 不重复，`'daily'` / `'weekly'`）。

- [ ] **Step 1: 新建 `src/components/todo/RepeatMenu.vue`**

```vue
<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useAnchoredMenu } from '@/composables/useAnchoredMenu'
import { enter } from '@/motion'

/**
 * 重复提醒下拉菜单（原型 #repeatMenu / showRepeatMenu，spec §4.5）。
 *
 * 三项：🚫 不重复 / 🔁 每天重复 / 🔁 每周重复；当前规则带 .active；锚在 🔁 徽章下方 +6、左缘对齐、
 * 右缘不出视口（useAnchoredMenu 负责定位 / 翻转 / 点外关闭 / Esc / 键盘导航 / 焦点归还）；
 * 入场 y −6 → 0 淡入（--dur-base）。选中后 emit pick，由调用方写库并 toast。
 */
const emit = defineEmits<{ (e: 'pick', rule: string | null): void }>()

/** 顺序与原型一致；value '' 表示不重复（提交时转 null）。 */
const OPTIONS: readonly { value: '' | 'daily' | 'weekly'; label: string }[] = [
  { value: '', label: '🚫 不重复' },
  { value: 'daily', label: '🔁 每天重复' },
  { value: 'weekly', label: '🔁 每周重复' },
]

const { visible, activeIndex, menuRef, style, open: openMenu, close, onKeydown } = useAnchoredMenu(() => OPTIONS.length)

/** 当前规则：打开时传入，渲染 .active。 */
const current = ref<string>('')

function choose(index: number): void {
  const option = OPTIONS[index]
  if (!option) return
  close()
  emit('pick', option.value === '' ? null : option.value)
}

/** 由父组件调用：以 🔁 徽章为锚点打开，光标预置在当前规则上。 */
async function open(anchor: HTMLElement, rule: string | null): Promise<void> {
  current.value = rule ?? ''
  const index = OPTIONS.findIndex((option) => option.value === current.value)
  await openMenu(anchor, index < 0 ? 0 : index)
  // 定位完成后再播入场（原型 y −6 → 0，.18s）；菜单带 backdrop-filter，只在瞬时动效里加 transform。
  await nextTick()
  if (menuRef.value) void enter(menuRef.value, { axis: 'y', distance: -6, duration: 'base' })
}

defineExpose({ open, close })
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      ref="menuRef"
      class="repeat-menu"
      role="listbox"
      tabindex="-1"
      aria-label="选择重复提醒"
      :style="style"
      @keydown="onKeydown($event, choose)"
    >
      <div
        v-for="(option, index) in OPTIONS"
        :key="option.value"
        class="repeat-opt"
        :class="{ active: option.value === current }"
        :data-repeat="option.value"
        role="option"
        :aria-selected="option.value === current"
        @click="choose(index)"
        @mouseenter="activeIndex = index"
      >
        {{ option.label }}
      </div>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 2: `TodoPage.vue` 接线**

- 第 3 行之后加 `import RepeatMenu from '@/components/todo/RepeatMenu.vue'`；
- `const tree = …` 之后加：

```ts
const repeatMenu = ref<InstanceType<typeof RepeatMenu> | null>(null)
/** 正在改重复规则的待办；菜单选中后据此写库。 */
let repeatTarget: Todo | null = null
```

- `removeTodo` 之后加：

```ts
/** 🔁 徽章 → 重复菜单（原型 showRepeatMenu）。 */
async function openRepeat(todo: Todo, anchor: HTMLElement): Promise<void> {
  if (guardDone(todo, '修改重复提醒')) return
  repeatTarget = todo
  await repeatMenu.value?.open(anchor, todo.repeat_rule)
}

/** 菜单选中：只改 repeat_rule，其余提醒字段原样回写。 */
async function applyRepeat(rule: string | null): Promise<void> {
  const todo = repeatTarget
  repeatTarget = null
  if (!todo || (todo.repeat_rule ?? null) === rule) return
  logger.info('panel-todo', `设置重复提醒 id=${todo.id} rule=${rule ?? '(none)'}`)
  try {
    await api.todos.reminder(todo.id, todo.remind_offset_minutes, todo.remind_desktop, todo.remind_email, rule)
    toast(`已设为${rule === 'daily' ? '每天重复' : rule === 'weekly' ? '每周重复' : '不重复'}`)
  } catch (error) {
    logger.error('panel-todo', '设置重复提醒失败', error)
    toast(String(error))
  }
}
```

- 模板：`<TodoTree` 的 `@edit-repeat="(todo) => openEditor('remind', …)"` 改为 `@edit-repeat="openRepeat"`；`</TodoTree>` / `<TodoTree …/>` 之后加 `<RepeatMenu ref="repeatMenu" @pick="applyRepeat" />`。

- [ ] **Step 3: `TodosView.vue` 接线**

同 Step 2：加 import、`repeatMenu` ref 与 `repeatTarget`、`openRepeat` / `applyRepeat`（scope 改 `'todos-view'`）；`<TodoTree` 加 `@edit-repeat="openRepeat"`（此前主窗口未接该事件）；`<TodoTree …/>` 之后加 `<RepeatMenu ref="repeatMenu" @pick="applyRepeat" />`。

- [ ] **Step 4: `DayView.vue` 接线**

- 第 9 行之后加 `import RepeatMenu from '@/components/todo/RepeatMenu.vue'`；
- `const listEl = …` 之后加 `repeatMenu` ref 与 `repeatTarget`（同 Step 2）；
- `remove()` 之后加 `openRepeat` / `applyRepeat`（scope `'day-view'`；`applyRepeat` 成功分支在 toast 之前加 `await load()`，日期详情不走 `useTodos` 的列表）；DayView 没有 `guardDone`，`openRepeat` 里改为 `if (todo.status === 'done') { toast('已完成的待办不允许修改'); return }`；
- 模板：`.day-meta` 里的 🔁 span（第 284–286 行）替换为：

```vue
            <span
              v-if="repeatLabel(item.todo)"
              class="todo-meta repeat"
              title="重复提醒（点击切换 / 结束）"
              @click.stop="openRepeat(item.todo, $event.currentTarget as HTMLElement)"
              ><span class="ix">🔁</span> {{ repeatLabel(item.todo) }}</span
            >
```

- 列表容器之后（`<CardConfirm …/>` 旁）加 `<RepeatMenu ref="repeatMenu" @pick="applyRepeat" />`。

- [ ] **Step 5: 校验并提交**

```bash
pnpm typecheck && pnpm format:check
git add src/components/todo/RepeatMenu.vue src/windows/Main/TodosView.vue src/windows/Main/DayView.vue src/windows/Panel/TodoPage.vue
git commit -m "feat(todo): 重复提醒菜单 RepeatMenu 并接入三处待办列表"
```

---

## Task 5: 弹窗缩放动效、粘贴板编辑光标与空内容拦截、toast 参数

**Files:**
- Modify: `src/motion/presets.ts`（新增 `popOut`）、`src/motion/index.ts:5`
- Modify: `src/components/base/ModalShell.vue`
- Modify: `src/components/clip/ClipEditorModal.vue`
- Modify: `src/composables/useToast.ts:4-5`
- Modify: `src/components/base/ToastHost.vue:28-32`

**Interfaces:**
- Produces: `popOut(el: HTMLElement): Promise<boolean>`（scale 1 → .96 + 淡出，`--dur-fast`，inQuad，播完保留透明终态；原型 closeClipEditor / closeTagManager）。
- `ModalShell` 对外契约不变（props / `close` emit）；`ClipEditorModal` 对外契约不变。

- [ ] **Step 1: `presets.ts` 新增 `popOut`，`index.ts` 导出**

`popIn()` 之后插入：

```ts
/** 弹窗退场：scale 1 → .96 + 淡出，--dur-fast，inQuad（原型 closeClipEditor / closeTagManager）；播完保留透明终态。 */
export function popOut(el: HTMLElement): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) {
    cancelRunning(el)
    el.style.opacity = '0'
    return Promise.resolve(true)
  }
  logger.debug('motion', 'popOut scale 1 → .96')
  return run(el, [el], { scale: [1, 0.96], opacity: [1, 0], duration: tokens.fast, ease: 'inQuad' }, true)
}
```

`src/motion/index.ts` 第 5 行改为 `export { enter, exit, staggerIn, pop, popIn, popOut, crossfade, type Axis, type SlideOptions } from './presets'`。

- [ ] **Step 2: `ModalShell.vue` 入退场**

整体替换为：

```vue
<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { popIn, popOut } from '@/motion'
import { logger } from '@/service/logger'

/**
 * 弹窗外壳：遮罩 + 毛玻璃面板 + 标题栏 + Esc 关闭 + 缩放入退场。
 *
 * 样式沿用原型：遮罩与面板的尺寸/层级挂在 ID 选择器上
 * （#tagManagerOverlay / #clipEditorOverlay），因此调用方需通过 overlayId / modalId 传入对应 ID。
 *
 * 整体 Teleport 到 body：遮罩是 position: fixed，若留在调用方子树内，
 * 会被带 backdrop-filter 的祖先（如面板的 .glass）当作包含块，
 * 只能覆盖祖先自身的盒子，弹窗随之被裁切。挂到 body 后始终以窗口为基准。
 * 面板窗口会另行把 .modal-overlay / .modal-shell 覆写为整页编辑器（见 window-fit.css）。
 *
 * 动效（原型 openTagManager / closeTagManager）：入场 scale .94 → 1（--dur-base），
 * 退场 scale → .96 淡出（--dur-fast）后才 emit close；.glass 只在这两段瞬时动效里带 transform。
 */
const props = defineProps<{
  /** 遮罩元素 ID，决定层级与遮罩样式。 */
  overlayId: string
  /** 面板元素 ID，决定弹窗宽度与内边距。 */
  modalId: string
  title: string
}>()

const emit = defineEmits<{ (e: 'close'): void }>()

const shell = ref<HTMLElement | null>(null)
let closing = false

/** 退场动效播完再通知调用方卸载；重复触发（✕ + Esc）只走一次。 */
async function close(): Promise<void> {
  if (closing) return
  closing = true
  logger.debug('modal-shell', `关闭 ${props.modalId}`)
  if (shell.value) await popOut(shell.value)
  emit('close')
}

/** Esc 关闭。用 capture 保证优先于内部控件的键盘处理。 */
function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.stopPropagation()
    void close()
  }
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown, true)
  if (shell.value) void popIn(shell.value)
})
onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown, true))
</script>

<template>
  <Teleport to="body">
    <!-- 点击遮罩空白处关闭；点击面板内部不冒泡到遮罩。 -->
    <div :id="props.overlayId" class="modal-overlay" @click.self="close">
      <div :id="props.modalId" ref="shell" class="glass modal-shell" role="dialog" aria-modal="true" :aria-label="title">
        <div class="clip-editor-header">
          <span class="clip-editor-title">{{ title }}</span>
          <button type="button" class="icon-btn" title="关闭（Esc）" @click="close">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
          </button>
        </div>

        <div class="modal-body">
          <slot />
        </div>

        <div v-if="$slots.footer" class="clip-editor-footer">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
```

（调用方 `TagManagerModal` / `ClipEditorModal` 的取消按钮直接 `emit('close')`，不经过退场动效；保持不变——它们卸载时 ModalShell 随之消失，与原型「取消」按钮走 close 动效略有差异，属可接受范围，验收记录里注明。）

- [ ] **Step 3: `ClipEditorModal.vue` 光标置末、空内容拦截**

整体替换为：

```vue
<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import ModalShell from '@/components/base/ModalShell.vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'

/**
 * 剪贴板内容编辑浮框（原型 openClipEditor / saveClipEdit）。
 *
 * 需求 2.2：编辑与置顶是两个独立功能——编辑弹框回显原文，保存后替换原内容、
 * 时间更新为最后修改时间，且**不影响置顶状态**；仅文本类条目可编辑。
 * 打开时聚焦且光标置末；清空后保存被拦截（toast「内容为空，未保存」）；支持 ⌃/⌘+Enter 快捷保存。
 */
const props = defineProps<{ content: string }>()

const emit = defineEmits<{
  (e: 'save', content: string): void
  (e: 'close'): void
}>()

const { toast } = useToast()
const draft = ref(props.content)
const textarea = ref<HTMLTextAreaElement | null>(null)

onMounted(() => {
  void nextTick(() => {
    const el = textarea.value
    if (!el) return
    el.focus()
    el.setSelectionRange(el.value.length, el.value.length)
  })
})

function save(): void {
  if (!draft.value.trim()) {
    logger.debug('clip-editor', '内容为空，拦截保存')
    toast('内容为空，未保存')
    return
  }
  logger.info('clip-editor', `保存剪贴板内容，长度 ${draft.value.length}`)
  emit('save', draft.value)
}
</script>

<template>
  <ModalShell
    overlay-id="clipEditorOverlay"
    modal-id="clipEditorModal"
    title="✏️ 编辑剪贴板内容"
    @close="emit('close')"
  >
    <textarea
      id="clipEditorTextarea"
      ref="textarea"
      v-model="draft"
      placeholder="编辑内容…"
      spellcheck="false"
      @keydown.ctrl.enter.prevent="save"
      @keydown.meta.enter.prevent="save"
    />

    <template #footer>
      <span class="clip-editor-hint"> 保存后替换原内容，时间更新为最后修改时间（⌃/⌘+Enter 快捷保存） </span>
      <div class="clip-editor-actions">
        <button type="button" class="btn ghost" @click="emit('close')">取消</button>
        <button type="button" class="btn primary" @click="save">保存修改</button>
      </div>
    </template>
  </ModalShell>
</template>
```

- [ ] **Step 4: toast 参数**

`src/composables/useToast.ts` 第 4–5 行改为：

```ts
/** 单条 toast 的默认展示时长（原型 showToast 的 1800ms）。 */
const DEFAULT_DURATION_MS = 1800
```

`src/components/base/ToastHost.vue` 第 20–21 行注释改为 `/* 上浮 20px + 淡入淡出（原型 showToast 的 y: 20 → 0）：原型用 GSAP 驱动，此处用 Vue 过渡实现等效观感。\n   时长与曲线走 tokens.css 的动效令牌，reduced-motion 下自动归零。 */`；第 31 行 `transform: translate(-50%, 6px);` 改为 `transform: translate(-50%, 20px);`。

- [ ] **Step 5: 校验并提交**

```bash
pnpm typecheck && pnpm format:check && pnpm exec vite build 2>&1 | grep -E "built in|error"
git add src/motion/presets.ts src/motion/index.ts src/components/base/ModalShell.vue src/components/clip/ClipEditorModal.vue src/composables/useToast.ts src/components/base/ToastHost.vue
git commit -m "feat(ui): 弹窗缩放动效、粘贴板编辑光标与空内容拦截、toast 参数对齐原型"
```

---

## Task 6: 实机验收与记录

**Files:**
- Create: `docs/superpowers/plans/2026-09-12-prototype-realign-phase4a-acceptance.md`
- Modify: spec §9（`docs/superpowers/specs/2026-09-12-prototype-realign-phase4a-overlays-design.md:154-158`）；必要时 `src/styles/extensions.css`（新开「§10 阶段四 A 验收补丁」分节注明原因）

- [ ] **Step 1: 静态检查全绿**

Run: `pnpm sync:styles --check && pnpm typecheck && pnpm test && pnpm format:check && cargo check --manifest-path src-tauri/Cargo.toml && pnpm exec vite build`
Expected: 全部通过（Rust 零改动，`cargo check` 只为确认没被误改）；记录 `test:unit` 通过数（含新增 `anchorBeside` 6 例）与 `editor-*.js` / `panel-*.js` 体积。
Run: `grep -rn "ConfirmPopover\|TodoEditorModal\|confirm-delete\|todoEditorModal\|te-field-date" src || echo "无残留"`
Expected: `无残留`。

- [ ] **Step 2: 实机（`pnpm tauri:dev` 后台，沿用 `.tmp/win.ps1` 驱动；打字机 + 深色各一轮）按 spec §6 六项逐条验证并截图**

1. 删除确认：主窗口笔记 / 粘贴板 / 待办 / 日期详情、面板粘贴板 / 待办，各点 ✕ → `#cardConfirm` 出现在卡片右侧带左缘箭头、箭头对齐卡片中心；靠右卡片翻到左侧（`.flip`）；面板内落在卡片下方箭头朝上、左缘对齐（`.below`）；列表**首张**卡片的浮层完整可见（阶段三遗留裁切已修）；滚动列表 → 浮层消失；面板内 Esc 只关浮层、再 Esc 收面板；确认删除生效并 toast；四种文案（该笔记 / 该条目 / 该待办事项 / 该子任务）；待办确认态下 ✕ 常显且父待办确认不点亮子任务的 ✕。
2. 待办编辑：主窗口待办页 ✏️ / 📅 / ⏰ / 标签 / 备注（图标与文本行两种形态）/ ＋子任务 / 顶部 ＋，以及日期详情页 ✏️ → editor 窗内浮层锚定卡片右侧带箭头（靠右卡片左翻；150% 缩放下位置与卡片对齐）、标题与提示与 spec §4.3 表一致、聚焦模式只显示单区段且焦点在对应控件；点空白 / Esc / ✕ / 取消关闭（淡出后关窗）；保存后主窗口列表与日期详情刷新；面板同路径（📅 新增锚到按钮；编辑期间面板不收起）；编辑子任务时完成时间上限锁父待办；历史日期补录提示带「（历史日期补录）」。
3. 重复菜单：有重复规则的待办（主窗口待办页 / 日期详情 / 面板）点 🔁 → 三项菜单锚在徽章下方 +6、当前项 `.active`、y −6 滑入；选择后 toast「已设为…」并刷新徽章（选「不重复」后徽章消失）；点外 / Esc 关闭。
4. 标签管理 / 粘贴板编辑：打开 scale .94 → 1、关闭（✕ / Esc / 点遮罩）scale → .96 淡出；粘贴板编辑打开时光标在文本末尾；清空后保存 → toast「内容为空，未保存」且弹窗不关；面板内整页化观感不倒退。
5. Toast：上浮 20px、1.8 秒后消失（对比阶段三 6px / 2 秒）。
6. 回归：面板 Esc 链（删除确认 → 收面板）、切页清确认、Zen 中 Esc 只退 Zen、面板失焦收起在编辑窗打开期间暂停、主窗口切视图后确认浮层不残留（`CardConfirm` 随视图卸载）。

- [ ] **Step 3: 写验收记录并回填 spec §9**

验收记录按阶段三 `2026-09-11-prototype-realign-phase3-acceptance.md` 的结构：环境 / 方法 → 自动化校验表 → 逐项比对表（打字机、深色各一列）→ 发现的问题与补丁清单 → 未验证项。spec §9 三条「待填」改为：面板内删除确认兜底位置观感结论（`.below` 是否遮挡下一张卡片、箭头对齐）、editor 窗锚定在 150% 缩放 / 靠右卡片 / 多屏下的表现（左翻是否命中、坐标是否偏移）、补丁清单（含「取消」按钮不走退场动效是否需要补）。

- [ ] **Step 4: 提交**

```bash
git add docs/superpowers/plans/2026-09-12-prototype-realign-phase4a-acceptance.md docs/superpowers/specs/2026-09-12-prototype-realign-phase4a-overlays-design.md src/styles/extensions.css
git commit -m "docs(design): 阶段四A 实机验收记录"
```

---

## 自检记录

- **Spec 覆盖**：§3 差异表 1–23 → Task 2（1、2、3、4、5、6）、Task 3（7、8、9、10、11、12、13、14、15）、Task 4（16）、Task 5（17、18、19、22）；20 / 21 / 23 为「保留」项无任务；§4.1 → Task 1；§4.2 → Task 2；§4.3 → Task 3（`TodoEditorPanel` + `constants/todoEditor.ts`）；§4.4 → Task 3（`useEditorWindow` / `EditorApp` / 三处调用方 / `window-fit.css` / `extensions.css`）；§4.5 → Task 4；§4.6 → Task 5；§5 测试 → Task 1 六例；§6 → Task 6；§7 六个批次 ↔ 六个 Task 一一对应。
- **类型与函数名一致性**：`Rect / AnchorOptions / AnchorResult / anchorBeside` 在 Task 1 定义，Task 2 `CardConfirm` 与 Task 3 `TodoEditorPanel` 同名引用；`SlideOptions.duration` 在 Task 2 Step 1 定义，Task 2（`CardConfirm` 'fast'）、Task 3（`TodoEditorPanel` 'base' / 'fast'）、Task 4（`RepeatMenu` 'base'）复用；`TodoTree.cardOf(id)` 在 Task 2 Step 4 expose，Task 3 Step 5 / 6 的 `cardOf(todo)` 调用；`TodoCard` / `TodoTree` 的 `edit-remark` 在 Task 2 加、Task 3 接 `mode: 'remark'`；`TodoEditorMode / TODO_SECTS / TODO_EDITOR_TITLES` 在 Task 3 Step 1 定义，`useEditorWindow` / `TodoEditorPanel` / 三处调用方同名引用；`EditorAnchor / TodoEditorPayload` 在 `useEditorWindow.ts` 定义、`EditorApp` 以 `import type` 引用；`physicalAnchorOf` 与 `EditorApp.resolveAnchor` 互逆（同用 `innerPosition` 物理坐标 + `scaleFactor`）；`RepeatMenu.open(anchor, rule) / pick(rule)` 在 Task 4 Step 1 定义、Step 2–4 调用；`popOut` 在 Task 5 Step 1 定义并从 `motion/index.ts` 导出、Step 2 使用；`api.todos.reminder` 五参签名与 `src/service/tauri.ts:133` 一致。
- **占位扫描**：无 TBD / 「类似于」/ 「适当处理」；Task 6 按要求只列步骤不含代码。
- **自检发现并就地修正**：
  1. spec §3 #5 与 #13 要求 `enter()` 用 `--dur-fast` / 0.18s，但 `motion/presets.ts` 的 `enter()` 时长固定为 `tokens.slow` 且用 outBack 回弹——Task 2 Step 1 给 `SlideOptions` 增加 `duration?: 'fast' | 'base' | 'slow'` 档位（默认值与现状一致，既有调用不受影响），非默认档位改用 `--ease-out`。
  2. spec §4.6 说 `ModalShell` 退场调 `pop()`「缩到 .96 淡出」，但 `presets.ts:146` 的 `pop()` 是 scale .95 → 1 的按压回弹、不含淡出——Task 5 新增 `popOut()`（scale 1 → .96 + opacity 1 → 0，`--dur-fast`，inQuad，保留终态）并导出。
  3. `window-fit.css:102-107` 把 editor 窗 `#app` 设为 `pointer-events: none`、只对 `.modal-overlay` 放开；新浮层容器是 `#todoEditorOverlay`（不带 `.modal-overlay` 类），不补规则整个窗口点不到——Task 3 Step 8 追加 `:root[data-window='editor'] #todoEditorOverlay { pointer-events: auto }`（spec 未提及）。
  4. spec §3 #6 的规则 `.todo-item.confirming .todo-del` 会让父待办确认态把子任务的 ✕ 也点亮（子任务 `.todo-item` 是父级后代）——Task 2 Step 7 改为 `.todo-item.confirming > .todo-body > .todo-del`，语义不变。
  5. spec §4.5 要求 `DayView` 接 `edit-repeat`，但 `DayView.vue:284-286` 的 🔁 span 只是展示、没有点击事件——Task 4 Step 4 给它加 `@click.stop` 并复用 `.todo-meta.repeat` 类。
  6. spec §3 #10 / §6 列出「📄 编辑备注」入口，但 `TodoCard.vue:181,187` 的 `RemarkDisplay @edit` 与 ✏️ 同走 `edit` 事件，调用方无法区分——Task 2 给 `TodoCard` / `TodoTree` 加 `edit-remark` 事件（备注徽章与文本行改发它），Task 3 接到 `mode: 'remark'`。
  7. spec §4.3 把 `TODO_SECTS` 放在 `TodoEditorPanel.vue` 内，但 `mode` 联合类型要被 `useEditorWindow` / `EditorApp` / 三处调用方共用，从 SFC 导出类型不便——抽到 `src/constants/todoEditor.ts`（连同标题表），组件只消费。
  8. `EditorApp.vue:75-80` 的 `parent` 只按 `payload.parentId` 解析，编辑既有子任务（edit / due 模式）时 `TodoEditorPanel` 拿不到父级，「子任务 ≤ 父」校验（spec §3 #15「保留」）会失效——Task 3 Step 4 (d) 改为 `payload.parentId ?? todo.parent_id`。
  9. 卡片删除 `ConfirmPopover` 后 `confirm-delete` / `cancel-delete` 事件不再有触发源，若只删用法不删声明，父级模板仍挂着死监听——Task 2 同步删除四张卡片的两个 emit 与所有父级 `@confirm-delete` / `@cancel-delete`，`TodoTree.confirmDelete` / 各页 `remove()` 改为无参、按 `confirm.confirm()` 返回的 id 找回对象。
  10. `DayView` 未监听 `todosChanged`（只有 `notesChanged`，`DayView.vue:124`），改走 editor 窗保存后日期详情不会刷新——Task 3 Step 7 加监听（spec §4.4 已提示核对）。
  11. spec §3 #19 与原型 `saveClipEdit` 不一致：原型空内容 toast 后仍关闭弹窗，spec 写「拦截并 return」（弹窗留着）——按 spec 实现，Task 6 验收第 4 项按「弹窗不关」核对。
  12. `TagManagerModal` / `ClipEditorModal` 的「取消」按钮直接 `emit('close')` 卸载，不经 `ModalShell.close()` 的退场动效；改它们需要 `ModalShell` 暴露 `close` 或调用方持 ref，超出本阶段范围——Task 5 注明为可接受差异，Task 6 记录。
  13. `anchorBeside` 的 `'below'` 兜底在 spec 里只写了「超出视口则改为 `anchor.top - H - 8`」，未写改到上方后仍可能超出——实现里再钳制一次到 `[10, vh − H − 10]`，Task 1 测试第 4 例覆盖「贴底改上方」。
  14. spec §4.4 说 `EditorApp` 拿到 payload 后换算锚点：若换算是异步的而 `ready` 不等它，浮层会先居中渲染再跳到卡片旁——Task 3 Step 4 (c)(d) 加 `anchorResolved` 门闩，`ready` 等它。
