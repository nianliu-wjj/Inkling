/**
 * 思维导图窗口令牌的静态断言（阶段五 Task 6）。
 *
 * 背景：导图窗口的 chrome（顶栏三岛 / 抽屉 / 模态 / 右键菜单 / 底栏 / 小地图）全部走 `--mm-*`，
 * 而 `--mm-*` 原先只在 tokens.css 的基础 `:root`（从原型逐字抄来的**浅色**值）与 typewriter
 * 主题块里定义过。其余 30 套主题（含 13 套深色）因此是全站唯一不跟随换肤的窗口——
 * 深色主题下白底浅字、浅色主题下反倒是深底。原型源码自己也把这条留成了待办
 * （docs/styles.css 的「⚠️ 下一步：在各 [data-theme] 主题块中覆盖以下 13 个令牌」）。
 *
 * 本测试把三件事固化成断言：
 *   1. 32 套主题**各自**都有全部 13 个 `--mm-*`（不漏套、不漏项）；
 *   2. `--mm-*` 取值引用的 10 个来源令牌在每套主题里都存在（缺一个会让规则静默失效）；
 *   3. 扩展层里新增的 `--mm-*` 一律由主题既有令牌推导，不许手挑颜色。
 *
 * 读文本而非跑浏览器：令牌是纯文本事实，静态断言足够，且无需 DOM 环境。
 */
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

/** 导图窗口 chrome 依赖的 13 个令牌（定义见 tokens.css 的「思维导图窗口语义色」一节）。 */
const MM_TOKENS = [
  '--mm-surface',
  '--mm-surface-2',
  '--mm-surface-3',
  '--mm-border',
  '--mm-text',
  '--mm-text-2',
  '--mm-text-dim',
  '--mm-primary',
  '--mm-primary-soft',
  '--mm-danger',
  '--mm-danger-soft',
  '--mm-accent-2',
  '--mm-accent-2-rgb',
] as const

/**
 * 13 个 `--mm-*` 的取值所引用的来源令牌。
 * 只要某套主题少定义了其中一个，对应的一条 `--mm-*` 就会静默失效（退化成继承值或无效值），
 * 所以逐套核对是必要的，不能只看「有没有写这 13 行」。
 */
const SOURCE_TOKENS = [
  '--scheme',
  '--wsa',
  '--menu-bg',
  '--glass-bg',
  '--accent',
  '--accent-rgb',
  '--text',
  '--text-strong',
  '--text-dim',
  '--red',
] as const

/** 项目根目录（测试由 `pnpm test:unit` 在根目录执行；用 import.meta.url 定位更稳）。 */
const ROOT = new URL('../../', import.meta.url)

/** 主题令牌块的来源：生成层 + 项目扩展层 + 原型参照（typewriter 的块只在后两者里）。 */
const THEME_BLOCK_FILES = ['src/styles/themes.css', 'src/styles/extensions.css', 'docs/styles.css']

/** 读项目根下的文件文本。 */
function read(relativePath: string): string {
  return readFileSync(new URL(relativePath, ROOT), 'utf8')
}

/** 主题清单以 constants/themes.ts 为唯一真源，避免测试里再维护一份 32 套的名单。 */
function themeKeys(): string[] {
  return [...read('src/constants/themes.ts').matchAll(/key: '([^']+)'/g)].map((m) => m[1])
}

/**
 * 收集每套主题的令牌定义块。
 *
 * 同一主题可能分散在多处：typewriter 的块在 themes.css / docs/styles.css，sepia 的自有令牌在
 * extensions.css §7、`--mm-*` 在同一文件的 §12。因此按 key 合并全部命中块，而不是只取第一个。
 *
 * **刻意不收集无 `data-theme` 的基础 `:root`**：它是浅色值（原型默认），若拿它给 `dark` 兜底，
 * 深色主题下导图窗口会变成全站唯一的浅色窗口——正是本任务要修的问题，必须有显式覆盖。
 */
function themeBlocks(): Map<string, string> {
  const blocks = new Map<string, string>()
  for (const file of THEME_BLOCK_FILES) {
    const pattern = /:root\[data-theme=['"]([^'"]+)['"]\]\s*\{([\s\S]*?)\n\}/g
    for (const [, key, body] of read(file).matchAll(pattern)) {
      blocks.set(key, (blocks.get(key) ?? '') + body)
    }
  }
  return blocks
}

/**
 * tokens.css 里无 `data-theme` 的基础 `:root` 块。
 * 它**不是**任何主题的兜底（见 themeBlocks 的说明），但确实是 `dark` 这套主题的令牌定义处：
 * `dark` 主题没有自己的 `[data-theme]` 块，落到基础 `:root` 就是它的语义。
 */
function baseRootBlock(): string {
  const matched = /(?:^|\n):root\s*\{([\s\S]*?)\n\}/.exec(read('src/styles/tokens.css'))
  assert.ok(matched, '未在 tokens.css 找到基础 :root 令牌块')
  return matched[1]
}

/** 令牌块里是否声明了某个自定义属性。`\s*:` 保证 `--mm-text` 不会命中 `--mm-text-2`。 */
function declares(block: string, token: string): boolean {
  return new RegExp(`${token}\\s*:`).test(block)
}

test('32 套主题各自都定义了全部 13 个 --mm-* 令牌', () => {
  const blocks = themeBlocks()
  const problems: string[] = []
  for (const key of themeKeys()) {
    const block = blocks.get(key)
    if (block === undefined) {
      // 显式块缺失（`dark` 最容易踩：它落到基础 `:root`，必须自己去覆盖）
      problems.push(`${key}：没有 :root[data-theme='${key}'] 块`)
      continue
    }
    const missing = MM_TOKENS.filter((token) => !declares(block, token))
    if (missing.length) problems.push(`${key}：缺 ${missing.join(' ')}`)
  }
  assert.deepEqual(problems, [], `以下主题未覆盖 --mm-*：\n${problems.join('\n')}`)
})

test('每套主题都定义了 --mm-* 取值引用的 10 个来源令牌', () => {
  const blocks = themeBlocks()
  const base = baseRootBlock()
  const problems: string[] = []
  for (const key of themeKeys()) {
    // `dark` 的主题令牌就是基础 `:root`（它没有自己的块）；其余主题必须自己声明——
    // 靠继承基础 `:root` 会拿到**深色**的值，浅色主题的 --mm-surface 就会是暗底。
    const block = key === 'dark' ? base : blocks.get(key)
    if (block === undefined) {
      problems.push(`${key}：没有令牌定义块`)
      continue
    }
    const missing = SOURCE_TOKENS.filter((token) => !declares(block, token))
    if (missing.length) problems.push(`${key}：缺 ${missing.join(' ')}`)
  }
  assert.deepEqual(problems, [], `以下主题缺 --mm-* 所依赖的来源令牌：\n${problems.join('\n')}`)
})

test('扩展层新增的 --mm-* 取值全部由主题既有令牌推导，不手挑颜色', () => {
  const literals: string[] = []
  // 只看主题块里的声明：`rgba(var(--wsa), .08)` 与 `color-mix(in srgb, var(--red) …)` 合格，
  // 裸的 `#ffffff` / `rgba(255, 255, 255, .9)` 说明是手挑的、不会随主题变。
  const pattern = /:root\[data-theme=['"]([^'"]+)['"]\]\s*\{([\s\S]*?)\n\}/g
  for (const [, key, body] of read('src/styles/extensions.css').matchAll(pattern)) {
    for (const line of body.split('\n')) {
      const declared = /^\s*(--mm-[\w-]+)\s*:\s*(.+?);\s*$/.exec(line)
      if (!declared) continue
      if (!/var\(|color-mix\(/.test(declared[2])) {
        literals.push(`${key} 的 ${declared[1]}: ${declared[2]}`)
      }
    }
  }
  assert.deepEqual(literals, [], `以下 --mm-* 未引用主题令牌：\n${literals.join('\n')}`)
})
