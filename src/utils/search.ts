import { parseMindMapData, type MindMapRoot } from '@/windows/MindMap/core/persistence'

/**
 * 搜索相关纯函数（原型 hiText / mindmapAllText / saveMindmap.rootText 的口径）。
 * 不依赖 Vue 与 DOM，可直接用 node:test 覆盖。
 */

/**
 * 命中高亮：忽略大小写只标首个命中片段，返回分段供模板渲染 `<mark>`。
 * 刻意不返回 HTML 字符串——模板用 v-for 渲染分段即可，避免 v-html 与转义问题。
 */
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

/** 剥离 simple-mind-map 富文本节点里的 HTML 标签与实体空格。 */
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

/** 导图全部节点文本（空格拼接），供笔记搜索；空串 / 非法数据返回空串。 */
export function mindmapAllText(json: string | null | undefined): string {
  if (!json?.trim()) return ''
  const out: string[] = []
  walk(parseMindMapData(json, '').root, out)
  return out.join(' ')
}

/** 导图根节点文字，作为导图笔记卡片正文（原型 saveMindmap 的 rootText）；缺失回退为 fallback。 */
export function mindmapRootText(json: string | null | undefined, fallback = '未命名导图'): string {
  if (!json?.trim()) return fallback
  return stripRichText(parseMindMapData(json, '').root.data?.text) || fallback
}
