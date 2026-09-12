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
