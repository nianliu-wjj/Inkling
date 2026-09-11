# 原型对齐 · 阶段二「主窗口」实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把主窗口的壳、侧边栏五页签、八个视图按新原型 `docs/index.html#mainWindow` 对齐：结构、交互、图标、日期文案、搜索高亮、热力图 / 趋势图，并新增启动台页与灵动岛页。

**Architecture:** 逐视图就地对齐（spec D12）：保留 `useNotes/useClips/useTodos/useSettings` + 后端事件驱动的数据流，抽出共享件（`Icon`、`HeatTip`、`MiniHeatmap`、`TrendChart`、`ClipArchiveCard`）与纯函数（`utils/heatmap.ts`、`utils/search.ts`、`utils/todo.ts::searchTodos`）。纯函数用 `node --test` 直接跑 TS（自定义解析钩子处理 `@/` 别名与无扩展名导入）。后端只加 `MAIN_SHOWN` 事件与 `launcher_show` 命令。

**Tech Stack:** Vue 3.5 `<script setup lang="ts">` · TypeScript 5.9 · Vite 7 · animejs 4.5 · Node 24（`node:test` + `module.register` 钩子）· Tauri 2 / Rust

**设计文档：** `docs/superpowers/specs/2026-09-10-prototype-realign-phase2-main-window-design.md`（下文简称 spec）

## Global Constraints

- 唯一参考原型 `docs/index.html` + `docs/styles.css` + `docs/app.js`；生成层样式**不改**（`pnpm sync:styles --check` 必须始终通过）；项目自有样式只进 `src/styles/extensions.css`。
- 组件模板只用原型类名；不新增设计语言；颜色只用令牌。
- 已拍定：缺失能力不渲染控件（D10）；趋势图改 SVG 并卸载 ECharts（D11）；笔记编辑暂留弹窗（D13）；提醒徽章保持相对偏移文案（D14）。
- 编码规范：Vue 一律 `<script setup lang="ts">`，禁止 `any`；关键节点走 `src/service/logger.ts`；Rust 新增项带 `///` 注释；每个文件头部有职责说明注释。
- 动效硬约束沿用阶段一（只动 transform/opacity；`.glass` 静止态无 transform；入场由 JS 驱动并可重播）。
- 验证：每任务结束 `pnpm typecheck` 零错误；改动纯函数时 `pnpm test:unit`；改动 Rust 时 `cargo check` + `cargo test`；最后 `pnpm exec vite build`。
- 提交：中文提交信息，每任务一次提交。

---

## File Structure

```
scripts/lib/
├─ ts-resolve-hooks.mjs          Node 解析钩子：@/ → src/，无扩展名相对导入补 .ts
└─ register-ts-hooks.mjs         --import 入口：注册上面的钩子
src/utils/
├─ heatmap.ts (+ .test.ts)       周一对齐网格 / 档位 / 月份标签（迷你与统计两套阈值）
├─ search.ts (+ .test.ts)        highlight / mindmapAllText / mindmapRootText
├─ todo.ts (+ .test.ts)          新增 searchTodos
└─ datetime.ts (+ .test.ts)      formatDueLabel 按原型；新增 formatDateKey
src/components/base/Icon.vue     原型五枚 SVG
src/components/card/
├─ NoteCard.vue                  原型结构 + 标签直删 + 导图徽章
├─ ClipArchiveCard.vue           新：主窗口粘贴板卡片
├─ TodoCard.vue                  原型结构 + 高亮 + 已完成父级 ＋子任务
└─ TodoTree.vue                  query / forceExpand / hitIds / archive
src/components/stats/
├─ HeatTip.vue                   悬浮明细（格子上方居中）
├─ MiniHeatmap.vue               当月迷你热力图
└─ TrendChart.vue                原型 SVG 折线
src/components/tag/TagList.vue   支持 shaking 展开 + shake-tip
src/composables/
├─ useLauncherSearch.ts          启动台页内嵌搜索（去抖 / 键盘 / 执行）
└─ useLauncherSettings.ts        启动器设置逻辑（从 SettingsView 抽出）
src/constants/navigation.ts      五页签常量
src/typings/domain.ts            View 扩展
src/motion/presets.ts            popIn
src/service/{events,tauri}.ts    mainShown / launcher.show
src/windows/Main/
├─ MainApp.vue Sidebar.vue NotesView.vue ClipsView.vue TodosView.vue DayView.vue StatsView.vue SettingsView.vue   改
├─ LauncherPageView.vue          新
└─ IslandPageView.vue            新
src/styles/extensions.css        删 §5 .archive-page > ul
src-tauri/src/{events.rs, app/windows.rs, ipc.rs, main.rs}   MAIN_SHOWN / launcher_show
```

---

## Task 1: 单测基础设施 + 纯函数（heatmap / search / datetime / todo）

**Files:**
- Create: `scripts/lib/ts-resolve-hooks.mjs`、`scripts/lib/register-ts-hooks.mjs`
- Create: `src/utils/heatmap.ts`、`src/utils/heatmap.test.ts`、`src/utils/search.ts`、`src/utils/search.test.ts`、`src/utils/datetime.test.ts`、`src/utils/todo.test.ts`
- Modify: `src/utils/datetime.ts`、`src/utils/todo.ts`、`package.json`

**Interfaces:**
- Produces:
  - `buildMonthGrid(activity: readonly ActivityDay[], year: number, month0: number, opts?: { selected?: string; today?: Date }): MonthCell[]`，`MonthCell = { key: string; blank: boolean; level: 0|1|2|3|4; overdue: boolean; selected: boolean; day: ActivityDay | null }`
  - `buildRangeGrid(activity, opts?: { days?: number; today?: Date }): { cells: RangeCell[]; months: { label: string; column: number }[] }`，`RangeCell = { key; level; overdue; day }`
  - `alphaOf(level): number`；`MINI_THRESHOLDS`、`STATS_THRESHOLDS`；`levelOf(total, thresholds)`
  - `highlight(text: string, query: string): { text: string; hit: boolean }[]`；`mindmapAllText(json: string | null | undefined): string`；`mindmapRootText(json, fallback = '未命名导图'): string`
  - `formatDueLabel(value)`（原型口径）；`formatDateKey(key: string, opts?: { todaySuffix?: boolean }): string`
  - `searchTodos(todos: readonly Todo[], query: string): { nodes: TodoNode[]; forceExpand: Set<string>; hitIds: Set<string> }`
  - `pnpm test:unit`、`pnpm test`

- [x] **Step 1: 解析钩子**

`scripts/lib/ts-resolve-hooks.mjs`：

```js
/**
 * Node `node --test` 直接运行 src 下 TypeScript 单测的解析钩子：
 *   1. `@/xxx` 别名 → `src/xxx`（与 tsconfig paths 一致）；
 *   2. 无扩展名的相对导入按 .ts / .mts / .js / .mjs 依次探测（项目源码统一省略扩展名）。
 * 类型剥离由 Node 24 内建完成，无需编译步骤。
 */
import { existsSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const SRC = pathToFileURL(path.resolve('src') + '/').href
const EXTENSIONS = ['.ts', '.mts', '.js', '.mjs']

export async function resolve(specifier, context, nextResolve) {
  let spec = specifier
  if (spec.startsWith('@/')) spec = new URL(spec.slice(2), SRC).href
  const relative = spec.startsWith('./') || spec.startsWith('../') || spec.startsWith('file:')
  if (relative && !path.extname(spec)) {
    const base = spec.startsWith('file:') ? spec : new URL(spec, context.parentURL).href
    for (const ext of EXTENSIONS) {
      if (existsSync(fileURLToPath(base + ext))) return nextResolve(base + ext, context)
    }
  }
  return nextResolve(spec, context)
}
```

`scripts/lib/register-ts-hooks.mjs`：

```js
/** `node --import` 入口：注册 TS 解析钩子（见 ts-resolve-hooks.mjs）。 */
import { register } from 'node:module'
register('./ts-resolve-hooks.mjs', import.meta.url)
```

`package.json` scripts 增加（放在 `test:scripts` 后）。**实施偏差**：`node:test` 的类型声明不在主 tsconfig 的 `types` 里，单测文件要从 `tsconfig.json` 排除（`exclude: ["src/**/*.test.ts"]`），另建 `tsconfig.test.json`（`extends` 主配置，`types: ["node"]`，只 include 测试与其依赖的纯函数模块）并在 `test:unit` 前先跑 `tsc -p tsconfig.test.json`：

```json
"test:unit": "node --import ./scripts/lib/register-ts-hooks.mjs --test \"src/**/*.test.ts\"",
"test": "pnpm test:scripts && pnpm test:unit",
```

- [x] **Step 2: 写失败的测试（四个文件）**

`src/utils/heatmap.test.ts`：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import type { ActivityDay } from '@/typings/domain'
import { MINI_THRESHOLDS, STATS_THRESHOLDS, alphaOf, buildMonthGrid, buildRangeGrid, levelOf } from './heatmap'

const day = (date: string, notes = 0, clips = 0, todos = 0, overdue = 0): ActivityDay => ({
  date, notes, clips, todos, completed: 0, overdue,
})

test('levelOf：迷你 <3/<6/<10，统计 <5/<10/<18', () => {
  assert.deepEqual([0, 1, 2, 3, 5, 6, 9, 10].map((n) => levelOf(n, MINI_THRESHOLDS)), [0, 1, 1, 2, 2, 3, 3, 4])
  assert.deepEqual([0, 4, 5, 9, 10, 17, 18].map((n) => levelOf(n, STATS_THRESHOLDS)), [0, 1, 2, 2, 3, 3, 4])
  assert.deepEqual([0, 1, 2, 3, 4].map(alphaOf), [0, 0.18, 0.38, 0.6, 0.88])
})

test('buildMonthGrid：周一对齐，2026-09-01 是周二 → 1 个空位；共 5 列 35 格', () => {
  const cells = buildMonthGrid([day('2026-09-10', 2, 0, 1, 1)], 2026, 8, { selected: '2026-09-10' })
  assert.equal(cells.length, 35)
  assert.equal(cells[0].blank, true)
  assert.equal(cells[1].key, '2026-09-01')
  const target = cells.find((c) => c.key === '2026-09-10')!
  assert.equal(target.level, 2)
  assert.equal(target.overdue, true)
  assert.equal(target.selected, true)
  assert.equal(cells[31].key, '2026-09-30')
  assert.equal(cells[32].blank, true)
})

test('buildRangeGrid：182 天起点回退到周一；月份标签去重且间距 ≥3 列', () => {
  const today = new Date(2026, 8, 10) // 周四
  const { cells, months } = buildRangeGrid([day('2026-09-10', 20)], { days: 182, today })
  // 今天 − 181 天 = 2026-03-13（周五）→ 回退到周一 2026-03-09
  assert.equal(cells[0].key, '2026-03-09')
  assert.equal(cells.at(-1)!.key, '2026-09-10')
  assert.equal(cells.at(-1)!.level, 4)
  assert.deepEqual(months.map((m) => m.label), ['3月', '4月', '5月', '6月', '7月', '8月', '9月'])
  for (let i = 1; i < months.length; i++) assert.ok(months[i].column - months[i - 1].column >= 3)
})
```

`src/utils/search.test.ts`：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import { highlight, mindmapAllText, mindmapRootText } from './search'

test('highlight：忽略大小写，只标首个命中', () => {
  assert.deepEqual(highlight('Hello world hello', 'HELLO'), [
    { text: 'Hello', hit: true },
    { text: ' world hello', hit: false },
  ])
  assert.deepEqual(highlight('abc', ''), [{ text: 'abc', hit: false }])
  assert.deepEqual(highlight('abc', 'x'), [{ text: 'abc', hit: false }])
  assert.deepEqual(highlight('xyz', 'xyz'), [{ text: 'xyz', hit: true }])
})

test('mindmapAllText：全量格式与旧格式都递归，剥富文本标签', () => {
  const full = JSON.stringify({ root: { data: { text: '<p>根</p>' }, children: [{ data: { text: '子1' }, children: [{ data: { text: '孙' }, children: [] }] }] } })
  assert.equal(mindmapAllText(full), '根 子1 孙')
  const legacy = JSON.stringify({ data: { text: 'A' }, children: [{ data: { text: 'B' }, children: [] }] })
  assert.equal(mindmapAllText(legacy), 'A B')
  assert.equal(mindmapAllText(''), '')
  assert.equal(mindmapAllText('{not json'), '')
  assert.equal(mindmapAllText(null), '')
})

test('mindmapRootText：根节点文字，缺失回退', () => {
  assert.equal(mindmapRootText(JSON.stringify({ root: { data: { text: '<p>减脂计划</p>' }, children: [] } })), '减脂计划')
  assert.equal(mindmapRootText(JSON.stringify({ root: { data: { text: '  ' }, children: [] } })), '未命名导图')
  assert.equal(mindmapRootText(null), '未命名导图')
})
```

`src/utils/datetime.test.ts`：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import { formatDateKey, formatDueLabel, toDateKey } from './datetime'

test('formatDueLabel：今天带「今天」，其余一律 M/D HH:mm（无昨天/明天）', () => {
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 9, 5)
  assert.equal(formatDueLabel(today.toISOString()), '今天 09:05')
  const tomorrow = new Date(today.getTime() + 86_400_000)
  assert.equal(formatDueLabel(tomorrow.toISOString()), `${tomorrow.getMonth() + 1}/${tomorrow.getDate()} 09:05`)
  const yesterday = new Date(today.getTime() - 86_400_000)
  assert.equal(formatDueLabel(yesterday.toISOString()), `${yesterday.getMonth() + 1}/${yesterday.getDate()} 09:05`)
  assert.equal(formatDueLabel('not-a-date'), '')
})

test('formatDateKey：原样 YYYY-MM-DD，可选「 · 今天」后缀', () => {
  const todayKey = toDateKey(new Date())
  assert.equal(formatDateKey('2026-01-05'), '2026-01-05')
  assert.equal(formatDateKey(todayKey, { todaySuffix: true }), `${todayKey} · 今天`)
  assert.equal(formatDateKey('2000-01-01', { todaySuffix: true }), '2000-01-01')
})
```

`src/utils/todo.test.ts`：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import type { Todo } from '@/typings/domain'
import { searchTodos } from './todo'

const todo = (id: string, content: string, due: string, extra: Partial<Todo> = {}): Todo => ({
  id, content, due_at: due, completed_at: null, status: 'open', remind_at: null, remind_offset_minutes: null,
  remind_desktop: false, remind_email: false, repeat_rule: null, remind_off: false, priority: 'medium',
  remark: '', parent_id: null, tags: [], created_at: due, updated_at: due, ...extra,
})

const data: Todo[] = [
  todo('p1', '写周报', '2026-09-10T02:00:00.000Z'),
  todo('c1', '整理数据', '2026-09-10T01:00:00.000Z', { parent_id: 'p1' }),
  todo('p2', '买牛奶', '2026-09-12T02:00:00.000Z', { priority: 'high' }),
  todo('c2', '写周报草稿', '2026-09-12T01:00:00.000Z', { parent_id: 'p2' }),
  todo('p3', '无关事项', '2026-09-11T02:00:00.000Z'),
]

test('searchTodos：命中父级 → 整棵树；只命中子任务 → 父级强制展开并标记命中子任务', () => {
  const r = searchTodos(data, '周报')
  assert.deepEqual(r.nodes.map((n) => n.todo.id), ['p2', 'p1']) // 日期降序
  assert.deepEqual([...r.forceExpand], ['p2'])
  assert.deepEqual([...r.hitIds], ['c2'])
  assert.equal(r.nodes[1].children.length, 1)
})

test('searchTodos：无命中返回空；空查询返回空', () => {
  assert.equal(searchTodos(data, '不存在').nodes.length, 0)
  assert.equal(searchTodos(data, '  ').nodes.length, 0)
})
```

- [x] **Step 3: 运行，确认失败**

Run: `pnpm test:unit`
Expected: 4 个文件均失败（模块或导出不存在）。

- [x] **Step 4: 实现 heatmap.ts**

```ts
import type { ActivityDay } from '@/typings/domain'
import { toDateKey } from './datetime'

/**
 * 热力图网格纯函数（原型 renderMiniHeat / renderHeatmap 的口径）。
 *
 * - 一律**周一对齐**（原型 `(getDay() + 6) % 7`）；
 * - 档位阈值分两套：侧栏迷你热力图 <3/<6/<10，统计页 <5/<10/<18；
 * - 透明度阶 [0, .18, .38, .6, .88] 叠在主题令牌 --hm-base 上，由调用方拼 rgba。
 */
export type HeatLevel = 0 | 1 | 2 | 3 | 4
export const MINI_THRESHOLDS: readonly [number, number, number] = [3, 6, 10]
export const STATS_THRESHOLDS: readonly [number, number, number] = [5, 10, 18]
const ALPHAS = [0, 0.18, 0.38, 0.6, 0.88] as const

export function levelOf(total: number, thresholds: readonly [number, number, number]): HeatLevel {
  if (total <= 0) return 0
  if (total < thresholds[0]) return 1
  if (total < thresholds[1]) return 2
  if (total < thresholds[2]) return 3
  return 4
}

export function alphaOf(level: HeatLevel): number {
  return ALPHAS[level]
}

export interface MonthCell {
  key: string
  blank: boolean
  level: HeatLevel
  overdue: boolean
  selected: boolean
  day: ActivityDay | null
}

function totalOf(day: ActivityDay | null): number {
  return day ? day.notes + day.clips + day.todos : 0
}

/** 当月网格：列为周（周一起），行为星期；首尾补空位使总格数为 7 的倍数。 */
export function buildMonthGrid(
  activity: readonly ActivityDay[],
  year: number,
  month0: number,
  opts: { selected?: string } = {},
): MonthCell[] {
  const byDate = new Map(activity.map((d) => [d.date, d]))
  const lead = (new Date(year, month0, 1).getDay() + 6) % 7
  const daysInMonth = new Date(year, month0 + 1, 0).getDate()
  const cols = Math.ceil((lead + daysInMonth) / 7)
  const cells: MonthCell[] = []
  for (let i = 0; i < cols * 7; i++) {
    const dayNum = i - lead + 1
    if (dayNum < 1 || dayNum > daysInMonth) {
      cells.push({ key: `blank-${i}`, blank: true, level: 0, overdue: false, selected: false, day: null })
      continue
    }
    const key = toDateKey(new Date(year, month0, dayNum))
    const day = byDate.get(key) ?? null
    cells.push({
      key,
      blank: false,
      level: levelOf(totalOf(day), MINI_THRESHOLDS),
      overdue: (day?.overdue ?? 0) > 0,
      selected: opts.selected === key,
      day,
    })
  }
  return cells
}

export interface RangeCell {
  key: string
  level: HeatLevel
  overdue: boolean
  day: ActivityDay | null
}

/** 近 N 天网格（默认 182），起点回退到周一；月份标签按周列去重且相邻间距 ≥3 列。 */
export function buildRangeGrid(
  activity: readonly ActivityDay[],
  opts: { days?: number; today?: Date } = {},
): { cells: RangeCell[]; months: { label: string; column: number }[] } {
  const days = opts.days ?? 182
  const today = opts.today ?? new Date()
  const byDate = new Map(activity.map((d) => [d.date, d]))
  const start = new Date(today.getFullYear(), today.getMonth(), today.getDate())
  start.setDate(start.getDate() - (days - 1))
  start.setDate(start.getDate() - ((start.getDay() + 6) % 7))
  const end = new Date(today.getFullYear(), today.getMonth(), today.getDate())

  const cells: RangeCell[] = []
  const months: { label: string; column: number }[] = []
  let prevMonth = -1
  let lastColumn = -99
  const cursor = new Date(start)
  let index = 0
  while (cursor <= end) {
    const key = toDateKey(cursor)
    const day = byDate.get(key) ?? null
    cells.push({ key, level: levelOf(totalOf(day), STATS_THRESHOLDS), overdue: (day?.overdue ?? 0) > 0, day })
    if (index % 7 === 0) {
      const month = cursor.getMonth() + 1
      const column = index / 7
      if (month !== prevMonth && column - lastColumn >= 3) {
        months.push({ label: `${month}月`, column })
        prevMonth = month
        lastColumn = column
      }
    }
    cursor.setDate(cursor.getDate() + 1)
    index += 1
  }
  return { cells, months }
}
```

- [x] **Step 5: 实现 search.ts**

```ts
import { parseMindMapData, type MindMapRoot } from '@/windows/MindMap/core/persistence'

/**
 * 搜索相关纯函数（原型 hiText / mindmapAllText 的口径）。
 */

/** 命中高亮：忽略大小写只标首个命中片段，返回分段供模板渲染 <mark>（不用 v-html）。 */
export function highlight(text: string, query: string): { text: string; hit: boolean }[] {
  const needle = query.trim().toLowerCase()
  if (!needle) return [{ text, hit: false }]
  const index = text.toLowerCase().indexOf(needle)
  if (index < 0) return [{ text, hit: false }]
  const segments: { text: string; hit: boolean }[] = []
  if (index > 0) segments.push({ text: text.slice(0, index), hit: false })
  segments.push({ text: text.slice(index, index + needle.length), hit: true })
  if (index + needle.length < text.length) segments.push({ text: text.slice(index + needle.length), hit: false })
  return segments
}

/** 剥离 simple-mind-map 富文本节点里的 HTML 标签。 */
function stripRichText(value: unknown): string {
  return String(value ?? '')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

function walk(node: MindMapRoot, out: string[]): void {
  const text = stripRichText(node.data?.text)
  if (text) out.push(text)
  for (const child of node.children ?? []) walk(child, out)
}

/** 导图全部节点文本（空格拼接），供笔记搜索；非法数据返回空串。 */
export function mindmapAllText(json: string | null | undefined): string {
  if (!json?.trim()) return ''
  const out: string[] = []
  walk(parseMindMapData(json, '').root, out)
  return out.join(' ')
}

/** 导图根节点文字，作为导图笔记卡片正文（原型 saveMindmap 的 rootText）。 */
export function mindmapRootText(json: string | null | undefined, fallback = '未命名导图'): string {
  if (!json?.trim()) return fallback
  return stripRichText(parseMindMapData(json, '').root.data?.text) || fallback
}
```

（`persistence.ts` 是纯函数模块、无 Vue 依赖，钩子能直接解析。）

- [x] **Step 6: datetime.ts 与 todo.ts**

`datetime.ts`：把 `formatDueLabel` 的 `switch` 改为只保留 `case 0: return \`今天 ${clock}\``，其余 `return \`${date.getMonth() + 1}/${date.getDate()} ${clock}\``；注释改为「当天显示「今天 HH:mm」，其余显示「M/D HH:mm」（原型口径）」。在 `formatDateKeyLabel` 之后新增：

```ts
/** 原型口径的日期键展示：原样 YYYY-MM-DD，可选追加「 · 今天」。 */
export function formatDateKey(key: string, opts: { todaySuffix?: boolean } = {}): string {
  return opts.todaySuffix && key === todayKey() ? `${key} · 今天` : key
}
```

`todo.ts` 末尾新增：

```ts
/**
 * 跨日期搜索（原型 renderTodoList 的搜索分支）：
 * - 命中父级 → 整棵树；只命中子任务 → 父级进 forceExpand，首个命中子任务进 hitIds；
 * - 结果按完成日期降序，再按优先级。
 */
export function searchTodos(
  todos: readonly Todo[],
  query: string,
): { nodes: TodoNode[]; forceExpand: Set<string>; hitIds: Set<string> } {
  const needle = query.trim().toLowerCase()
  const empty = { nodes: [] as TodoNode[], forceExpand: new Set<string>(), hitIds: new Set<string>() }
  if (!needle) return empty
  const match = (t: Todo) => t.content.toLowerCase().includes(needle)
  const tree = buildTodoTree(todos)
  const forceExpand = new Set<string>()
  const hitIds = new Set<string>()
  const nodes = tree.filter((node) => {
    const parentHit = match(node.todo)
    const child = node.children.find(match)
    if (!parentHit && !child) return false
    if (child) hitIds.add(child.id)
    if (!parentHit) forceExpand.add(node.todo.id)
    return true
  })
  nodes.sort((a, b) => {
    const dateDiff = dateKeyOf(b.todo.due_at).localeCompare(dateKeyOf(a.todo.due_at))
    return dateDiff !== 0 ? dateDiff : weightOf(a.todo.priority) - weightOf(b.todo.priority)
  })
  return { nodes, forceExpand, hitIds }
}
```

（`todo.ts` 顶部 `import { parseTime } from './datetime'` 改为 `import { dateKeyOf, parseTime } from './datetime'`。）

- [x] **Step 7: 验证并提交**

```bash
pnpm test:unit
pnpm typecheck
git add scripts/lib/ts-resolve-hooks.mjs scripts/lib/register-ts-hooks.mjs package.json src/utils
git commit -m "feat(main): 热力图/搜索/日期/待办搜索纯函数与 node:test 单测基础设施"
```
Expected: `pass 9 / fail 0`；typecheck 零错误。

---

## Task 2: 共享件（Icon / HeatTip / MiniHeatmap / TrendChart）与卸载 ECharts

**Files:**
- Create: `src/components/base/Icon.vue`、`src/components/stats/HeatTip.vue`、`src/components/stats/MiniHeatmap.vue`、`src/components/stats/TrendChart.vue`
- Modify: `src/motion/presets.ts`、`src/motion/index.ts`、`package.json`（remove echarts）

**Interfaces:**
- `Icon`：`props.name: IconName`（`'close' | 'pin' | 'edit' | 'paste' | 'link'`）
- `HeatTip`：`props.day: ActivityDay`、`props.anchor: DOMRect`
- `MiniHeatmap`：`props.activity: readonly ActivityDay[]`、`props.selectedDate: string`；emits `pick(key)`、`hover(day: ActivityDay | null, anchor: DOMRect | null)`
- `TrendChart`：`props.months: readonly MonthTrend[]`
- `popIn(el: HTMLElement): Promise<boolean>`

- [x] **Step 1: Icon.vue**

```vue
<script setup lang="ts">
/**
 * 原型内联 SVG 图标（docs/app.js 的 ICON_CLOSE / ICON_PIN / ICON_EDIT / ICON_PASTE / ICON_LINK）。
 * 11×11，描边取 currentColor，随按钮颜色变化；装饰性图标，可读名由外层按钮的 title 提供。
 */
export type IconName = 'close' | 'pin' | 'edit' | 'paste' | 'link'
defineProps<{ name: IconName }>()
</script>

<template>
  <svg v-if="name === 'close'" width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
    <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
  </svg>
  <svg v-else-if="name === 'pin'" width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M9 4h6l1 7 3 3v2H5v-2l3-3 1-7z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
    <path d="M12 16v5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
  </svg>
  <svg v-else-if="name === 'edit'" width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M4 20h4l11-11-4-4L4 16v4z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
    <path d="M13 7l4 4" stroke="currentColor" stroke-width="1.6" />
  </svg>
  <svg v-else-if="name === 'paste'" width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    <rect x="8" y="2" width="8" height="4" rx="1" stroke="currentColor" stroke-width="1.6" />
  </svg>
  <svg v-else width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M14 3h7v7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
    <path d="M21 3l-9 9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    <path d="M19 14v5a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
  </svg>
</template>
```

- [x] **Step 2: popIn 预设**

`src/motion/presets.ts` 在 `pop` 之后加入：

```ts
/** 窗口 / 大面板入场：scale .94 → 1 + 淡入，--dur-base，--ease-out（原型 openMainWindow 的 gsap 参数）。 */
export function popIn(el: HTMLElement): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) {
    cancelRunning(el)
    clearInline(el)
    return Promise.resolve(true)
  }
  logger.debug('motion', 'popIn scale .94 → 1')
  return run(el, [el], { scale: [0.94, 1], opacity: [0, 1], duration: tokens.base, ease: tokens.easeOut })
}
```

`src/motion/index.ts` 的 presets 导出行加入 `popIn`。

- [x] **Step 3: HeatTip.vue**

```vue
<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from 'vue'
import { enter } from '@/motion'
import type { ActivityDay } from '@/typings/domain'

/**
 * 热力图悬浮明细（原型 showHeatTip）：fixed 定位在格子上方居中，上方放不下则翻到下方；
 * 存在逾期时红边。由父组件 Teleport 到 body，避免被滚动容器裁剪。
 */
const props = defineProps<{ day: ActivityDay; anchor: DOMRect }>()

const el = ref<HTMLElement | null>(null)
const style = ref<{ top: string; left: string }>({ top: '0px', left: '0px' })

const WEEK = ['日', '一', '二', '三', '四', '五', '六']
function weekdayOf(key: string): string {
  const [y, m, d] = key.split('-').map(Number)
  return WEEK[new Date(y, m - 1, d).getDay()]
}

async function place(): Promise<void> {
  await nextTick()
  const tip = el.value
  if (!tip) return
  const r = props.anchor
  const tw = tip.offsetWidth
  const th = tip.offsetHeight
  let top = r.top - th - 8
  if (top < 8) top = r.bottom + 8
  const left = Math.min(Math.max(8, r.left + r.width / 2 - tw / 2), window.innerWidth - tw - 8)
  style.value = { top: `${top}px`, left: `${left}px` }
}

watch(() => [props.anchor.top, props.anchor.left, props.day.date], () => void place())
onMounted(async () => {
  await place()
  if (el.value) void enter(el.value, { axis: 'y', distance: 4 })
})
</script>

<template>
  <div id="heatTip" ref="el" :class="{ ovd: props.day.overdue > 0 }" :style="style">
    <div class="tip-title">{{ props.day.date }} 周{{ weekdayOf(props.day.date) }}</div>
    <div class="tip-row">📝 笔记 <b>{{ props.day.notes }}</b> 条</div>
    <div class="tip-row">📋 复制项 <b>{{ props.day.clips }}</b> 条</div>
    <div class="tip-row">
      ✅ 待办 <b>{{ props.day.todos }}</b> 条 · 已完成 <b>{{ props.day.completed }}</b>
      <template v-if="props.day.overdue > 0"> · <span class="ovd-red">逾期 {{ props.day.overdue }}</span></template>
    </div>
  </div>
</template>
```

（`enter` 的 4px 位移 + `--dur-slow` 比原型 150ms 略长；本阶段接受，动效令牌是全站统一节奏。）

- [x] **Step 4: MiniHeatmap.vue**

```vue
<script setup lang="ts">
import { computed } from 'vue'
import type { ActivityDay } from '@/typings/domain'
import { alphaOf, buildMonthGrid } from '@/utils/heatmap'

/**
 * 侧边栏当月迷你热力图（原型 renderMiniHeat）：周一对齐、<3/<6/<10 四档、
 * 悬浮看明细（由父组件渲染 HeatTip）、点击查该日全部记录、选中日描边。
 */
const props = defineProps<{ activity: readonly ActivityDay[]; selectedDate: string }>()
const emit = defineEmits<{
  (e: 'pick', key: string): void
  (e: 'hover', day: ActivityDay | null, anchor: DOMRect | null): void
}>()

const now = new Date()
const title = computed(() => `${now.getMonth() + 1}月活跃（悬浮明细 · 点击查当日）`)
const cells = computed(() =>
  buildMonthGrid(props.activity, now.getFullYear(), now.getMonth(), { selected: props.selectedDate }),
)

function hmBase(): string {
  return getComputedStyle(document.documentElement).getPropertyValue('--hm-base').trim() || '108,140,255'
}
function background(level: number): string | undefined {
  return level ? `rgba(${hmBase()}, ${alphaOf(level as 0 | 1 | 2 | 3 | 4)})` : undefined
}

/** 无记录的日期也要能悬浮：给一份全 0 的明细。 */
function dayOf(cell: { key: string; day: ActivityDay | null }): ActivityDay {
  return cell.day ?? { date: cell.key, notes: 0, clips: 0, todos: 0, completed: 0, overdue: 0 }
}
</script>

<template>
  <div class="mini-heat" @mouseleave="emit('hover', null, null)">
    <div class="mh-title">{{ title }}</div>
    <div class="mh-grid">
      <template v-for="cell in cells" :key="cell.key">
        <i v-if="cell.blank" class="mh-blank" />
        <span
          v-else
          class="heat-cell mh-cell"
          :class="{ ovd: cell.overdue, selected: cell.selected }"
          :style="background(cell.level) ? { background: background(cell.level) } : undefined"
          :data-date="cell.key"
          @mouseenter="emit('hover', dayOf(cell), ($event.currentTarget as HTMLElement).getBoundingClientRect())"
          @click="emit('pick', cell.key)"
        />
      </template>
    </div>
  </div>
</template>
```

- [x] **Step 5: TrendChart.vue**

```vue
<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import type { MonthTrend } from '@/typings/domain'

/**
 * 近 6 个月趋势折线（原型 renderTrend 的内联 SVG）：
 * 620×190 视口，网格 4 条（0 / ⅓ / ⅔ / max，max 向上取整到 10），三条折线 + 数据点 title 悬浮，
 * 颜色读主题令牌 --trend-*，主题切换后重取。
 */
const props = defineProps<{ months: readonly MonthTrend[] }>()
const { settings } = useSettings()

const W = 620
const H = 190
const P = { l: 40, r: 12, t: 24, b: 28 }
const iw = W - P.l - P.r
const ih = H - P.t - P.b

function themeVar(name: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback
}

/** 颜色随主题变化：以 theme 为依赖重算。 */
const colors = ref(readColors())
function readColors() {
  return {
    note: themeVar('--trend-note', '#ff8a8a'),
    clip: themeVar('--trend-clip', '#ffd76e'),
    todo: themeVar('--trend-todo', '#7ee0a8'),
    dim: themeVar('--text-dim', 'rgba(255,255,255,.5)'),
    wsa: themeVar('--wsa', '255,255,255'),
  }
}
watch(() => settings.value.theme, () => (colors.value = readColors()), { flush: 'post' })

const series = computed(() => [
  { key: 'notes' as const, label: '笔记', color: colors.value.note },
  { key: 'clips' as const, label: '粘贴板', color: colors.value.clip },
  { key: 'todos' as const, label: '待办', color: colors.value.todo },
])

const niceMax = computed(() => {
  const max = Math.max(10, ...props.months.flatMap((m) => [m.notes, m.clips, m.todos]))
  return Math.ceil(max / 10) * 10
})
const x = (i: number) => P.l + (props.months.length === 1 ? iw / 2 : (i * iw) / Math.max(1, props.months.length - 1))
const y = (v: number) => P.t + ih - (v / niceMax.value) * ih
const grid = computed(() => [0, 1, 2, 3].map((g) => ({ v: (niceMax.value * g) / 3, y: y((niceMax.value * g) / 3) })))
const monthLabel = (month: string) => `${Number(month.slice(5))}月`
</script>

<template>
  <div class="trend-chart">
    <div class="trend-legend">
      <span v-for="s in series" :key="s.key"><i :style="{ background: s.color }" />{{ s.label }}</span>
    </div>
    <svg :viewBox="`0 0 ${W} ${H}`" preserveAspectRatio="xMidYMid meet" role="img" aria-label="月度趋势折线图">
      <template v-for="g in grid" :key="g.v">
        <line :x1="P.l" :y1="g.y" :x2="W - P.r" :y2="g.y" :stroke="`rgba(${colors.wsa},.09)`" />
        <text :x="P.l - 8" :y="g.y + 3.5" text-anchor="end" font-size="10" :fill="colors.dim">{{ Math.round(g.v) }}</text>
      </template>
      <template v-for="s in series" :key="s.key">
        <polyline
          :points="props.months.map((m, i) => `${x(i)},${y(m[s.key])}`).join(' ')"
          fill="none"
          :stroke="s.color"
          stroke-width="2"
          stroke-linejoin="round"
          stroke-linecap="round"
        />
        <circle v-for="(m, i) in props.months" :key="m.month" :cx="x(i)" :cy="y(m[s.key])" r="3.2" :fill="s.color">
          <title>{{ m.month }} · {{ s.label }}：{{ m[s.key] }}</title>
        </circle>
      </template>
      <text v-for="(m, i) in props.months" :key="m.month" :x="x(i)" :y="H - 8" text-anchor="middle" font-size="10.5" :fill="colors.dim">
        {{ monthLabel(m.month) }}
      </text>
    </svg>
  </div>
</template>
```

- [x] **Step 6: 卸载 ECharts 并验证**

```bash
pnpm remove echarts
grep -rn "echarts" src package.json; echo "exit=$?"    # 此时 StatsView 仍引用 → 本步先临时保留其 import 会报错
```

因此 **本任务顺序调整为**：先在 `StatsView.vue` 中删除 ECharts 相关代码（`import * as echarts`、`echarts.use`、`chart`、`renderTrend`、`onResize`、`trendHost`、`watch(trend, renderTrend)`、卸载钩子），趋势区替换为 `<TrendChart :months="trend" />`（导入 `TrendChart`）。热力图部分暂不动（Task 8 再改）。然后：

```bash
pnpm remove echarts
grep -rn "echarts" src package.json; echo "exit=$?"     # 期望无输出、exit=1
pnpm typecheck
pnpm exec vite build 2>&1 | grep -E "main-.*\.js|built in"
git add src/components/base/Icon.vue src/components/stats src/motion package.json pnpm-lock.yaml src/windows/Main/StatsView.vue
git commit -m "feat(main): 共享件 Icon / HeatTip / MiniHeatmap / TrendChart 与 popIn 预设，趋势图改原型 SVG 并卸载 echarts"
```
记录 `main-*.js` 体积（验收第 7 项要对比阶段一的 556254 字节）。

---

## Task 3: 后端 MAIN_SHOWN 事件与 launcher_show 命令

**Files:**
- Modify: `src-tauri/src/events.rs`、`src-tauri/src/app/windows.rs:833-840`、`src-tauri/src/ipc.rs`、`src-tauri/src/main.rs`、`src/service/events.ts`、`src/service/tauri.ts`

- [x] **Step 1: Rust**

`events.rs` 末尾：
```rust
/// 主窗口已由 show_main 显示（广播）：前端据此重播入场动效。
pub const MAIN_SHOWN: &str = "inkling://main-shown";
```
`windows::show_main` 中 `let _ = app.emit(events::NAVIGATE, view.to_string());` 之后加 `let _ = app.emit(events::MAIN_SHOWN, ());`。

`ipc.rs` 在 `launcher_hide` 之后：
```rust
/// 显示启动台浮窗（主窗口启动台页的「呼出浮窗启动台」按钮）。
#[tauri::command]
pub fn launcher_show(app: AppHandle) -> Result<(), String> {
    windows::launcher_show(&app)
}
```
`main.rs` 命令列表 `ipc::launcher_hide,` 之后加 `ipc::launcher_show,`。

- [x] **Step 2: 前端契约**

`events.ts` 的 `AppEvents` 加 `/** 主窗口已显示（广播）：主窗口据此重播入场动效。 */ mainShown: 'inkling://main-shown',`；`tauri.ts` 的 `launcher` 对象加 `show: () => invoke<void>('launcher_show'),`。

- [x] **Step 3: 验证并提交**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check && cargo check --manifest-path src-tauri/Cargo.toml && cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | grep "^test result"
pnpm typecheck
git add src-tauri/src/events.rs src-tauri/src/app/windows.rs src-tauri/src/ipc.rs src-tauri/src/main.rs src/service/events.ts src/service/tauri.ts
git commit -m "feat(ipc): 主窗口 main-shown 事件与 launcher_show 命令"
```

---

## Task 4: 主窗口壳与侧边栏五页签

**Files:**
- Modify: `src/typings/domain.ts:1`、`src/constants/navigation.ts`、`src/windows/Main/MainApp.vue`、`src/windows/Main/Sidebar.vue`

- [x] **Step 1: 类型与常量**

`domain.ts`：`export type View = 'notes' | 'clips' | 'todos' | 'launcher' | 'island' | 'stats' | 'settings'`。

`constants/navigation.ts` 整体替换：

```ts
import type { View } from '@/typings/domain'

/**
 * 主窗口侧边栏页签（原型 #archiveSide .side-nav 的五项，顺序固定）。
 * 计数键为空的页签不显示徽章；分类色 --tint 由生成层 CSS 按 data-view 提供。
 */
export interface NavigationItem {
  key: Extract<View, 'notes' | 'clips' | 'todos' | 'launcher' | 'island'>
  icon: string
  label: string
  countKey?: 'notes' | 'clips' | 'todos'
}

export const navigationItems: readonly NavigationItem[] = [
  { key: 'notes', icon: '📝', label: '笔记', countKey: 'notes' },
  { key: 'clips', icon: '📋', label: '粘贴板', countKey: 'clips' },
  { key: 'todos', icon: '✅', label: '待办', countKey: 'todos' },
  { key: 'launcher', icon: '🚀', label: '启动台' },
  { key: 'island', icon: '🏝️', label: '灵动岛' },
]
```

- [x] **Step 2: Sidebar.vue**

脚本：删除本地 `NAV` 与整段迷你热力图逻辑（`heatBackground`、`monthGrid`、`monthLabel`、`toDateKey` 导入）；导入 `navigationItems`、`MiniHeatmap`、`HeatTip`；新增：

```ts
/** 迷你热力图悬浮明细：由本组件 Teleport 到 body 渲染。 */
const tip = ref<{ day: ActivityDay; anchor: DOMRect } | null>(null)
function onHeatHover(day: ActivityDay | null, anchor: DOMRect | null): void {
  tip.value = day && anchor ? { day, anchor } : null
}
function onKeyActivate(event: KeyboardEvent, view: View): void {
  event.preventDefault()
  emit('navigate', view)
}
```
`View` 类型导入保留。模板的 `.side-nav` 改为：

```vue
<div class="side-nav" role="tablist" aria-label="数据视图切换">
  <div
    v-for="item in navigationItems"
    :key="item.key"
    class="side-item"
    :class="{ active: props.view === item.key }"
    :data-view="item.key"
    role="tab"
    tabindex="0"
    :aria-selected="props.view === item.key"
    :title="item.label"
    @click="emit('navigate', item.key)"
    @keydown.enter="onKeyActivate($event, item.key)"
    @keydown.space="onKeyActivate($event, item.key)"
  >
    <span class="si-icon"><span class="ix">{{ item.icon }}</span></span>
    <span class="si-label">{{ item.label }}</span>
    <span class="si-count">{{ item.countKey ? props.counts[item.countKey] || '' : '' }}</span>
  </div>
</div>

<MiniHeatmap :activity="props.activity" :selected-date="props.selectedDate" @pick="emit('pick-date', $event)" @hover="onHeatHover" />
<Teleport to="body">
  <HeatTip v-if="tip" :day="tip.day" :anchor="tip.anchor" />
</Teleport>
```
底部两个按钮的 emoji 包一层 `<span class="ix">`（原型）。头注释更新为五页签。

- [x] **Step 3: MainApp.vue**

- `view` 类型 `View | 'day'`，新增 `LauncherPageView`、`IslandPageView` 分支（组件在 Task 9/10 创建；本任务先建两个最小占位文件，内容仅 `<div class="archive-page"><div class="page-title">🚀 启动台</div></div>` / `🏝️ 灵动岛`）。
- 计数：`notes = notes.filter(n => !n.is_draft).length`、`clips = clips.length`、`todos = todos.length`。
- 入场：`const root = ref<HTMLElement | null>(null)`；`onMounted` 里 `if (root.value) void popIn(root.value)`；订阅 `AppEvents.mainShown` 同样 `popIn`。模板根 `<div id="mainWindow" ref="root" ...>`。
- `navigate()` 收到未知值时 `logger.warn` 并忽略。

- [x] **Step 4: 验证并提交**

```bash
pnpm typecheck
git add src/typings/domain.ts src/constants/navigation.ts src/windows/Main/MainApp.vue src/windows/Main/Sidebar.vue src/windows/Main/LauncherPageView.vue src/windows/Main/IslandPageView.vue
git commit -m "feat(main): 侧边栏五页签与迷你热力图悬浮明细，主窗口入场动效与 main-shown 重播"
```

---

## Task 5: 笔记页

**Files:**
- Modify: `src/components/tag/TagList.vue`、`src/components/card/NoteCard.vue`、`src/windows/Main/NotesView.vue`

- [x] **Step 1: TagList 支持抖动态**

props 新增 `shaking?: boolean`（默认 false）：为 true 时 `visibleTags = props.tags`（全部展开）且渲染 `<span class="shake-tip">再次点击 ✕ 确认删除</span>`；`shakingTag` 仍按标签名传给 `TagChip`。

- [x] **Step 2: NoteCard 按原型**

props 新增 `shaking?: boolean`、`shakingTag?: string | null`；emits 新增 `(e: 'remove-tag', tag: string)`。模板：

```vue
<div class="archive-item" :class="{ pinned: props.note.pinned, shaking: props.shaking }">
  <ConfirmPopover v-if="props.confirming" ... />
  <button type="button" class="card-close" title="删除该笔记" @click="emit('ask-delete')"><Icon name="close" /></button>
  <div class="a-text">
    <span v-if="isMindmap" class="clip-type mindmap"><span class="ix">🧠</span> 思维导图</span>
    <span v-html="html" />
  </div>
  <div class="a-meta">
    <span><template v-if="props.note.pinned"><span class="ix">📌</span> </template>{{ stamp }}</span>
    <TagList :tags="props.note.tags" :max="3" deletable :shaking="props.shaking" :shaking-tag="props.shakingTag" @open="emit('open-tags')" @remove="emit('remove-tag', $event)" />
    <span class="a-ops">
      <IconBtn :variant="props.note.pinned ? 'active-pin' : ''" :title="props.note.pinned ? '取消置顶' : '置顶'" @click="emit('pin')"><Icon name="pin" /></IconBtn>
      <IconBtn title="编辑" @click="emit('edit')"><Icon name="edit" /></IconBtn>
    </span>
  </div>
</div>
```
`html`：导图笔记为 `renderMarkdownInline(mindmapRootText(note.mindmap_data))`，文本笔记为 `renderMarkdown(note.content)`。删除 `.note-kind` 徽章与 `mindmapLabel`。

- [x] **Step 3: NotesView**

- 删除 `kindFilter` 与工具栏下拉；工具栏改为 `<div class="note-arch-bar"><input class="search-input" placeholder="🔍 搜索笔记…（正文、标签与思维导图节点）" /><button class="btn tiny" title="新建思维导图（保存后作为笔记卡片入列表）" @click="openMindmap()"><span class="ix">🧠</span> 思维导图</button></div>`。
- `visible` 过滤补 `|| (note.editor_mode === 'mindmap' && mindmapAllText(note.mindmap_data).toLowerCase().includes(key))`。
- 标签直删：`const shake = useShakeConfirm()`；`shakeKey(note, tag) = \`${note.id}:${tag}\``；`removeTag(note, tag)`：`if (!shake.press(shakeKey)) { toast('再次点击 ✕ 确认删除该标签'); return }` → `api.notes.save({ id, content, tags: note.tags.filter(t => t !== tag), editorMode, mindmapData, draft: false })`，成功 toast `已删除标签 #tag`。卡片属性：`:shaking="shake.shakingId?.startsWith(note.id + ':')"`、`:shaking-tag="shake.shakingId?.startsWith(note.id + ':') ? shake.shakingId.slice(note.id.length + 1) : null"`（用 computed 封装成函数 `shakingTagOf(note)`）。
- 空提示：`keyword ? '未找到匹配的笔记' : '暂无笔记'`，类名 `todo-empty`。

- [x] **Step 4: 验证并提交**

```bash
pnpm typecheck
git add src/components/tag/TagList.vue src/components/card/NoteCard.vue src/windows/Main/NotesView.vue
git commit -m "feat(main): 笔记页对齐原型（工具栏 / 导图卡片 / 标签抖动直删 / 导图节点搜索）"
```

---

## Task 6: 粘贴板页

**Files:**
- Create: `src/components/card/ClipArchiveCard.vue`
- Modify: `src/windows/Main/ClipsView.vue`、`src/styles/extensions.css`

- [x] **Step 1: ClipArchiveCard.vue**

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
 * 主窗口粘贴板归档卡片（原型 renderArchive 的 .archive-item.clip-arch）：
 * 正文最多两行（CSS 截断）、类型徽章、时间、右下操作组（粘贴 / 置顶 / 外链仅 link / 编辑仅文本类）。
 * 面板里的紧凑卡片是另一个组件（ClipCard），阶段三对齐。
 */
const props = withDefaults(defineProps<{ entry: ClipboardEntry; confirming?: boolean }>(), { confirming: false })
const emit = defineEmits<{
  (e: 'paste'): void
  (e: 'pin'): void
  (e: 'edit'): void
  (e: 'open-link'): void
  (e: 'ask-delete'): void
  (e: 'confirm-delete'): void
  (e: 'cancel-delete'): void
}>()

const stamp = computed(() => formatStamp(props.entry.modified_at || props.entry.copied_at))
const editable = computed(() => props.entry.content_type !== 'image')
const isLink = computed(() => props.entry.content_type === 'link')
</script>

<template>
  <div class="archive-item clip-arch" :class="{ pinned: props.entry.pinned }">
    <ConfirmPopover v-if="props.confirming" text="⚠️ 确认删除该剪贴板条目？" @confirm="emit('confirm-delete')" @cancel="emit('cancel-delete')" />
    <button type="button" class="card-close" title="删除该条目" @click="emit('ask-delete')"><Icon name="close" /></button>
    <div class="a-text">{{ props.entry.preview || props.entry.content }}</div>
    <div class="a-meta">
      <ClipTypeBadge :content-type="props.entry.content_type" />
      <span><template v-if="props.entry.pinned"><span class="ix">📌</span> </template>{{ stamp }}</span>
      <span class="a-ops">
        <IconBtn title="粘贴到鼠标光标处" @click="emit('paste')"><Icon name="paste" /></IconBtn>
        <IconBtn :variant="props.entry.pinned ? 'active-pin' : ''" :title="props.entry.pinned ? '取消置顶' : '置顶'" @click="emit('pin')"><Icon name="pin" /></IconBtn>
        <IconBtn v-if="isLink" variant="clip-open" title="用默认浏览器打开该链接" @click="emit('open-link')"><Icon name="link" /></IconBtn>
        <IconBtn v-if="editable" title="编辑内容（弹框回显修改）" @click="emit('edit')"><Icon name="edit" /></IconBtn>
      </span>
    </div>
  </div>
</template>
```

- [x] **Step 2: ClipsView 换卡片、容器改 div、删桥接**

模板 `<ul v-stagger-list>` → `<div v-stagger-list>`，`ClipCard` → `ClipArchiveCard`（去掉 `archive` 属性），空提示 `<div class="todo-empty">{{ keyword ? '未找到匹配的条目' : '暂无粘贴板条目' }}</div>`；搜索框 placeholder `🔍 搜索粘贴板历史…`。`extensions.css` 删除 §5 中 `.archive-page > ul` 规则及其注释。

- [x] **Step 3: 验证并提交**

```bash
pnpm typecheck && pnpm exec prettier --check src/styles/extensions.css
git add src/components/card/ClipArchiveCard.vue src/windows/Main/ClipsView.vue src/styles/extensions.css
git commit -m "feat(main): 粘贴板页归档卡片对齐原型，删除裸 ul 桥接"
```

---

## Task 7: 待办页

**Files:**
- Modify: `src/components/card/TodoCard.vue`、`src/components/card/TodoTree.vue`、`src/windows/Main/TodosView.vue`

- [x] **Step 1: TodoCard 结构与高亮**

- props 新增 `query?: string`（默认 ''）、`childCount?: number`（默认 0）；`isParent` 语义改为「深度 0」。
- 文本：`<span class="todo-text"><template v-for="(seg, i) in highlight(props.todo.content, props.query)" :key="i"><mark v-if="seg.hit">{{ seg.text }}</mark><template v-else>{{ seg.text }}</template></template></span>`。
- 结构改为原型：`.todo-body > [ConfirmPopover?, .todo-del(仅未完成), .todo-head > [tree-toggle, checkbox, .todo-main > [.todo-row, RemarkDisplay(text 形态)]], .todo-foot > [.todo-foot-left(标签 + DueBadge), .todo-ops]]`——即 `.todo-foot` 从 `.todo-main` 移出，与 `.todo-head` 平级。
- `todo-del` 用 `<button class="card-close todo-del" title="删除待办"><Icon name="close" /></button>`，`v-if="!done"`。
- `＋子任务`：`v-if="props.depth === 0 && props.childCount < 5"`，title `添加子任务（${childCount}/5）` + 已完成时追加 ` · 新建后自动恢复为未完成`；`⏰` 与 `✏️` 仅未完成显示（`v-if="!done"`），✏️ 用 `<Icon name="edit" />`。
- 头注释更新。

- [x] **Step 2: TodoTree**

props 新增 `query?: string`、`forceExpand?: ReadonlySet<string>`、`hitIds?: ReadonlySet<string>`、`archive?: boolean`；`isCollapsed(id)` 改为 `collapsed.has(id) && !props.forceExpand?.has(id)`；根 `<ul v-stagger-list class="todo-list" :class="{ 'todo-arch-list': props.archive }">`；给每个 `TodoCard` 传 `:query`、`:child-count="node.children.length"`、`:search-hit="props.hitIds?.has(todo.id)"`（子任务用 `child.id`）；逾期分区标题改为 `⚠️ 逾期事项 · 按完成时间与优先级置顶（N 项）`。

- [x] **Step 3: TodosView**

- 日期条：`‹` `{{ currentDate }}` `›` `今天`（常显）`<span class="todo-date-hint">逾期未完成自动置顶</span>` 搜索框 `todo-new-btn`（`<Icon>` 不适用，保留原有 SVG ＋）。
- 搜索态：`const search = computed(() => searchTodos(todos.value, keyword.value))`；`visible` 非搜索态不变；`<TodoTree :todos="searching ? flatten(search.nodes) : visible" :query="keyword" :force-expand="search.forceExpand" :hit-ids="search.hitIds" :show-date-chip="searching" archive ...>`，其中 `flatten(nodes) = nodes.flatMap(n => [n.todo, ...n.children])`。
- 空提示按原型：搜索态 `未找到匹配「q」的待办事项`，否则 `该日暂无待办事项`。
- 删除 `formatDateKeyLabel` 导入。

- [x] **Step 4: 验证并提交**

```bash
pnpm typecheck
git add src/components/card/TodoCard.vue src/components/card/TodoTree.vue src/windows/Main/TodosView.vue
git commit -m "feat(main): 待办页对齐原型（卡片底栏结构 / 日期条 / 搜索高亮与强制展开 / 子任务上限）"
```

---

## Task 8: 日期详情页与统计页

**Files:**
- Modify: `src/windows/Main/DayView.vue`、`src/windows/Main/StatsView.vue`

- [x] **Step 1: DayView**

- 标题 `formatDateKey(props.dateKey, { todaySuffix: true })`。
- 筛选 chip：`role="button" tabindex="0" :aria-pressed="filter === option.key"`，Enter/Space 触发；标签用 `<span class="ix">📝</span> 笔记` 形式。
- `useTodos()` 取 `todos` 以查父级文本：`parentTextOf(todo) = todos.find(t => t.id === todo.parent_id)?.content`。
- 卡片模板按 spec §4.2 顺序：`day-time` · `day-badge` · `.day-body > [.day-title-row(PriorityBadge readonly | ClipTypeBadge | 导图徽章 | .day-text.d-clamp4|3 | .day-overdue), .day-meta(⏰ 前 N 分钟 · 🔁 每天/每周), .day-tags, .day-remark, .day-sub]` · `.day-ops(edit / del)`；正文用 `renderMarkdownInline(textOf(item))`；导图笔记正文为 `mindmapRootText`。
- 编辑：`editTarget` 三态 —— 笔记文本 → `NoteEditModal`；导图 → `api.windows.mindmapOpen(id)`；粘贴板 → `ClipEditorModal`（保存走 `api.clipboard.update`）；待办 → `TodoEditorModal mode="edit"`（已完成 toast「已完成的待办不允许修改」）。保存后 `await load()`。
- 空提示 `todo-empty`：搜索态 `未找到匹配「q」的记录`，否则 `该日暂无记录/笔记/粘贴板条目/待办事项`。

- [x] **Step 2: StatsView 热力图**

用 `buildRangeGrid(activity)` 替换本地 `grid`；格子 `:style` 用 `alphaOf(cell.level)` 拼 `rgba(--hm-base, α)`；悬浮改为 `HeatTip`（`Teleport to="body"`，`@mouseenter` 传 `getBoundingClientRect()`，容器 `@mouseleave` 清空）；月份标签 `left = column * 17`；图例：

```vue
<div class="heat-legend">少
  <i v-for="l in [1,2,3,4]" :key="l" :style="{ background: `rgba(${hmBase}, ${alphaOf(l as HeatLevel)})` }" />
  多 <i class="lg-ovd" :style="{ background: `rgba(${hmBase}, .3)` }" /> 存在逾期
</div>
```

- [x] **Step 3: 验证并提交**

```bash
pnpm typecheck
git add src/windows/Main/DayView.vue src/windows/Main/StatsView.vue
git commit -m "feat(main): 日期详情页与统计热力图对齐原型（周一对齐 / 图例 / 悬浮明细 / 编辑入口）"
```

---

## Task 9: 启动台页

**Files:**
- Create: `src/composables/useLauncherSearch.ts`、`src/composables/useLauncherSettings.ts`
- Modify: `src/windows/Main/LauncherPageView.vue`、`src/windows/Main/SettingsView.vue`

- [x] **Step 1: useLauncherSearch.ts**

```ts
import { computed, ref, watch, type Ref } from 'vue'
import { logger } from '@/service/logger'
import { api, type LauncherHit } from '@/service/tauri'

/** 命中类型 → 原型 .li-cat 文案。 */
export const LAUNCHER_KIND_LABEL: Record<LauncherHit['kind'], string> = {
  app: '应用', uwp: '应用', file: '文件', folder: '文件夹', command: '命令',
}
export const LAUNCHER_KIND_ICON: Record<LauncherHit['kind'], string> = {
  app: '🖥', uwp: '🧩', file: '📄', folder: '📁', command: '⚡',
}

/**
 * 启动台内嵌搜索（主窗口启动台页与浮窗启动台共用数据源）：
 * 输入去抖 30ms → api.launcher.search；↑↓ 循环选择；Enter / 点击执行「打开」。
 */
export function useLauncherSearch(): {
  query: Ref<string>
  hits: Ref<LauncherHit[]>
  active: Ref<number>
  current: Ref<LauncherHit | null>
  move: (delta: number) => void
  run: (index?: number) => Promise<void>
  onKeydown: (event: KeyboardEvent) => void
} {
  const query = ref('')
  const hits = ref<LauncherHit[]>([])
  const active = ref(0)
  let debounce: ReturnType<typeof setTimeout> | null = null

  async function search(): Promise<void> {
    try {
      hits.value = await api.launcher.search(query.value)
      active.value = 0
    } catch (error) {
      logger.error('launcher-page', '搜索失败', error)
      hits.value = []
    }
  }
  watch(query, () => {
    if (debounce) clearTimeout(debounce)
    debounce = setTimeout(() => void search(), 30)
  }, { immediate: true })

  const current = computed(() => hits.value[active.value] ?? null)
  function move(delta: number): void {
    if (!hits.value.length) return
    active.value = (active.value + delta + hits.value.length) % hits.value.length
  }
  async function run(index = active.value): Promise<void> {
    const hit = hits.value[index]
    if (!hit) return
    logger.info('launcher-page', `执行 ${hit.kind} ${hit.name}`)
    try {
      await api.launcher.launch(hit.path, hit.kind, 'open', query.value)
    } catch (error) {
      logger.error('launcher-page', '执行失败', error)
    }
  }
  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') { event.preventDefault(); move(1) }
    else if (event.key === 'ArrowUp') { event.preventDefault(); move(-1) }
    else if (event.key === 'Enter') { event.preventDefault(); void run() }
  }
  return { query, hits, active, current, move, run, onKeydown }
}
```

- [x] **Step 2: useLauncherSettings.ts**

把 `SettingsView.vue` 中 `launcherStatus / rebuilding / refreshLauncherStatus / rebuildLauncher / toggleFullDiskIndex / setExtraExcludes / LauncherRootRow / launcherRoots / saveLauncherRoots / newRootPath / addLauncherRoot / removeLauncherRoot / setRootDepth` 原样搬入，签名 `useLauncherSettings(patch: (p: Partial<Settings>) => Promise<void>)`，返回这些成员；`onMounted(refreshLauncherStatus)` 由调用方负责。

- [x] **Step 3: LauncherPageView.vue**

按 spec §4.2：`.page-title` 🚀 启动台 → `.stats-legend` 说明 → `.launcher-page-hero > [.launcher-input-row(.launcher-ico 🚀 + input#launcherPageInput), .launcher-list(.launcher-item(.active) > .li-ico .li-name(+small path) .li-cat | .launcher-empty), .launcher-footer]` → `.page-title.page-sub-title` ⚙️ 启动台设置 → `.settings-body`：快捷键行（录制逻辑：从 SettingsView 抽出 `recording/recordTarget/startRecording/rebind` 到本页与设置页各自一份，设置页只保留面板快捷键，`recordTarget` 简化）、全盘索引开关、排除目录、索引状态与重建、扫描根目录、`<div class="setting-row"><span>浮窗启动台</span><button class="btn tiny" @click="api.launcher.show()">呼出浮窗启动台</button></div>`。页脚：有结果 `↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 N 项`，无结果 `.launcher-empty` `未找到匹配项`。空查询时列表显示常用项（后端已支持）。`onMounted(refreshLauncherStatus)`。

- [x] **Step 4: SettingsView 移除启动器分节**

删除「启动器搜索」分节模板与相关脚本（改用 `useLauncherSettings` 后本页不再需要）、`LauncherStatus` 导入；快捷键录制只保留面板一项。

- [x] **Step 5: 验证并提交**

```bash
pnpm typecheck
git add src/composables/useLauncherSearch.ts src/composables/useLauncherSettings.ts src/windows/Main/LauncherPageView.vue src/windows/Main/SettingsView.vue
git commit -m "feat(main): 启动台页（内嵌搜索 + 启动器设置迁入 + 呼出浮窗）"
```

---

## Task 10: 灵动岛页与设置页重排

**Files:**
- Modify: `src/windows/Main/IslandPageView.vue`、`src/windows/Main/SettingsView.vue`

- [x] **Step 1: IslandPageView.vue**

脚本：`useSettings`、`useTodos`、`pickTodayTodos`、`formatClock`（`@/utils/datetime`）、`isOverdue`；`first = computed(() => pickTodayTodos(todos.value)[0] ?? null)`；`count = pickTodayTodos(...).length`；灵动岛设置逻辑（`ISLAND_LIMITS / clampNumber / patchIslandNumber / enabledIslandPluginIds / toggleIslandPlugin`）从 SettingsView 搬入；`patch` 同设置页写法。

模板：

```vue
<div class="archive-page">
  <div class="page-title">🏝️ 灵动岛</div>
  <div class="stats-legend">主屏顶部中央胶囊：轮播当日待办，悬停查看详情，点击唤出面板到待办页</div>
  <div class="island-preview-wrap">
    <div class="island-preview">
      <div class="di-item island-preview-item">
        <template v-if="first">
          <span class="di-dot" :class="first.priority" />
          <span class="di-text">{{ first.content }}</span>
          <span class="di-time" :class="{ ovd: isOverdue(first) }">{{ formatClock(first.due_at) }}</span>
        </template>
        <template v-else>
          <span class="di-dot idle" /><span class="di-text dim">今日待办已全部完成 🎉</span>
        </template>
      </div>
    </div>
    <div class="island-preview-hint">预览与屏幕顶部胶囊同源渲染（展示当前播放的第一条）；实际胶囊位于屏幕顶部中央</div>
    <div class="island-page-stat">当前播放 {{ count }} 条待办 · 每条停留 {{ settings.island_cycle_seconds }} 秒{{ settings.island_enabled ? '' : ' · 灵动岛已停用' }}{{ settings.island_click_through ? ' · 点击穿透' : '' }}</div>
  </div>
  <div class="page-title page-sub-title">⚙️ 灵动岛设置</div>
  <div class="settings-body"> …（启用 / 轮播间隔 / 透明度 / 点击穿透 / 宽度 / 高度 / 插件，从 SettingsView 原样搬入）… </div>
</div>
```

- [x] **Step 2: SettingsView 重排**

删除「灵动岛」分节；`settings-body` 行序：失焦自动收起 → 粘贴板保留天数 → 开机静默自启动 → 全局快捷键 → 备注展示样式 → 主题 → 面板唤出位置 → 窗口毛玻璃 → 玻璃质感 → 数据目录 → 面板插件 → 邮件提醒。头注释更新。

- [x] **Step 3: 验证并提交**

```bash
pnpm typecheck && pnpm exec vite build 2>&1 | grep -E "built in|error"
git add src/windows/Main/IslandPageView.vue src/windows/Main/SettingsView.vue
git commit -m "feat(main): 灵动岛页（预览 + 灵动岛设置迁入）与偏好设置页行序对齐原型"
```

---

## Task 11: 实机验收与记录

**Files:**
- Create: `docs/superpowers/plans/2026-09-10-prototype-realign-phase2-acceptance.md`
- Modify: spec §9；必要时 `src/styles/extensions.css`（新开分节注明阶段）

- [x] **Step 1**（打字机已跑，深色与部分条目未跑，见验收记录 §6）：`pnpm tauri:dev`（后台，沿用 `.tmp/win.ps1` 驱动），按 spec §6 十一项在打字机 / 深色各跑一遍并截图。
- [x] **Step 2**：入场 scale 是否露边、`main-*.js` 体积对比、补丁清单回填 spec §9。
- [x] **Step 3**：全部静态检查 `pnpm sync:styles --check && pnpm typecheck && pnpm test && cargo test`，写验收记录，提交 `docs(design): 阶段二实机验收记录与结论回填`。

---

## 自检记录

- **Spec 覆盖**：§3 差异表 1–24 → Task 2（1）、5（2–6）、6（7）、7（8, 9, 11, 12, 14）、1（9, 13 的函数）、8（13, 16, 17, 18, 20）、4（15, 16, 19, 20, 24）、9（22）、10（21, 23）；D11 → Task 2；后端 → Task 3；测试 → Task 1；验收 → Task 11。
- **类型一致性**：`HeatLevel`、`buildMonthGrid/buildRangeGrid` 返回结构在 Task 1/2/4/8 一致；`searchTodos` 返回 `{ nodes, forceExpand, hitIds }` 与 Task 7 使用一致；`popIn` 在 Task 2 定义、Task 4 使用；`api.launcher.show` 在 Task 3 定义、Task 9 使用；`useLauncherSettings(patch)` 在 Task 9 定义与使用。
- **占位扫描**：Task 4 的两个占位视图在 Task 9/10 被完整实现；无 TBD。
