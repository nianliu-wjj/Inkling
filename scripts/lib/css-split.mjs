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
