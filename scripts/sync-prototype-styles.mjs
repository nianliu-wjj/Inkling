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

/** 递归统计规则条数（含 @media 内部）。 */
function countRules(nodes) {
  return nodes.reduce((sum, n) => sum + (n.type === 'rule' ? 1 : n.type === 'at' ? countRules(n.children) : 0), 0)
}

function parseArgs(argv) {
  const check = argv.includes('--check')
  const outIdx = argv.indexOf('--out')
  if (outIdx >= 0 && !argv[outIdx + 1]) throw new Error('--out 需要目录参数')
  const out = outIdx >= 0 ? path.resolve(argv[outIdx + 1]) : DEFAULT_OUT
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

  // 比对时统一换行符：Windows 上 git autocrlf 可能把工作树文件写成 CRLF。
  const normalize = (s) => s.replace(/\r\n/g, '\n')
  const dirty = outputs.filter((o) => {
    const existing = fs.existsSync(o.file) ? normalize(fs.readFileSync(o.file, 'utf8')) : null
    return existing !== normalize(o.text)
  })

  if (check) {
    if (dirty.length) {
      console.error(
        `[sync:styles] 以下生成文件与原型不一致，请运行 pnpm sync:styles：\n  ${dirty.map((o) => o.file).join('\n  ')}`,
      )
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
