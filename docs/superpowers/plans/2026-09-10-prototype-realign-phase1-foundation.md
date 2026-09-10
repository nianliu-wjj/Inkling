# 原型对齐 · 阶段一「基础层」实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把应用的样式地基切换到新原型 `docs/styles.css`：脚本生成四层样式、自建样式收拢到 `extensions.css`、打字机默认主题（含旧库 v5 迁移）、animejs 动效模块替换 gsap、列表错落入场。

**Architecture:** `scripts/sync-prototype-styles.mjs` 按原型分节注释把 `docs/styles.css` 切成 `tokens/base/components/themes` 四个生成文件（勿手改）；项目自有样式全部进 `src/styles/extensions.css`；`src/motion/` 封装 animejs v4，对外只暴露 `enter/exit/staggerIn/pop/crossfade` 预设与 `v-stagger-list` 指令。不改任何窗口的页面结构。

**Tech Stack:** Vue 3.5 `<script setup lang="ts">` · TypeScript 5.9 · Vite 7 · animejs 4.5 · prettier 3 · Node 24（`node:test`）· Tauri 2 / Rust（rusqlite）

**设计文档：** `docs/superpowers/specs/2026-09-10-prototype-realign-phase1-foundation-design.md`（下文简称 spec）

## Global Constraints

- 唯一参考原型：`docs/styles.css`；`doc/` 已被用户删除（工作树中为未提交的删除），历史内容用 `git show ac81857:doc/styles.css` 取。
- 生成层 `src/styles/{tokens,base,components,themes}.css` **永远不手改**；项目自有样式只能进 `src/styles/extensions.css` 及其后各层（window-fit / motion / glass）。
- 加载顺序固定：tokens → base → components → themes → extensions → window-fit → motion → glass。
- 演示脚手架拒绝名单（选择器任一复合选择器命中即删）：`#desktop #menubar #clock #trayIcon #trayMenu #fakeApp #onboarding #demoBar .dot`（精确，允许后接 `.`/`:`/`[`）与前缀 `.tray-icon .fake- .onboard- .demo- .menubar-`。
- 动效令牌以原型为准：`--dur-fast 120ms / --dur-base 180ms / --dur-slow 280ms / --ease-out cubic-bezier(.22,.61,.36,1) / --ease-spring cubic-bezier(.34,1.56,.64,1)`；项目扩展令牌 `--dur-instant 90ms / --ease-in-out / --lift 2px / --stagger 30ms / --glass-blur 24px / --glass-saturate 160% / --glass-border-width 1px` 只在 extensions.css 声明。
- 主题：默认 `typewriter`；顺序与原型 THEMES 逐字一致，末尾追加项目自建 `sepia`，共 32 套；任何主题都写 `data-theme` 属性。
- 动效硬约束（spec §8.3）：`.glass` 只在瞬时入场/收起中加 transform；不用 CSS `animation`+`fill-mode: both` 做入场；只动 transform/opacity；退出时长 ≈ 入场 60–70%；新动效取消同元素旧动效并清内联样式。
- 编码规范：Vue 一律 `<script setup lang="ts">`，禁止 `any`；关键节点走 `src/service/logger.ts` 记日志，禁止裸 `console.log`；Rust 新增 `pub`/`fn` 带 `///` 文档注释；CSS 只用令牌表达颜色（生成层与迁入的旧规则原样保留）。
- 验证：每个任务结束运行 `pnpm typecheck`（零错误）；改动 Rust 时 `cargo check --manifest-path src-tauri/Cargo.toml` 与 `cargo test --manifest-path src-tauri/Cargo.toml`；改动样式时 `pnpm exec vite build`。前端无单元测试框架，前端任务以静态检查 + 实机运行验收；脚本层用 `node --test`。
- 提交：中文提交信息，每个任务一次提交，提交前对应检查项必须通过。

---

## File Structure

```
scripts/
├─ lib/
│   ├─ css-split.mjs          纯函数：parseCss / stringify / splitSections / isDeniedSelector / filterDenied / rootTokenDuplicates
│   └─ css-split.test.mjs     node:test 单元测试
└─ sync-prototype-styles.mjs  CLI：读 docs/styles.css → 生成四层（--check 校验 / --out 指定目录）
src/styles/
├─ tokens.css / base.css / components.css / themes.css   生成层（勿手改）
├─ extensions.css             项目扩展层（新建，手写）
├─ index.ts                   加载顺序（改）
└─ window-fit.css / motion.css / glass.css / island.css / launcher.css / mindmap*.css   不改
src/motion/
├─ tokens.ts                  读取 CSS 动效令牌 → animejs 参数；reduced-motion 归零
├─ presets.ts                 enter / exit / staggerIn / pop / crossfade（Promise<boolean>）
├─ directive.ts               vStaggerList：容器内新出现的直接子元素错落入场
└─ index.ts                   统一出口
src/constants/themes.ts       主题清单重排 + typewriter（改）
src/composables/useTheme.ts   任何主题都写 data-theme（改）
*.html（9 个入口）            <html data-theme="typewriter">（改）
src/windows/Panel/PanelApp.vue                 gsap → motion（改）
src/windows/Main/{NotesView,ClipsView,DayView}.vue、src/windows/Panel/ClipPage.vue、src/components/card/TodoTree.vue   v-stagger-list（改）
src-tauri/src/data/mod.rs     with_v5 + migrate（改）；src-tauri/src/domain/models.rs 默认主题（改）
```

---

## Task 1: 清理工作树，文档指向新原型

用户已在 IDE 中删除旧原型 `doc/` 并把两份 md 迁到 `docs/`（工作树未提交）。先把这一步作为独立提交落下，再开始改代码，避免后续任务的提交混入无关变更。

**Files:**
- Delete（已在工作树删除）: `doc/*`
- Add（已在工作树新增）: `docs/Inkling 架构设计文档.md`、`docs/文档审查与风险清单.md`
- Modify: `README.md`、`.gitignore`、`.prettierignore`

- [ ] **Step 1: 确认工作树状态与预期一致**

Run: `git status --short`
Expected: 6 行 ` D doc/...`、2 行 `?? docs/*.md`、1 行 `?? .tmp/`。若出现其他改动，先停下向用户确认。

- [ ] **Step 2: 忽略 .tmp/ 临时目录**

在 `.gitignore` 末尾追加一行 `.tmp/`；在 `.prettierignore` 末尾追加一行 `.tmp/`。

- [ ] **Step 3: README 指向 docs/**

把 `README.md` 中的 `doc/index.html` 全部替换为 `docs/index.html`：

Run: `sed -i 's#`doc/index.html`#`docs/index.html`#g; s#doc/index.html#docs/index.html#g' README.md && grep -n "doc/index" README.md`
Expected: 只剩 `docs/index.html` 的匹配行，没有 `doc/index.html`。

- [ ] **Step 4: 提交**

```bash
git add -A doc docs README.md .gitignore .prettierignore
git status --short   # 确认 .tmp/ 未被暂存
git commit -m "chore(docs): 移除旧原型 doc/，架构文档与审查清单迁入 docs/"
```

---

## Task 2: 样式切分纯函数库（TDD）

**Files:**
- Create: `scripts/lib/css-split.mjs`
- Test: `scripts/lib/css-split.test.mjs`
- Modify: `package.json`（新增 `test:scripts` 脚本）

**Interfaces:**
- Produces（Task 3 与 Task 4 使用）:
  - `parseCss(css: string): Node[]`，`Node` 为 `{type:'comment',text}` | `{type:'statement',text}` | `{type:'rule',selectors:string[],body:string}` | `{type:'at',head,children:Node[]}` | `{type:'atraw',head,body}`
  - `stringify(nodes: Node[]): string`
  - `splitSections(nodes, spec: {name: string|null, until: string|null}[]): Record<string, Node[]>`（`name` 为 null 的段落键为 `__drop<i>`）
  - `isDeniedSelector(selector: string): boolean`
  - `filterDenied(nodes): { nodes: Node[], removed: string[] }`
  - `rootTokenDuplicates(nodes): string[]`

- [ ] **Step 1: 写失败的测试**

创建 `scripts/lib/css-split.test.mjs`：

```js
import test from 'node:test'
import assert from 'node:assert/strict'
import {
  parseCss,
  stringify,
  splitSections,
  isDeniedSelector,
  filterDenied,
  rootTokenDuplicates,
} from './css-split.mjs'

test('parseCss：注释 / 规则 / 嵌套 @media / 字符串内的花括号', () => {
  const css = `/* 头 */\n.a, .b { color: red; --x: url("data:a{b}"); }\n@media (x) { .c { top: 0 } }\n@keyframes k { from { opacity: 0 } to { opacity: 1 } }`
  const nodes = parseCss(css)
  assert.equal(nodes.length, 4)
  assert.deepEqual(nodes[0], { type: 'comment', text: '/* 头 */' })
  assert.equal(nodes[1].type, 'rule')
  assert.deepEqual(nodes[1].selectors, ['.a', '.b'])
  assert.equal(nodes[1].body, 'color: red; --x: url("data:a{b}");')
  assert.equal(nodes[2].type, 'at')
  assert.equal(nodes[2].children[0].selectors[0], '.c')
  assert.equal(nodes[3].type, 'atraw')
})

test('stringify 后再 parse 得到同样的结构', () => {
  const css = `.a { color: red }\n@media (x) { .c { top: 0 } }\n@keyframes k { from { opacity: 0 } }`
  const once = parseCss(css)
  const twice = parseCss(stringify(once))
  assert.deepEqual(twice, once)
})

test('isDeniedSelector：任一复合选择器命中拒绝名单即为演示脚手架', () => {
  assert.equal(isDeniedSelector('#demoBar'), true)
  assert.equal(isDeniedSelector('[data-theme="typewriter"] #demoBar'), true)
  assert.equal(isDeniedSelector('#menubar:hover .x'), true)
  assert.equal(isDeniedSelector('.dot.red'), true)
  assert.equal(isDeniedSelector('.onboard-title'), true)
  assert.equal(isDeniedSelector('.fake-app .x'), true)
  assert.equal(isDeniedSelector('.nav-dot.dot-note'), false)
  assert.equal(isDeniedSelector('.page-title'), false)
  assert.equal(isDeniedSelector('#hotzone'), false)
  assert.equal(isDeniedSelector('.di-dot.high'), false)
})

test('filterDenied：逗号列表只删命中项，整条命中则丢弃，递归 @media', () => {
  const nodes = parseCss(`.ix, .onboard-title, #toast { filter: grayscale(1) }\n#demoBar { top: 0 }\n@media (x) { #menubar { a: b } .keep { a: b } }`)
  const { nodes: kept, removed } = filterDenied(nodes)
  assert.equal(kept.length, 2)
  assert.deepEqual(kept[0].selectors, ['.ix', '#toast'])
  assert.equal(kept[1].type, 'at')
  assert.deepEqual(kept[1].children.map((n) => n.selectors[0]), ['.keep'])
  assert.deepEqual(removed, ['.onboard-title', '#demoBar', '#menubar'])
})

test('splitSections：按标题注释切段，标记注释归入下一段，缺标记报错', () => {
  const css = `:root { --a: 1 }\n/* ═══ 全局质量层 ═══ */\n.q { a: b }\n/* ═══ 桌面环境 ═══ */\n#desktop { a: b }\n/* ═══ hotzone ═══ */\n.c { a: b }\n/* ═══ 多主题系统 ═══ */\n[data-theme="x"] { a: b }`
  const spec = [
    { name: 'tokens', until: '═══ 全局质量层' },
    { name: 'base', until: '═══ 桌面环境' },
    { name: null, until: '═══ hotzone ═══' },
    { name: 'components', until: '多主题系统' },
    { name: 'themes', until: null },
  ]
  const s = splitSections(parseCss(css), spec)
  assert.deepEqual(Object.keys(s), ['tokens', 'base', '__drop2', 'components', 'themes'])
  assert.equal(s.tokens.length, 1)
  assert.equal(s.base[0].type, 'comment')
  assert.equal(s.base[1].selectors[0], '.q')
  assert.equal(s.__drop2[1].selectors[0], '#desktop')
  assert.equal(s.components[1].selectors[0], '.c')
  assert.equal(s.themes[1].selectors[0], '[data-theme="x"]')
  assert.throws(() => splitSections(parseCss('.a { b: c }'), spec), /全局质量层/)
})

test('rootTokenDuplicates：:root 内重复定义的 -- 令牌', () => {
  const nodes = parseCss(`:root { --a: 1; --b: url("x;y"); --a: 2; color: red }\n.x { --a: 3 }`)
  assert.deepEqual(rootTokenDuplicates(nodes), ['--a'])
})
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `node --test "scripts/**/*.test.mjs"`
Expected: 失败，错误含 `Cannot find module` … `css-split.mjs`。

- [ ] **Step 3: 实现 css-split.mjs**

创建 `scripts/lib/css-split.mjs`：

```js
/**
 * 原型样式切分与过滤的纯函数库（无 I/O），供 sync-prototype-styles.mjs 与单元测试共用。
 *
 * 取舍：这不是完整的 CSS 解析器，只识别「注释 / 语句 / 规则 / 带子规则的 @ 规则 / 原样保留的 @ 规则」
 * 五类顶层结构，规则体按原文保留，足以完成分节、按选择器过滤与令牌唯一性检查。
 */

/** 从 index（指向引号）跳过字符串字面量，返回结束引号之后的索引；支持反斜杠转义。 */
function skipString(text, i) {
  const quote = text[i]
  i++
  while (i < text.length) {
    if (text[i] === '\\') {
      i += 2
      continue
    }
    if (text[i] === quote) return i + 1
    i++
  }
  return i
}

/** 从 index（指向 '{'）读到配对的 '}'，返回 [内部文本, '}' 之后的索引]。跳过字符串与注释。 */
function readBlock(css, i) {
  let depth = 0
  const start = i
  while (i < css.length) {
    const ch = css[i]
    if (ch === '"' || ch === "'") {
      i = skipString(css, i)
      continue
    }
    if (css.startsWith('/*', i)) {
      const end = css.indexOf('*/', i)
      i = end < 0 ? css.length : end + 2
      continue
    }
    if (ch === '{') depth++
    else if (ch === '}') {
      depth--
      if (depth === 0) return [css.slice(start + 1, i), i + 1]
    }
    i++
  }
  throw new Error(`CSS 花括号不配对（起始偏移 ${start}）`)
}

/** 在深度 0 处按分隔符切分（跳过字符串、括号与方括号内部）。 */
function splitTopLevel(text, isSeparator) {
  const out = []
  let depth = 0
  let cur = ''
  for (let i = 0; i < text.length; i++) {
    const ch = text[i]
    if (ch === '"' || ch === "'") {
      const end = skipString(text, i)
      cur += text.slice(i, end)
      i = end - 1
      continue
    }
    if (ch === '(' || ch === '[') depth++
    else if (ch === ')' || ch === ']') depth--
    if (depth === 0 && isSeparator(ch)) {
      out.push(cur)
      cur = ''
    } else cur += ch
  }
  out.push(cur)
  return out
}

/** 把选择器列表文本切成规范化的选择器数组（逗号分隔，空白折叠）。 */
export function splitSelectorList(head) {
  return splitTopLevel(head, (ch) => ch === ',')
    .map((s) => s.replace(/\s+/g, ' ').trim())
    .filter(Boolean)
}

/** 把单个选择器切成复合选择器（按空白与 > + ~ 组合符切分）。 */
export function compoundsOf(selector) {
  return splitTopLevel(selector, (ch) => /[\s>+~]/.test(ch))
    .map((s) => s.trim())
    .filter(Boolean)
}

const BLOCK_AT_RULES = /^@(media|supports|container|layer)\b/

/** 解析 CSS 文本为节点数组。 */
export function parseCss(css) {
  const nodes = []
  let i = 0
  const n = css.length
  while (i < n) {
    if (/\s/.test(css[i])) {
      i++
      continue
    }
    if (css.startsWith('/*', i)) {
      const end = css.indexOf('*/', i)
      const stop = end < 0 ? n : end + 2
      nodes.push({ type: 'comment', text: css.slice(i, stop) })
      i = stop
      continue
    }
    // 读到深度 0 的 '{' 或 ';' 为止作为头部
    const start = i
    let depth = 0
    while (i < n) {
      const ch = css[i]
      if (ch === '"' || ch === "'") {
        i = skipString(css, i)
        continue
      }
      if (ch === '(') depth++
      else if (ch === ')') depth--
      else if (depth === 0 && (ch === '{' || ch === ';')) break
      i++
    }
    const head = css.slice(start, i).trim()
    if (i >= n) {
      if (head) nodes.push({ type: 'statement', text: head })
      break
    }
    if (css[i] === ';') {
      i++
      if (head) nodes.push({ type: 'statement', text: `${head};` })
      continue
    }
    const [body, next] = readBlock(css, i)
    i = next
    if (head.startsWith('@')) {
      if (BLOCK_AT_RULES.test(head)) nodes.push({ type: 'at', head, children: parseCss(body) })
      else nodes.push({ type: 'atraw', head, body: body.trim() })
    } else {
      nodes.push({ type: 'rule', selectors: splitSelectorList(head), body: body.trim() })
    }
  }
  return nodes
}

/** 节点数组序列化回 CSS 文本（不做美化，交给 prettier）。 */
export function stringify(nodes) {
  let out = ''
  for (const node of nodes) {
    switch (node.type) {
      case 'comment':
      case 'statement':
        out += `${node.text}\n`
        break
      case 'rule':
        out += `${node.selectors.join(',\n')} {\n${node.body}\n}\n`
        break
      case 'at':
        out += `${node.head} {\n${stringify(node.children)}}\n`
        break
      case 'atraw':
        out += `${node.head} {\n${node.body}\n}\n`
        break
      default:
        throw new Error(`未知节点类型 ${String(node.type)}`)
    }
  }
  return out
}

/**
 * 按标题注释把顶层节点切段。
 * spec: [{ name, until }]，遇到包含 `until` 文本的注释节点即进入下一段（该注释归入下一段）；
 * name 为 null 的段落键为 `__drop<i>`。最后一段 until 必须为 null。缺任一标记则抛错。
 */
export function splitSections(nodes, spec) {
  const buckets = spec.map(() => [])
  let idx = 0
  for (const node of nodes) {
    const marker = spec[idx].until
    if (marker && node.type === 'comment' && node.text.includes(marker)) idx++
    buckets[idx].push(node)
  }
  if (idx !== spec.length - 1) {
    const missing = spec
      .slice(idx, -1)
      .map((s) => s.until)
      .join('、')
    throw new Error(`原型分节标记缺失或顺序不对：${missing}`)
  }
  return Object.fromEntries(spec.map((s, i) => [s.name ?? `__drop${i}`, buckets[i]]))
}

/** 演示脚手架拒绝名单：精确匹配（允许后接 . : [）与前缀匹配。 */
const DENY_EXACT = ['#desktop', '#menubar', '#clock', '#trayIcon', '#trayMenu', '#fakeApp', '#onboarding', '#demoBar', '.dot']
const DENY_PREFIX = ['.tray-icon', '.fake-', '.onboard-', '.demo-', '.menubar-']

function isDeniedCompound(token) {
  for (const d of DENY_EXACT) {
    if (token === d || token.startsWith(`${d}.`) || token.startsWith(`${d}:`) || token.startsWith(`${d}[`)) return true
  }
  return DENY_PREFIX.some((p) => token.startsWith(p))
}

/** 选择器的任一复合选择器命中拒绝名单即视为演示脚手架。 */
export function isDeniedSelector(selector) {
  return compoundsOf(selector).some(isDeniedCompound)
}

/** 递归过滤：逗号列表只删命中项；整条命中则丢弃；@ 规则内为空则丢弃。返回被删选择器清单。 */
export function filterDenied(nodes, removed = []) {
  const out = []
  for (const node of nodes) {
    if (node.type === 'rule') {
      const keep = node.selectors.filter((s) => {
        const denied = isDeniedSelector(s)
        if (denied) removed.push(s)
        return !denied
      })
      if (keep.length) out.push({ ...node, selectors: keep })
    } else if (node.type === 'at') {
      const children = filterDenied(node.children, removed).nodes
      if (children.length) out.push({ ...node, children })
    } else {
      out.push(node)
    }
  }
  return { nodes: out, removed }
}

/** 规则体切分为声明数组（去注释，跳过字符串与括号内的分号）。 */
function splitDeclarations(body) {
  const stripped = body.replace(/\/\*[\s\S]*?\*\//g, '')
  return splitTopLevel(stripped, (ch) => ch === ';')
    .map((d) => d.trim())
    .filter((d) => d.includes(':'))
}

/** 顶层 `:root` 规则中重复定义的 `--` 令牌名。 */
export function rootTokenDuplicates(nodes) {
  const counts = new Map()
  for (const node of nodes) {
    if (node.type !== 'rule' || !node.selectors.includes(':root')) continue
    for (const decl of splitDeclarations(node.body)) {
      const name = decl.slice(0, decl.indexOf(':')).trim()
      if (name.startsWith('--')) counts.set(name, (counts.get(name) ?? 0) + 1)
    }
  }
  return [...counts].filter(([, c]) => c > 1).map(([name]) => name)
}
```

- [ ] **Step 4: 运行测试，确认通过**

Run: `node --test "scripts/**/*.test.mjs"`
Expected: `# pass 6`、`# fail 0`。

- [ ] **Step 5: 加 npm 脚本并提交**

在 `package.json` 的 `scripts` 中加入 `"test:scripts": "node --test "scripts/**/*.test.mjs""`（放在 `typecheck` 之后）。

```bash
pnpm test:scripts
git add scripts/lib/css-split.mjs scripts/lib/css-split.test.mjs package.json
git commit -m "feat(styles): 原型样式切分纯函数库（分节/演示脚手架过滤/令牌唯一性）"
```

---

## Task 3: 同步脚本 CLI

**Files:**
- Create: `scripts/sync-prototype-styles.mjs`
- Modify: `package.json`（`sync:styles` 脚本）

**Interfaces:**
- Consumes: Task 2 的 `parseCss / splitSections / filterDenied / stringify / rootTokenDuplicates`
- Produces: `pnpm sync:styles`（写入 `src/styles/`）、`pnpm sync:styles --check`（不一致时退出码 1）、`pnpm sync:styles --out <dir>`（写到指定目录，供试运行）

- [ ] **Step 1: 写 CLI**

创建 `scripts/sync-prototype-styles.mjs`：

```js
#!/usr/bin/env node
/**
 * 把 docs/styles.css（唯一样式源）切分为 src/styles 的四个生成层：
 *   tokens.css / base.css / components.css / themes.css
 *
 * 用法：
 *   node scripts/sync-prototype-styles.mjs            生成并写入 src/styles/
 *   node scripts/sync-prototype-styles.mjs --check    只比对，不一致则退出码 1（提交前自检）
 *   node scripts/sync-prototype-styles.mjs --out DIR  写到 DIR（试运行）
 *
 * 规则见 docs/superpowers/specs/2026-09-10-prototype-realign-phase1-foundation-design.md §5。
 */
import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import prettier from 'prettier'
import { filterDenied, parseCss, rootTokenDuplicates, splitSections, stringify } from './lib/css-split.mjs'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const SOURCE = path.join(ROOT, 'docs', 'styles.css')
const DEFAULT_OUT = path.join(ROOT, 'src', 'styles')

/** 分段规则：遇到包含 until 的标题注释即进入下一段；name 为 null 的段落整体丢弃。 */
const SECTIONS = [
  { name: 'tokens', until: '═══ 全局质量层', label: '令牌层：尺度令牌 + 深色基础主题令牌 + 导图语义色 + body/.glass/.btn' },
  { name: 'base', until: '═══ 桌面环境', label: '全局质量层：焦点环 / 禁用态 / 动效降级 / 滚动条 / 选区' },
  { name: null, until: '═══ hotzone ═══', label: '演示脚手架（丢弃）' },
  { name: 'components', until: '多主题系统', label: '组件层：hotzone 至重复提醒菜单' },
  { name: 'themes', until: null, label: '主题层：30 套 [data-theme]（含打字机）' },
]

function countRules(nodes) {
  return nodes.reduce((sum, n) => sum + (n.type === 'rule' ? 1 : n.type === 'at' ? countRules(n.children) : 0), 0)
}

function parseArgs(argv) {
  const check = argv.includes('--check')
  const outIdx = argv.indexOf('--out')
  const out = outIdx >= 0 ? path.resolve(argv[outIdx + 1] ?? '') : DEFAULT_OUT
  if (outIdx >= 0 && !argv[outIdx + 1]) throw new Error('--out 需要目录参数')
  return { check, out }
}

async function main() {
  const { check, out } = parseArgs(process.argv.slice(2))
  const css = fs.readFileSync(SOURCE, 'utf8')
  const hash = crypto.createHash('sha256').update(css).digest('hex')
  const sections = splitSections(parseCss(css), SECTIONS)
  const prettierConfig = (await prettier.resolveConfig(path.join(DEFAULT_OUT, 'tokens.css'))) ?? {}

  const outputs = []
  for (const spec of SECTIONS) {
    if (!spec.name) continue
    const { nodes, removed } = filterDenied(sections[spec.name])
    if (spec.name === 'tokens') {
      const dup = rootTokenDuplicates(nodes)
      if (dup.length) throw new Error(`tokens.css 的 :root 存在重复令牌：${dup.join(', ')}`)
    }
    const header =
      `/* 由 scripts/sync-prototype-styles.mjs 从 docs/styles.css 生成，勿手改。\n` +
      `   重新生成：pnpm sync:styles   校验：pnpm sync:styles --check\n` +
      `   原型 SHA-256：${hash}\n` +
      `   分段：${spec.label} */\n\n`
    const text = await prettier.format(header + stringify(nodes), { ...prettierConfig, parser: 'css' })
    outputs.push({ name: spec.name, file: path.join(out, `${spec.name}.css`), text, rules: countRules(nodes), removed })
  }

  const normalize = (s) => s.replace(/\r\n/g, '\n')
  let dirty = []
  for (const o of outputs) {
    const existing = fs.existsSync(o.file) ? normalize(fs.readFileSync(o.file, 'utf8')) : null
    if (existing !== normalize(o.text)) dirty.push(o.file)
  }

  if (check) {
    if (dirty.length) {
      console.error(`[sync:styles] 以下生成文件与原型不一致，请运行 pnpm sync:styles：\n  ${dirty.join('\n  ')}`)
      process.exit(1)
    }
    console.log('[sync:styles] --check 通过：生成层与 docs/styles.css 一致')
    return
  }

  fs.mkdirSync(out, { recursive: true })
  for (const o of outputs) fs.writeFileSync(o.file, o.text, 'utf8')
  for (const o of outputs) {
    console.log(`[sync:styles] ${o.name}.css：${o.rules} 条规则，剔除演示选择器 ${o.removed.length} 个`)
    if (o.removed.length) console.log(`  ${o.removed.join(' | ')}`)
  }
  console.log(`[sync:styles] 已写入 ${out}（原型 SHA-256 ${hash.slice(0, 12)}…）`)
}

main().catch((error) => {
  console.error(`[sync:styles] 失败：${error instanceof Error ? error.message : String(error)}`)
  process.exit(1)
})
```

- [ ] **Step 2: 加 npm 脚本**

在 `package.json` 的 `scripts` 中加入 `"sync:styles": "node scripts/sync-prototype-styles.mjs"`（放在 `test:scripts` 之后）。

- [ ] **Step 3: 试运行到临时目录并核对**

Run: `pnpm sync:styles --out .tmp/gen && ls .tmp/gen && head -8 .tmp/gen/tokens.css`
Expected: 输出四行 `[sync:styles] xxx.css：N 条规则…`；`.tmp/gen` 含四个文件；tokens.css 前 5 行为生成头（含 SHA-256）。被剔除的选择器清单应恰好包含 `.onboard-title`、`[data-theme="typewriter"] #demoBar`、`[data-theme="typewriter"] #menubar`，以及「桌面环境」段之外不应再出现 `#desktop/#fakeApp/.dot` 等（该段已整段丢弃，不进入统计）。

Run: `grep -c "typewriter" .tmp/gen/themes.css && grep -c "\-\-sp-xs\|\-\-mm-surface:" .tmp/gen/tokens.css && grep -n "focus-visible" .tmp/gen/base.css | head -2`
Expected: 第一个数 > 20（打字机主题块进入 themes.css）；第二个数为 2（尺度令牌与导图令牌进入 tokens.css）；base.css 含 `:focus-visible`。

- [ ] **Step 4: 提交**

```bash
git add scripts/sync-prototype-styles.mjs package.json
git commit -m "feat(styles): 原型样式同步脚本 CLI（生成 / --check / --out）"
```

---

## Task 4: 生成四层样式，自建样式收拢到 extensions.css

这是阶段一最关键的一步：生成层覆盖旧文件的同时，必须把项目自建规则一次性搬进 `extensions.css`，否则应用会立刻丢失感应区指示器、窗口控制按钮、代码块等样式。

**Files:**
- Create: `src/styles/extensions.css`
- Regenerate: `src/styles/tokens.css`、`base.css`、`components.css`、`themes.css`
- Modify: `src/styles/index.ts`

**Interfaces:**
- Consumes: Task 2 的 `parseCss / stringify`（用于抽取脚本）；Task 3 的 `pnpm sync:styles`
- Produces: 全局 CSS 类名契约不变；新增 `.is-animating`（Task 7/9 使用）；扩展令牌 `--dur-instant --ease-in-out --lift --stagger --glass-blur --glass-saturate --glass-border-width`

- [ ] **Step 1: 从提交 ac81857 抽取自建组件规则**

创建一次性脚本 `.tmp/extract-app-only.mjs`（不提交）：

```js
// 从 ac81857 的 components.css 中抽出「所有选择器都不在新原型里」的规则（带前置注释），供 extensions.css 使用。
import { execSync } from 'node:child_process'
import fs from 'node:fs'
import { parseCss, stringify } from '../scripts/lib/css-split.mjs'

const norm = (s) => s.replace(/"/g, "'")
const protoSel = new Set()
const collect = (nodes) => {
  for (const n of nodes) {
    if (n.type === 'rule') n.selectors.forEach((s) => protoSel.add(norm(s)))
    else if (n.type === 'at') collect(n.children)
  }
}
collect(parseCss(fs.readFileSync('docs/styles.css', 'utf8')))

const app = parseCss(execSync('git show ac81857:src/styles/components.css', { encoding: 'utf8' }))
const out = []
let pendingComment = null
for (const n of app) {
  if (n.type === 'comment') {
    pendingComment = n
    continue
  }
  if (n.type === 'rule' && n.selectors.every((s) => !protoSel.has(norm(s)))) {
    if (pendingComment) out.push(pendingComment)
    out.push(n)
  }
  pendingComment = null
}
fs.writeFileSync('.tmp/app-only.css', stringify(out))
console.log('rules:', out.filter((n) => n.type === 'rule').length)
```

Run: `node .tmp/extract-app-only.mjs && grep -c "" .tmp/app-only.css`
Expected: `rules: 95`（89 条纯自建 + 6 条旧原型遗留：`.card-confirm`、`.todo-item:has(.card-confirm) .todo-del`、`#todoEditorModal`、`#todoEditorModal .search-input`、`.nav-dot:hover`、`.nav-dot.active`）。其中两条 `.nav-dot` 规则粘贴后要挪到第 5 节（注明「阶段三迁移后删除」：圆点导航仍是 emoji 字符，需要旧的悬浮/激活态）；另外抽取脚本只取规则不取关键帧，第 6 节末尾需手工补上 `@keyframes hotzoneIndicatorIn` 与 `@keyframes hotzoneDots`（`git show ac81857:src/styles/components.css` 中搜索这两个名字，逐字复制）。若数字不同，打开 `.tmp/app-only.css` 逐条核对：允许的内容只能是 spec §6 表格列出的分组，不允许出现 `.nav-dot`、`.tag-chip`（非 shaking）、`.todo-item`（非 has）这类原型同名规则。

- [ ] **Step 2: 写 extensions.css 的手写部分**

创建 `src/styles/extensions.css`，内容为下面的手写部分 + 第 3 步粘贴的抽取内容 + 第 4 步粘贴的棕褐主题：

```css
/* ═══════════════════════════════════════════════════════════════════════
 * Inkling 项目扩展层（手写）
 *
 * 生成层（tokens / base / components / themes）由 scripts/sync-prototype-styles.mjs 从
 * docs/styles.css 生成、勿手改；一切项目自有样式只能写在本文件及其后各层（window-fit / motion / glass）。
 * 加载顺序：themes 之后、window-fit 之前（见 styles/index.ts）。
 *
 * 分节头注明来源与迁入日期；标注「阶段 N 迁移后删除」的分节是过渡桥接，
 * 对应阶段按新原型重构组件后整节删除。
 * ═══════════════════════════════════════════════════════════════════════ */

/* ═══ 1. 扩展令牌（迁自 tokens.css@ac81857，2026-09-10） ═══
 * 原型只定义 --dur-fast/base/slow 与 --ease-out/--ease-spring，以下为项目补充：
 *   --dur-instant 按下即时反馈；--ease-in-out 往返型过渡；--lift 卡片悬浮抬升；--stagger 列表逐项延迟；
 *   --glass-* 玻璃质感档位（见 glass.css）。.glass 改为引用玻璃令牌，data-glass 才能覆盖档位。 */
:root {
  --dur-instant: 90ms;
  --ease-in-out: cubic-bezier(0.65, 0, 0.35, 1);
  --lift: 2px;
  --stagger: 30ms;
  --glass-blur: 24px;
  --glass-saturate: 160%;
  --glass-border-width: 1px;
}
.glass {
  backdrop-filter: blur(var(--glass-blur)) saturate(var(--glass-saturate));
  -webkit-backdrop-filter: blur(var(--glass-blur)) saturate(var(--glass-saturate));
  border-width: var(--glass-border-width);
}
/* 尊重系统「减少动态效果」：扩展令牌同样归零（原型令牌由 base.css 的通配规则兜底；
   JS 动效由 src/motion/tokens.ts 读取 matchMedia 后自行归零）。 */
@media (prefers-reduced-motion: reduce) {
  :root {
    --dur-instant: 0.01ms;
    --stagger: 0ms;
    --lift: 0px;
  }
}

/* ═══ 2. 交互修正与窗口基础（迁自 base.css@ac81857，2026-09-10） ═══ */
/* tokens.css 的 `* { user-select: none }` 会让输入区无法选中文本，
   此处对所有可编辑区域恢复，保证笔记编辑器与输入框可正常选词。 */
input,
textarea,
[contenteditable='true'],
.ProseMirror,
.cm-editor {
  user-select: text;
  -webkit-user-select: text;
}
/* 无边框窗口：标题栏整体可拖拽，但其中的按钮需排除，否则点不动。 */
[data-tauri-drag-region] {
  app-region: drag;
  -webkit-app-region: drag;
}
[data-tauri-drag-region] button,
[data-tauri-drag-region] input,
[data-tauri-drag-region] .no-drag {
  app-region: no-drag;
  -webkit-app-region: no-drag;
}
/* 主窗口「毛玻璃」开关关闭态：把玻璃令牌降级为不透明实色。
   .glass 与全部组件无需改动即可退化为实心卡片，32 套主题同样生效。
   开关实现见 src/windows/Main/SettingsView.vue 与 ipc 命令 set_main_acrylic。 */
:root[data-acrylic='off'] {
  --glass-bg: var(--bg-deep);
  --glass-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
}
:root[data-acrylic='off'] .glass {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
/* 应用根节点铺满窗口；透明窗口下由 .glass 自身绘制背景。 */
#app {
  height: 100vh;
  overflow: hidden;
}

/* ═══ 3. 原型同名规则的项目补充声明（迁自 components.css@ac81857，2026-09-10） ═══
 * 原型同名规则由生成层提供，这里只写项目追加的声明，不重复原型的值。 */
/* 自绘窗口控制按钮的悬浮层不被标题栏裁切 */
.window-titlebar {
  overflow: visible;
}
/* 待办编辑栅格内的控件铺满且可收缩 */
.te-field input,
.te-field select {
  width: 100%;
  min-width: 0;
}
/* 优先级菜单项：点击区 ≥ 32px，勾选标记以此为定位锚点 */
.prio-opt {
  min-height: 32px;
  position: relative;
  outline: none;
}

/* ═══ 4. 错落入场动画态（新增，2026-09-10） ═══
 * src/motion 的预设在动画期间给元素加 .is-animating：打字机主题的卡片自带 transform 过渡，
 * 若不关闭，JS 逐帧写入的 transform 会被 CSS transition 再插值一次，形成拖影。 */
.is-animating {
  transition: none !important;
}

/* ═══ 5. 过渡桥接：旧原型规则（阶段四迁移后整节删除） ═══
 * ConfirmPopover / TodoEditorModal 目前仍按旧原型结构实现（卡片上方行内确认框、居中遮罩弹窗）；
 * 新原型改为全局锚定浮层 #cardConfirm 与锚定卡片右侧的 #todoEditorPanel，
 * 生成层因此不再提供下列规则或改变了 #todoEditorOverlay 的布局。阶段四对齐后删除本节与第 6 节中
 * 的 .card-confirm / #todoEditorModal 相关规则。 */
@keyframes confirmIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
  }
}
.card-confirm-text {
  flex: 1;
  font-size: 12px;
  color: var(--red-soft);
  text-align: left;
}
#todoEditorOverlay {
  background: rgba(8, 10, 18, 0.45);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ═══ 6. 自建组件样式（迁自 components.css@ac81857，2026-09-10，由 .tmp/extract-app-only.mjs 抽取） ═══ */
```

- [ ] **Step 3: 粘贴抽取内容**

把 `.tmp/app-only.css` 的全部内容追加到第 6 节标题之后。

Run: `cat .tmp/app-only.css >> src/styles/extensions.css && grep -c "^\.\|^#\|^:root\|^\[" src/styles/extensions.css`
Expected: 数字 ≥ 100（手写规则 + 93 条抽取规则的选择器行）。

- [ ] **Step 4: 追加棕褐主题**

```bash
printf '\n/* ═══ 7. 棕褐主题（项目自建，迁自 themes.css@ac81857，2026-09-10） ═══ */\n' >> src/styles/extensions.css
git show ac81857:src/styles/themes.css | sed -n '1085,1120p' >> src/styles/extensions.css
tail -5 src/styles/extensions.css
```
Expected: 末尾为棕褐主题块的收尾 `}`；上方可见 `/* 棕褐（参考 images/4.jpg…) */` 与 `:root[data-theme='sepia'] {`。

- [ ] **Step 5: 生成四层并格式化扩展层**

```bash
pnpm sync:styles
pnpm exec prettier --write src/styles/extensions.css
```
Expected: 四行 `[sync:styles] …` 输出；`git status --short` 显示 `M src/styles/{tokens,base,components,themes}.css` 与 `?? src/styles/extensions.css`。

- [ ] **Step 6: 调整加载顺序**

把 `src/styles/index.ts` 改为：

```ts
/**
 * 全局样式入口。
 *
 * 加载顺序不可调换：
 *   tokens（生成：尺度令牌 + 深色基础主题令牌 + 基元）
 *   → base（生成：全局质量层）
 *   → components（生成：原型组件样式）
 *   → themes（生成：30 套 [data-theme] 覆盖，必须压过默认值）
 *   → extensions（手写：项目扩展令牌 / 交互修正 / 自建组件 / 过渡桥接 / 棕褐主题，必须在 themes 之后）
 *   → window-fit（手写：原型浮层定位 → 真实窗口的适配）
 *   → motion（手写：动效层，覆盖组件里硬编码的 transition）
 *   → glass（手写：3 档 [data-glass] 质感覆盖，必须在 themes 之后才能压过主题的阴影）
 *
 * 生成层由 scripts/sync-prototype-styles.mjs 从 docs/styles.css 生成，勿手改。
 * 所有窗口入口统一引入本文件。
 */
import './tokens.css'
import './base.css'
import './components.css'
import './themes.css'
import './extensions.css'
import './window-fit.css'
import './motion.css'
import './glass.css'
```

- [ ] **Step 7: 选择器覆盖核对**

创建一次性脚本 `.tmp/coverage.mjs`（不提交）：

```js
// 新原型的每个选择器都应出现在生成层中（演示脚手架段与被剔除的演示选择器除外）；
// 属性选择器按 prettier 风格统一引号后比较。
import fs from 'node:fs'
import { parseCss, isDeniedSelector, splitSections } from '../scripts/lib/css-split.mjs'
const norm = (x) => x.replace(/"/g, "'").replace(/\[([\w-]+)=([\w-]+)\]/g, "[$1='$2']")
const sels = (nodes) => {
  const s = new Set()
  const walk = (ns) => ns.forEach((n) => (n.type === 'rule' ? n.selectors.forEach((x) => s.add(norm(x))) : n.type === 'at' ? walk(n.children) : null))
  walk(nodes)
  return s
}
const SPEC = [
  { name: 'tokens', until: '═══ 全局质量层' },
  { name: 'base', until: '═══ 桌面环境' },
  { name: null, until: '═══ hotzone ═══' },
  { name: 'components', until: '多主题系统' },
  { name: 'themes', until: null },
]
const sections = splitSections(parseCss(fs.readFileSync('docs/styles.css', 'utf8')), SPEC)
const dropped = sels(sections.__drop2)
const proto = new Set()
for (const k of ['tokens', 'base', 'components', 'themes']) sels(sections[k]).forEach((x) => proto.add(x))
const generated = new Set()
for (const f of ['tokens', 'base', 'components', 'themes']) sels(parseCss(fs.readFileSync(`src/styles/${f}.css`, 'utf8'))).forEach((x) => generated.add(x))
const missing = [...proto].filter((x) => !generated.has(x) && !isDeniedSelector(x) && !dropped.has(x))
console.log('新原型选择器在生成层缺失：', missing.length, missing.slice(0, 10))
const ext = sels(parseCss(fs.readFileSync('src/styles/extensions.css', 'utf8')))
console.log('扩展层与原型同名的选择器：', [...ext].filter((x) => proto.has(x)))
```

Run: `node .tmp/coverage.mjs`
Expected: 第一行 `缺失： 0 []`（演示脚手架段 `#menubar/.context-menu/.menu-*` 等整段丢弃、属性选择器引号差异均已在脚本内排除）；第二行只含 `:root`、`.glass`、`.window-titlebar`、`.te-field input`、`.te-field select`、`.prio-opt`、`.card-confirm-text`、`#todoEditorOverlay`。出现其他同名项说明抽取多带了原型规则，回到第 1 步核对。

- [ ] **Step 8: 静态检查与构建**

```bash
pnpm sync:styles --check
pnpm typecheck
pnpm exec vite build
```
Expected: `--check 通过`；typecheck 零错误；vite build 成功（CSS 语法错误会在此暴露）。

- [ ] **Step 9: 提交**

```bash
git add src/styles/tokens.css src/styles/base.css src/styles/components.css src/styles/themes.css src/styles/extensions.css src/styles/index.ts
git commit -m "feat(styles): 由新原型生成四层样式，自建样式收拢到 extensions.css"
```

---

## Task 5: 打字机默认主题与主题清单（前端）

**Files:**
- Modify: `src/constants/themes.ts`、`src/composables/useTheme.ts`
- Modify: `index.html panel.html pinned.html reminder.html hotzone.html island.html launcher.html editor.html mindmap.html`

**Interfaces:**
- Produces: `themes`（32 项，`typewriter` 第一）、`DEFAULT_THEME = 'typewriter'`；`useTheme().applyTheme(key)` 行为不变但任何主题都写 `data-theme`

- [ ] **Step 1: 重写主题清单**

把 `src/constants/themes.ts` 整体替换为：

```ts
/**
 * 主题清单。
 *
 * 32 套主题：默认 `typewriter`（打字机）与其余 29 套原型主题定义在 styles/themes.css 的
 * :root[data-theme="..."]；`dark` 是 styles/tokens.css 的 :root 基础令牌（设 data-theme="dark"
 * 时没有对应块，直接落到基础令牌）；`sepia`（棕褐）为本项目按 images/4.jpg 新增，
 * 定义在 styles/extensions.css。色点用于偏好设置页的主题下拉预览（样式 .theme-dots）。
 *
 * 数据来源：docs/app.js 的 THEMES 常量，顺序与色点逐字一致；sepia 追加在末尾。
 */

/** 单套主题的展示信息。 */
export interface ThemeOption {
  /** 主题标识，对应 CSS 的 data-theme 值。 */
  key: string
  /** 中文展示名。 */
  label: string
  /** 四枚预览色点：背景 / 主强调 / 次强调 / 第三强调。 */
  dots: readonly [string, string, string, string]
}

export const themes: readonly ThemeOption[] = [
  { key: 'typewriter', label: '打字机', dots: ['#ece7db', '#c62828', '#1a1a1a', '#2e7d32'] },
  { key: 'dark', label: '深色', dots: ['#1e2232', '#6c8cff', '#ffd76e', '#7ee0a8'] },
  { key: 'light', label: '浅色', dots: ['#f2f5fc', '#4c68e0', '#b8860b', '#12805c'] },
  { key: 'cupcake', label: '纸杯蛋糕', dots: ['#fdf0f4', '#e56ba5', '#8fd3c7', '#f5c26b'] },
  { key: 'bumblebee', label: '大黄蜂', dots: ['#f6f3ea', '#a3860b', '#2b2b2b', '#8a8a8a'] },
  { key: 'emerald', label: '翡翠绿', dots: ['#e9f5ee', '#0f8f5f', '#2f7d6d', '#c2654a'] },
  { key: 'business', label: '商务蓝', dots: ['#e8edf6', '#2752c4', '#5b7bd5', '#94a3c4'] },
  { key: 'neon', label: '霓虹未来', dots: ['#160b2e', '#22d3ee', '#e879f9', '#a3e635'] },
  { key: 'retro', label: '复古', dots: ['#f0e4cc', '#b4713a', '#7a5c2e', '#c9a86a'] },
  { key: 'romance', label: '浪漫', dots: ['#fbeaf1', '#d2568f', '#9f7aea', '#f4a7c3'] },
  { key: 'halloween', label: '万圣节', dots: ['#1a1220', '#ff7a1a', '#8a2be2', '#5c4033'] },
  { key: 'fantasy', label: '奇幻', dots: ['#1c1030', '#c084fc', '#fcd34d', '#7dd3fc'] },
  { key: 'oled', label: '极黑', dots: ['#050505', '#4d8dff', '#f5c518', '#66bb6a'] },
  { key: 'luxury', label: '奢华', dots: ['#14100a', '#d4af37', '#e8c96a', '#a8c686'] },
  { key: 'dracula', label: '德古拉', dots: ['#282a36', '#bd93f9', '#50fa7b', '#ff79c6'] },
  { key: 'print', label: '印刷色', dots: ['#f5f5f1', '#1f1f24', '#0e7490', '#c0392b'] },
  { key: 'autumn', label: '秋日', dots: ['#f7ecd9', '#c6612c', '#7d8a2c', '#b0527e'] },
  { key: 'businessgray', label: '商务灰', dots: ['#eef0f2', '#495057', '#4a7c59', '#a35376'] },
  { key: 'psychedelic', label: '迷幻', dots: ['#12002e', '#ff3ec8', '#3ee8ff', '#ffe14d'] },
  { key: 'lemon', label: '柠檬', dots: ['#fbf8d8', '#9b7900', '#5c8a2c', '#b3400c'] },
  { key: 'night', label: '夜色', dots: ['#0b1026', '#5c7cfa', '#fbbf24', '#7dd3fc'] },
  { key: 'coffee', label: '咖啡', dots: ['#1b1210', '#c08552', '#ddb271', '#9caf88'] },
  { key: 'winter', label: '冬日', dots: ['#eef4fa', '#4a7fb5', '#4d8a6a', '#c05b6a'] },
  { key: 'abyss', label: '深渊', dots: ['#020c14', '#0e9db8', '#34c98e', '#e0b84d'] },
  { key: 'aqua', label: '水色', dots: ['#e4f6f8', '#0891b2', '#2c8a6b', '#3a6ea5'] },
  { key: 'latte', label: '焦糖拿铁', dots: ['#f2e7d8', '#a0673c', '#6b8a4a', '#b06a8a'] },
  { key: 'dim', label: '暗色', dots: ['#17181c', '#7c8cf8', '#d9b44a', '#6fbf8f'] },
  { key: 'aurora', label: '北极光', dots: ['#06131a', '#34d399', '#67e8f9', '#fbbf24'] },
  { key: 'pastel', label: '粉彩', dots: ['#fdf0f7', '#9d7bd8', '#6bbf95', '#d67ba0'] },
  { key: 'sunset', label: '日落', dots: ['#1f1030', '#fb923c', '#fde047', '#f472b6'] },
  { key: 'wireframe', label: '线框', dots: ['#f8f8f6', '#52525b', '#4a7c59', '#c04440'] },
  { key: 'sepia', label: '棕褐', dots: ['#332a20', '#b04a42', '#d6a35c', '#a3b083'] },
] as const

/** 默认主题标识（与原型一致：打字机）。 */
export const DEFAULT_THEME = 'typewriter'
```

- [ ] **Step 2: 任何主题都写 data-theme**

在 `src/composables/useTheme.ts` 中，把 `writeToDom` 及其注释替换为：

```ts
/**
 * 把主题写进 DOM。
 *
 * 与原型 `document.documentElement.dataset.theme = t.id` 一致：任何主题（含 dark）都写
 * data-theme 属性。dark 没有对应的 [data-theme] 块，直接落到 tokens.css 的 :root 基础令牌；
 * 其余 31 套由 themes.css / extensions.css 的 :root[data-theme="..."] 覆盖。
 */
function writeToDom(key: string): void {
  document.documentElement.setAttribute('data-theme', key)
}
```

同时把该文件头部注释里的「窗口启动瞬间抢先上主题，避免默认深色闪一下」改为「避免入口 html 上的默认打字机闪一下再切换」。

- [ ] **Step 3: 入口 html 首帧主题**

Run:
```bash
sed -i 's#<html lang="zh-CN">#<html lang="zh-CN" data-theme="typewriter">#' index.html panel.html pinned.html reminder.html hotzone.html island.html launcher.html editor.html mindmap.html
grep -c 'data-theme="typewriter"' index.html panel.html pinned.html reminder.html hotzone.html island.html launcher.html editor.html mindmap.html
```
Expected: 九个文件各输出 `:1`。

- [ ] **Step 4: 排查前端其他写死的默认值**

Run: `grep -rn "'dark'" src --include=*.ts --include=*.vue`
Expected: 无结果。若有（例如 useData 的设置默认值），改为从 `@/constants/themes` 导入 `DEFAULT_THEME` 使用。

- [ ] **Step 5: 验证并提交**

```bash
pnpm typecheck
git add src/constants/themes.ts src/composables/useTheme.ts index.html panel.html pinned.html reminder.html hotzone.html island.html launcher.html editor.html mindmap.html
git commit -m "feat(theme): 打字机为默认主题，主题清单顺序对齐原型，入口首帧主题"
```

---

## Task 6: 后端默认主题与 v5 迁移（TDD）

**Files:**
- Modify: `src-tauri/src/data/mod.rs`（`migrate()`、新增 `with_v5`、tests）
- Modify: `src-tauri/src/domain/models.rs:227`（`Settings::default().theme`）

**Interfaces:**
- Produces: `PRAGMA user_version = 5`；`Store::with_v5(&self) -> Result<(), String>`

- [ ] **Step 1: 写失败的测试**

在 `src-tauri/src/data/mod.rs` 的 `mod tests` 末尾（`v3_migration_normalizes_existing_reminders` 之后）加入：

```rust
    /// 建一个只有 settings 表的库，并按需写入 theme 行。
    fn settings_db(theme: Option<&str>) -> rusqlite::Connection {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .unwrap();
        if let Some(theme) = theme {
            db.execute("INSERT INTO settings(key, value) VALUES('theme', ?1)", [theme])
                .unwrap();
        }
        db
    }

    fn theme_rows(store: &Store) -> Vec<String> {
        let mut stmt = store
            .db
            .prepare("SELECT value FROM settings WHERE key='theme'")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn v5_migration_switches_old_default_dark_to_typewriter() {
        let store = Store {
            db: settings_db(Some("dark")),
            data_dir: std::path::PathBuf::from("."),
        };
        store.with_v5().unwrap();
        assert_eq!(theme_rows(&store), vec!["typewriter".to_string()]);
    }

    #[test]
    fn v5_migration_keeps_other_themes_untouched() {
        let store = Store {
            db: settings_db(Some("neon")),
            data_dir: std::path::PathBuf::from("."),
        };
        store.with_v5().unwrap();
        assert_eq!(theme_rows(&store), vec!["neon".to_string()]);
    }

    #[test]
    fn v5_migration_is_idempotent_and_never_inserts_rows() {
        // 已迁移过的库再跑一次：结果不变、不报错。
        let store = Store {
            db: settings_db(Some("dark")),
            data_dir: std::path::PathBuf::from("."),
        };
        store.with_v5().unwrap();
        store.with_v5().unwrap();
        assert_eq!(theme_rows(&store), vec!["typewriter".to_string()]);

        // 从未写过 theme 行的库：迁移不能凭空插入行，默认值交给 Settings::default()。
        let empty = Store {
            db: settings_db(None),
            data_dir: std::path::PathBuf::from("."),
        };
        empty.with_v5().unwrap();
        assert!(theme_rows(&empty).is_empty());
    }

    #[test]
    fn default_settings_theme_is_typewriter() {
        assert_eq!(crate::domain::models::Settings::default().theme, "typewriter");
    }
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml v5_migration 2>&1 | tail -20`
Expected: 编译错误 `no method named `with_v5``。

- [ ] **Step 3: 实现 with_v5 与默认值**

在 `src-tauri/src/data/mod.rs` 的 `migrate()` 中 `if version < 4 { … }` 之后、`Ok(())` 之前加入：

```rust
        if version < 5 {
            self.with_v5()
                .map_err(|e| format!("数据库迁移到 v5 失败: {e}"))?;
            self.db
                .pragma_update(None, "user_version", 5)
                .map_err(db_err)?;
        }
```

在 `with_v4` 定义之后加入：

```rust
    /// v5 增量：默认主题由深色改为打字机（对齐新原型 docs/）。
    ///
    /// 用户只要保存过一次设置，settings 里就会存下当时的默认值 dark；与 v4 面板位置迁移
    /// 同一取舍——无法区分「主动选深色」与「保存时带上的默认值」，一律归到新默认，
    /// 主动偏好深色的用户在设置页重选一次即可。从未写过 theme 行的库不插入任何行。
    fn with_v5(&self) -> Result<(), String> {
        let changed = self
            .db
            .execute(
                "UPDATE settings SET value='typewriter' WHERE key='theme' AND value='dark'",
                [],
            )
            .map_err(db_err)?;
        if changed > 0 {
            eprintln!("[data] v5 迁移：主题由 dark 归到新默认 typewriter");
        }
        Ok(())
    }
```

把 `src-tauri/src/domain/models.rs` 中 `theme: "dark".into(),` 改为 `theme: "typewriter".into(),`。

- [ ] **Step 4: 运行全部 Rust 测试**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -15
```
Expected: check 零 error；`test result: ok.`，其中含 4 个新用例；现有 `v4_migration_moves_bottom_panel_back_to_top` 仍通过（它只调用 `with_v4`，主题断言 `dark` 不受影响）。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/data/mod.rs src-tauri/src/domain/models.rs
git commit -m "feat(data): v5 迁移把旧库默认主题 dark 归到打字机，设置默认值同步"
```

---

## Task 7: animejs 动效模块

**Files:**
- Create: `src/motion/tokens.ts`、`src/motion/presets.ts`、`src/motion/index.ts`

**Interfaces:**
- Produces:
  - `readMotionTokens(): MotionTokens`（`{ fast, base, slow, stagger: number; easeOut, easeSpring: EasingParam; reduced: boolean }`）
  - `enter(el: HTMLElement, opts: SlideOptions): Promise<boolean>`、`exit(el, opts): Promise<boolean>`（`SlideOptions = { axis: 'x' | 'y'; distance: number; scale?: number }`；返回 `true` = 播完，`false` = 被新动效打断）
  - `staggerIn(container: HTMLElement, targets: HTMLElement[], opts?: { distance?: number }): Promise<boolean>`
  - `pop(el: HTMLElement): Promise<boolean>`、`crossfade(outEl: HTMLElement, inEl: HTMLElement): Promise<boolean>`

- [ ] **Step 1: tokens.ts**

创建 `src/motion/tokens.ts`：

```ts
import { cubicBezier, type EasingParam } from 'animejs'
import { logger } from '@/service/logger'

/**
 * 运行时动效参数：来自 CSS 动效令牌（tokens.css / extensions.css），JS 动效与 CSS 过渡共用一套节奏。
 * reduced = 系统开启「减少动态效果」，所有时长归零，预设据此直接落到终态、不创建动画。
 */
export interface MotionTokens {
  /** --dur-fast，毫秒。 */
  fast: number
  /** --dur-base，毫秒。 */
  base: number
  /** --dur-slow，毫秒。 */
  slow: number
  /** --stagger，列表逐项延迟，毫秒。 */
  stagger: number
  /** --ease-out。 */
  easeOut: EasingParam
  /** --ease-spring。 */
  easeSpring: EasingParam
  reduced: boolean
}

/** 令牌读取失败时的兜底值（与原型 :root 逐字一致）。 */
const FALLBACK = {
  fast: 120,
  base: 180,
  slow: 280,
  stagger: 30,
  easeOut: cubicBezier(0.22, 0.61, 0.36, 1),
  easeSpring: cubicBezier(0.34, 1.56, 0.64, 1),
} as const

/** 把 "180ms" / ".2s" 解析为毫秒；无法解析返回 fallback。 */
export function parseDuration(raw: string, fallback: number): number {
  const match = raw.trim().match(/^([\d.]+)(ms|s)$/)
  if (!match) return fallback
  const value = Number(match[1])
  if (!Number.isFinite(value)) return fallback
  return match[2] === 's' ? value * 1000 : value
}

/** 把 CSS `cubic-bezier(a,b,c,d)` 转成 animejs 缓动函数；其他写法返回 fallback。 */
export function parseEase(raw: string, fallback: EasingParam): EasingParam {
  const match = raw.trim().match(/^cubic-bezier\(([^)]+)\)$/)
  if (!match) return fallback
  const parts = match[1].split(',').map((p) => Number(p.trim()))
  if (parts.length !== 4 || parts.some((p) => !Number.isFinite(p))) return fallback
  const [x1, y1, x2, y2] = parts as [number, number, number, number]
  return cubicBezier(x1, y1, x2, y2)
}

/** 读取当前文档的动效令牌。每次调用都重新读，主题切换后无需重置。 */
export function readMotionTokens(): MotionTokens {
  const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  const style = getComputedStyle(document.documentElement)
  const read = (name: string): string => style.getPropertyValue(name)
  const tokens: MotionTokens = {
    fast: parseDuration(read('--dur-fast'), FALLBACK.fast),
    base: parseDuration(read('--dur-base'), FALLBACK.base),
    slow: parseDuration(read('--dur-slow'), FALLBACK.slow),
    stagger: parseDuration(read('--stagger'), FALLBACK.stagger),
    easeOut: parseEase(read('--ease-out'), FALLBACK.easeOut),
    easeSpring: parseEase(read('--ease-spring'), FALLBACK.easeSpring),
    reduced,
  }
  if (reduced) {
    logger.debug('motion', '系统要求减少动态效果，JS 动效整体关闭')
    return { ...tokens, fast: 0, base: 0, slow: 0, stagger: 0 }
  }
  return tokens
}
```

- [ ] **Step 2: presets.ts**

创建 `src/motion/presets.ts`：

```ts
import { animate, stagger, type JSAnimation } from 'animejs'
import { logger } from '@/service/logger'
import { readMotionTokens } from './tokens'

/**
 * 动效预设：调用方不接触 animejs API，只拿 Promise<boolean>（true = 播完，false = 被打断）。
 *
 * 硬约束（本项目实测踩坑，见 spec §8.3）：
 * 1. 带 backdrop-filter 的 .glass 只允许在 enter/exit 这类瞬时动效里加 transform，静止态必须无 transform。
 * 2. 入场一律 JS 驱动，不用 CSS animation + fill-mode: both——WebView2 在窗口隐藏后挂起动画，
 *    再显示时可能停在透明态。
 * 3. 只动 transform 与 opacity。
 * 4. 同一元素同一时刻只有一条动画：新预设先取消旧动画并清内联样式。
 */

export type Axis = 'x' | 'y'

export interface SlideOptions {
  axis: Axis
  /** 位移距离（px），正负即方向：入场从 distance 滑到 0，收起从 0 滑到 distance。 */
  distance: number
  /** 入场起始缩放（如 0.97）；不传则不缩放。 */
  scale?: number
}

type AnimateParams = NonNullable<Parameters<typeof animate>[1]>

interface Running {
  animation: JSAnimation
  targets: HTMLElement[]
  settle: (completed: boolean) => void
}

/** 键为「动画的锚点元素」（单元素动效即元素本身，错落动效为容器）。 */
const running = new WeakMap<Element, Running>()

function clearInline(el: HTMLElement): void {
  el.style.transform = ''
  el.style.opacity = ''
  el.classList.remove('is-animating')
}

/** 取消锚点上进行中的动画并复位其目标元素；被取消的 Promise 以 false 结束。 */
function cancelRunning(anchor: Element): void {
  const prev = running.get(anchor)
  if (!prev) return
  running.delete(anchor)
  prev.animation.cancel()
  prev.targets.forEach(clearInline)
  prev.settle(false)
}

/**
 * 通用执行：keepEnd = true 时播完后保留终态内联样式（收起后元素应停在透明态，直到下次入场重置）。
 */
function run(anchor: Element, targets: HTMLElement[], params: AnimateParams, keepEnd = false): Promise<boolean> {
  cancelRunning(anchor)
  targets.forEach((el) => el.classList.add('is-animating'))
  return new Promise<boolean>((resolve) => {
    let record: Running | undefined
    const animation = animate(targets, {
      ...params,
      onComplete: () => {
        if (record && running.get(anchor) === record) running.delete(anchor)
        targets.forEach((el) => (keepEnd ? el.classList.remove('is-animating') : clearInline(el)))
        resolve(true)
      },
    })
    record = { animation, targets, settle: resolve }
    running.set(anchor, record)
  })
}

/** 入场：位移 + 淡入（+ 可选缩放），--dur-slow，outBack(1.6)——与原型 showPanel 一致。 */
export function enter(el: HTMLElement, opts: SlideOptions): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) {
    cancelRunning(el)
    clearInline(el)
    return Promise.resolve(true)
  }
  const prop = opts.axis === 'x' ? 'translateX' : 'translateY'
  logger.debug('motion', `enter ${prop} ${opts.distance}px → 0`)
  return run(el, [el], {
    [prop]: [opts.distance, 0],
    opacity: [0, 1],
    ...(opts.scale === undefined ? {} : { scale: [opts.scale, 1] }),
    duration: tokens.slow,
    ease: 'outBack(1.6)',
  })
}

/** 收起：位移 + 淡出，--dur-base，inQuad——与原型 hidePanel 一致；播完保留透明终态。 */
export function exit(el: HTMLElement, opts: SlideOptions): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) {
    cancelRunning(el)
    el.style.opacity = '0'
    return Promise.resolve(true)
  }
  const prop = opts.axis === 'x' ? 'translateX' : 'translateY'
  logger.debug('motion', `exit ${prop} 0 → ${opts.distance}px`)
  return run(
    el,
    [el],
    {
      [prop]: [0, opts.distance],
      opacity: [1, 0],
      duration: tokens.base,
      ease: 'inQuad',
    },
    true,
  )
}

/**
 * 列表错落入场：targets 逐项延迟 --stagger，位移 8px + 淡入，--dur-base，--ease-out。
 * 先同步把目标置为透明，避免动画首帧前闪现终态。锚点为容器：同一容器新一轮会取消上一轮。
 */
export function staggerIn(container: HTMLElement, targets: HTMLElement[], opts: { distance?: number } = {}): Promise<boolean> {
  const tokens = readMotionTokens()
  if (!targets.length) return Promise.resolve(true)
  if (tokens.reduced) {
    cancelRunning(container)
    targets.forEach(clearInline)
    return Promise.resolve(true)
  }
  cancelRunning(container)
  targets.forEach((el) => (el.style.opacity = '0'))
  return run(container, targets, {
    translateY: [opts.distance ?? 8, 0],
    opacity: [0, 1],
    duration: tokens.base,
    delay: stagger(tokens.stagger),
    ease: tokens.easeOut,
  })
}

/** 按压/勾选回弹：scale .95 → 1，--dur-fast，--ease-spring。 */
export function pop(el: HTMLElement): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) return Promise.resolve(true)
  return run(el, [el], { scale: [0.95, 1], duration: tokens.fast, ease: tokens.easeSpring })
}

/** 同容器内容替换：旧元素淡出（--dur-fast），新元素淡入（--dur-base）。 */
export async function crossfade(outEl: HTMLElement, inEl: HTMLElement): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) return true
  const [a, b] = await Promise.all([
    run(outEl, [outEl], { opacity: [1, 0], duration: tokens.fast, ease: tokens.easeOut }, true),
    run(inEl, [inEl], { opacity: [0, 1], duration: tokens.base, ease: tokens.easeOut }),
  ])
  return a && b
}
```

- [ ] **Step 3: index.ts**

创建 `src/motion/index.ts`：

```ts
/**
 * 动效模块统一出口。调用方只用这里导出的预设与指令，不直接依赖 animejs。
 */
export { readMotionTokens, type MotionTokens } from './tokens'
export { enter, exit, staggerIn, pop, crossfade, type Axis, type SlideOptions } from './presets'
```

（`vStaggerList` 在 Task 9 加入本文件的导出。）

- [ ] **Step 4: 类型检查**

Run: `pnpm typecheck`
Expected: 零错误。若 `'outBack(1.6)'` 或 `delay: stagger(...)` 报类型错误，说明 animejs 类型与预期不符：`ease` 改为 `'outBack(1.6)' as const`，`delay` 保持 `stagger(tokens.stagger)`（`stagger` 返回的函数是 `delay` 的合法类型）。

- [ ] **Step 5: 提交**

```bash
git add src/motion
git commit -m "feat(motion): animejs 动效模块（令牌读取 + enter/exit/staggerIn/pop/crossfade 预设）"
```

---

## Task 8: 面板入场/收起迁移到 animejs，卸载 gsap

**Files:**
- Modify: `src/windows/Panel/PanelApp.vue`（`import gsap`、`hide()`、`playEnter()`、头注释）
- Modify: `package.json`、`pnpm-lock.yaml`（移除 gsap）

**Interfaces:**
- Consumes: Task 7 的 `enter / exit`

- [ ] **Step 1: 替换导入**

把 `src/windows/Panel/PanelApp.vue` 第 2 行 `import gsap from 'gsap'` 删除，并在 `import { MAX_HOTKEY_SLOTS, resolvePlugins } from '@/panel-plugins'` 之后加入：

```ts
import { enter, exit } from '@/motion'
```

- [ ] **Step 2: 改 hide()**

把 `hide()` 中的

```ts
  if (panel.value) {
    await gsap.to(panel.value, {
      [motionAxis()]: motionDistance(12),
      opacity: 0,
      duration: 0.15,
      ease: 'power2.in',
    })
  }
```

替换为：

```ts
  if (panel.value) {
    // 与原型 hidePanel 一致：24px 位移 + 淡出，180ms inQuad。被新的入场打断时不再隐藏窗口。
    const completed = await exit(panel.value, { axis: motionAxis(), distance: motionDistance(24) })
    if (!completed) {
      logger.debug('panel', '收起动画被入场打断，取消隐藏')
      return
    }
  }
```

- [ ] **Step 3: 改 playEnter()**

把 `playEnter()` 及其上方的注释整体替换为：

```ts
/**
 * 入场：与原型 showPanel 一致——30px 位移 + 淡入 + 0.97 起始缩放，280ms outBack(1.6)。
 * 位移轴与方向按面板唤出位置推导（项目扩展，原型只有顶部）。
 * 由 onMounted 与后端 panelShown 事件触发，不依赖 CSS 动画自身的时间线（硬约束见 src/motion/presets.ts）。
 */
function playEnter(): void {
  if (!panel.value) return
  void enter(panel.value, { axis: motionAxis(), distance: motionDistance(30), scale: 0.97 })
}
```

同时把文件头注释中的 `- 滑入 200ms / 滑出 150ms 的弹性过渡（GSAP）；` 改为 `- 入场 280ms outBack / 收起 180ms inQuad 的弹性过渡（animejs，与原型 showPanel/hidePanel 一致）；`。

- [ ] **Step 4: 卸载 gsap 并验证**

```bash
pnpm remove gsap
grep -rn "gsap" src package.json; echo "exit=$?"
pnpm typecheck
```
Expected: grep 无输出且 `exit=1`；typecheck 零错误。

- [ ] **Step 5: 提交**

```bash
git add src/windows/Panel/PanelApp.vue package.json pnpm-lock.yaml
git commit -m "feat(motion): 面板入场收起改用 animejs 并对齐原型时长曲线，卸载 gsap"
```

---

## Task 9: 列表错落入场指令与接入

**Files:**
- Create: `src/motion/directive.ts`
- Modify: `src/motion/index.ts`
- Modify: `src/windows/Main/NotesView.vue`、`src/windows/Main/ClipsView.vue`、`src/windows/Main/DayView.vue`、`src/windows/Panel/ClipPage.vue`、`src/components/card/TodoTree.vue`

**Interfaces:**
- Consumes: Task 7 的 `staggerIn`
- Produces: `vStaggerList: Directive<HTMLElement>`，模板写法 `v-stagger-list`（无参数）；`TodoTree` 的接入同时覆盖主窗口待办页与面板待办页

- [ ] **Step 1: directive.ts**

创建 `src/motion/directive.ts`：

```ts
import type { Directive } from 'vue'
import { logger } from '@/service/logger'
import { staggerIn } from './presets'

/** 每个容器最多错落的项数，其余立即显示（长列表不能让用户等一串入场）。 */
const LIMIT = 12

/** 容器 → 已经出现过的直接子元素。只有首次出现的子元素才入场，删除/重排不重播。 */
const seenByContainer = new WeakMap<HTMLElement, WeakSet<Element>>()

function reveal(container: HTMLElement): void {
  let seen = seenByContainer.get(container)
  if (!seen) {
    seen = new WeakSet()
    seenByContainer.set(container, seen)
  }
  const fresh: HTMLElement[] = []
  for (const child of Array.from(container.children)) {
    if (!(child instanceof HTMLElement) || seen.has(child)) continue
    seen.add(child)
    fresh.push(child)
  }
  if (!fresh.length) return
  logger.debug('motion', `列表新增 ${fresh.length} 项，错落入场前 ${Math.min(fresh.length, LIMIT)} 项`)
  void staggerIn(container, fresh.slice(0, LIMIT))
}

/**
 * v-stagger-list：容器内新出现的直接子元素逐项错落入场。
 *
 * - mounted：首屏全部子元素入场（受 LIMIT 限制）；
 * - updated：宿主组件每次重渲染后检查，只对 Vue 新建的 DOM 节点入场（keyed v-for 复用的节点不动）；
 * - 不用 <TransitionGroup>：它依赖 CSS 类与 fill 态，违反 spec §8.3 硬约束 2。
 */
export const vStaggerList: Directive<HTMLElement> = {
  mounted: reveal,
  updated: reveal,
  beforeUnmount(el) {
    seenByContainer.delete(el)
  },
}
```

- [ ] **Step 2: 导出指令**

在 `src/motion/index.ts` 末尾加入：

```ts
export { vStaggerList } from './directive'
```

- [ ] **Step 3: 接入五处模板**

每个文件在 `<script setup lang="ts">` 的 import 区加入 `import { vStaggerList } from '@/motion'`（`<script setup>` 中以 `v` 开头的导入即可在模板用 `v-stagger-list`），然后改容器：

1. `src/windows/Main/NotesView.vue`：包住 `NoteCard` 列表的 `<div>` 改为 `<div v-stagger-list>`。
2. `src/windows/Main/ClipsView.vue`：`<ul>` 改为 `<ul v-stagger-list>`。
3. `src/windows/Main/DayView.vue`：`day-hint` 下方包住 `day-item` 的 `<div>` 改为 `<div v-stagger-list>`。
4. `src/windows/Panel/ClipPage.vue`：`<ul class="clip-list">` 改为 `<ul v-stagger-list class="clip-list">`。
5. `src/components/card/TodoTree.vue`：`<ul class="todo-list">` 改为 `<ul v-stagger-list class="todo-list">`（只对顶层项与分区标题错落；`.todo-children` 是子级 `<ul>`，不受影响）。

- [ ] **Step 4: 静态检查**

```bash
pnpm typecheck
grep -rn "v-stagger-list" src | wc -l
```
Expected: 零错误；计数为 5。

- [ ] **Step 5: 提交**

```bash
git add src/motion/directive.ts src/motion/index.ts src/windows/Main/NotesView.vue src/windows/Main/ClipsView.vue src/windows/Main/DayView.vue src/windows/Panel/ClipPage.vue src/components/card/TodoTree.vue
git commit -m "feat(motion): 列表错落入场指令 v-stagger-list，接入主窗口与面板六处列表"
```

---

## Task 10: 实机验收与记录

**Files:**
- Modify: `docs/superpowers/specs/2026-09-10-prototype-realign-phase1-foundation-design.md`（§12 实机结论）
- Modify（仅在发现破损时）: `src/styles/extensions.css` 或 `src/styles/window-fit.css`
- Create: `docs/superpowers/plans/2026-09-10-prototype-realign-phase1-acceptance.md`

- [ ] **Step 1: 启动应用与原型**

```bash
pnpm tauri:dev
```
另开浏览器打开 `docs/index.html`（原型演示），用于并排比对。

- [ ] **Step 2: 逐窗口比对（打字机 + 深色各一遍）**

在设置页切换主题，对九个窗口逐一检查并记录到验收文档（每项写「通过 / 问题描述」）：

| # | 窗口 | 检查点 |
|---|---|---|
| 1 | 主窗口 | 米色纸面背景、白卡黑描边、等宽字体、锐利直角（打字机）；深色下毛玻璃与原来一致；侧边栏 / 列表 / 设置页 / 统计页无错位 |
| 2 | 呼出面板 | Ctrl+Shift+Space 呼出 5 次：入场 280ms 弹性、收起 180ms；无残影、无停在透明态；窗口隐藏再显示后入场重播；圆点导航仍可点击切换 |
| 3 | 置顶浮窗 | 铺满窗口、圆角、透明度滑块可用 |
| 4 | 提醒卡片 | 触发一条到点提醒：卡片样式随主题、关闭与稍后提醒可用 |
| 5 | 感应区 | 鼠标触顶：指示器出现、进度条增长（extensions.css 第 6 节生效） |
| 6 | 灵动岛 | 胶囊显示待办轮播、悬停展开（island.css 未受影响） |
| 7 | 启动台 | Alt+Space 呼出：输入框可选中文字、结果列表可上下键选择 |
| 8 | 独立编辑窗口 | 从面板打开待办编辑：居中遮罩弹窗（extensions.css 第 5 节桥接生效）、字段可编辑 |
| 9 | 思维导图窗口 | 新建导图：工具栏、画布、侧栏无错位（mindmap.css 未受影响） |

额外：Tab 键在主窗口巡检一圈，焦点环可见且不与自建焦点样式叠出双环；卡片删除二次确认浮层（`.card-confirm`）正常弹出。

- [ ] **Step 3: 列表错落与 hover 复位**

主窗口切换笔记 / 粘贴板 / 待办 / 日期详情四个视图，面板切换粘贴板 / 待办两页：首次进入逐项入场（≤12 项）；搜索词变化时新出现的项入场、原有项不动；动画结束后悬浮卡片仍有抬升（内联样式已清）；打字机主题下无拖影。

- [ ] **Step 4: reduced-motion 与主题切换**

Windows 设置 → 辅助功能 → 视觉效果 → 关闭「动画效果」，重启应用：面板与列表无动效但内容完整可见。恢复设置。
设置页主题下拉：顺序与原型一致（打字机第一，棕褐最后），逐套切换无控制台错误（`pnpm tauri:dev` 的 devtools）。

- [ ] **Step 5: 数据库迁移实机**

```bash
sqlite3 "%APPDATA%/com.inkling.app/inkling.db" "PRAGMA user_version; SELECT value FROM settings WHERE key='theme';"
```
（数据库路径以 `src-tauri/src/data/mod.rs` 中 `data_dir` 的实际取值为准；若本机无 sqlite3，改用设置页「打开数据目录」后以任意 SQLite 工具查看。）
Expected: `5` 与 `typewriter`（此前为 dark 的旧库）。

- [ ] **Step 6: 修复破损（如有）并回填结论**

发现破损时，只在 `extensions.css`（新开第 8 节「阶段一验收补丁」）或 `window-fit.css` 加最小覆盖，注明「阶段 N 迁移后删除」。把以下两项填入 spec §12：
- 面板入场 scale .97 在 WebView2 下是否保留（若毛玻璃错位则在 `playEnter` 去掉 `scale: 0.97` 并写明）；
- 破损补丁清单（文件 / 选择器 / 所属阶段），无则写「无」。

- [ ] **Step 7: 写验收记录并提交**

创建 `docs/superpowers/plans/2026-09-10-prototype-realign-phase1-acceptance.md`，内容为第 2–5 步的表格化结果（每项：检查点 / 打字机结果 / 深色结果 / 备注）。

```bash
pnpm sync:styles --check && pnpm typecheck && pnpm test:scripts
git add docs/superpowers/specs/2026-09-10-prototype-realign-phase1-foundation-design.md docs/superpowers/plans/2026-09-10-prototype-realign-phase1-acceptance.md src/styles
git commit -m "docs(design): 阶段一实机验收记录与实机结论回填"
```

---

## 自检记录

- **Spec 覆盖**：§4 分层 → Task 4 Step 6；§5 脚本 → Task 2/3；§6 extensions → Task 4；§7.1–7.4 → Task 5；§7.5 → Task 6；§8.1–8.3 → Task 7；§8.5 → Task 8；§8.4/8.6 → Task 9；§9 验证 → 各任务末步 + Task 10；§10 提交批次 → 每任务一次提交（批次 ① 拆为 Task 2/3/4 三次提交，批次 ③ 拆为 Task 7/8 两次）。
- **与 spec 的三处差异（已同步写回 spec）**：拒绝名单按「任一复合选择器命中」而非「起始复合选择器」；`v-stagger-list` 无参数、只对新出现的子元素入场，TodoTree 不需要新增 prop；extensions.css 增加第 1 节玻璃令牌 + `.glass` 覆盖、第 3 节同名补充声明、第 5 节过渡桥接。
- **类型一致性**：`enter/exit(el, { axis, distance, scale? }): Promise<boolean>`、`staggerIn(container, targets, { distance? })`、`vStaggerList` 在 Task 7/8/9 间一致；Rust `with_v5(&self) -> Result<(), String>` 与 `migrate()` 调用一致。
