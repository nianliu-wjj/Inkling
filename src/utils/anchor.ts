/**
 * 浮层锚定纯函数：把浮层摆到锚点卡片旁边（原型 app.js `positionTodoEditor` / `refreshCardConfirm` 的定位段）。
 *
 * 优先卡片右侧（+14）→ 右侧不够翻到左侧（−14 − W，placement = 'left'，箭头转到右缘）→ 两侧都不够时按
 * `fallback` 兜底：'center' 是原型的水平居中盖住卡片；'below' 是 spec D27 的卡片下方（箭头朝上、左缘对齐），
 * 下方也放不下时改到卡片上方并返回 'above'（4B D29：箭头朝下）。纵向 `align` 'center' 对齐卡片中心（删除确认）、'top' 对齐卡片顶部（待办编辑），
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
  placement: 'right' | 'left' | 'center' | 'below' | 'above'
  /** 箭头相对浮层顶部的纵向位置（px）；placement = 'below' / 'above' 时箭头改为朝上 / 朝下贴边，此值不再使用。 */
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
    // D27：卡片下方、左缘对齐；下方放不下则改到卡片上方（D29：placement = 'above'，箭头朝下）。
    left = clamp(anchor.left, VIEWPORT_MARGIN, maxLeft)
    let top = bottom + BELOW_GAP
    let vertical: AnchorResult['placement'] = 'below'
    if (top > maxTop) {
      top = anchor.top - H - BELOW_GAP
      vertical = 'above'
    }
    top = clamp(top, VIEWPORT_MARGIN, maxTop)
    return { left, top, placement: vertical, caretY: clamp(centerY - top, CARET_MIN, H - CARET_MIN) }
  }

  const rawTop = opts.align === 'top' ? anchor.top : centerY - H / 2
  const top = clamp(rawTop, VIEWPORT_MARGIN, maxTop)
  const caretY = clamp(centerY - top, CARET_MIN, H - CARET_MIN)
  return { left, top, placement, caretY }
}
