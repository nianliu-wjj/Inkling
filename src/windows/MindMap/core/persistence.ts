/**
 * 思维导图文档数据的持久化格式。
 *
 * `note.mindmap_data` 存 `mindMap.getData(true)` 的 JSON：`{ root, layout, theme, view }`。
 * 历史数据（2026-09-06 之前）只存了 root（`{ data, children }`），读取时按 root 兼容；
 * 一旦保存就升级为全量格式。纯函数，不依赖 Vue 与库。
 */
export interface MindMapRoot {
  data: Record<string, unknown>
  children: MindMapRoot[]
}

export interface MindMapFullData {
  root: MindMapRoot
  layout?: string
  theme?: { template?: string; config?: Record<string, unknown> }
  view?: unknown
}

export function createEmptyRoot(text: string): MindMapRoot {
  return { data: { text }, children: [] }
}

function isRoot(value: unknown): value is MindMapRoot {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as MindMapRoot).data === 'object' &&
    (value as MindMapRoot).data !== null
  )
}

/** 解析持久化字符串；空串 / 非法 JSON / 结构不对时回退为一个可编辑的空根节点。 */
export function parseMindMapData(source: string | null | undefined, rootText = '中心主题'): MindMapFullData {
  if (!source?.trim()) return { root: createEmptyRoot(rootText) }
  let parsed: unknown
  try {
    parsed = JSON.parse(source)
  } catch {
    // 手工损坏或旧版本写坏的数据不阻断编辑器，回退为空根节点。
    return { root: createEmptyRoot(rootText) }
  }
  if (typeof parsed !== 'object' || parsed === null) return { root: createEmptyRoot(rootText) }
  const candidate = parsed as Partial<MindMapFullData>
  // 全量格式：含合法 root。
  if (isRoot(candidate.root)) {
    return {
      root: candidate.root,
      layout: typeof candidate.layout === 'string' ? candidate.layout : undefined,
      theme: typeof candidate.theme === 'object' && candidate.theme !== null ? candidate.theme : undefined,
      view: candidate.view,
    }
  }
  // 旧格式：整个对象就是 root。
  if (isRoot(parsed)) return { root: parsed }
  return { root: createEmptyRoot(rootText) }
}

export function serializeMindMapData(data: MindMapFullData): string {
  return JSON.stringify(data)
}
