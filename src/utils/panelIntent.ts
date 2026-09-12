/**
 * 面板呼出意图解析（纯函数，无 Tauri 依赖，便于单测）。
 *
 * 后端 `pending_panel_intent` 以 JSON 字符串交给前端：`{ page, noteId? }`；
 * 这里负责把它解析成结构化对象，任何格式不合法的输入一律视为「无意图」。
 */

/** 面板呼出意图（对应 Rust `pending_panel_intent` 的 JSON）：切到哪一页，可选要回显的笔记 id。 */
export interface PanelIntent {
  page: string
  noteId?: string
}

/** 解析后端返回的意图 JSON；格式不合法（空值 / 非 JSON / 非对象 / 缺 page / page 非字符串）时视为无意图。 */
export function parsePanelIntent(raw: string | null | undefined): PanelIntent | null {
  if (!raw) return null
  try {
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null) return null
    const record = parsed as Record<string, unknown>
    if (typeof record.page !== 'string') return null
    return { page: record.page, noteId: typeof record.noteId === 'string' ? record.noteId : undefined }
  } catch {
    return null
  }
}
