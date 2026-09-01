import { computed, nextTick, onBeforeUnmount, ref, type ComputedRef, type CSSProperties, type Ref } from 'vue'
import { logger } from '@/service/logger'

/** 菜单相对锚点的展开方向。 */
export type MenuPlacement = 'bottom' | 'top'

/** 视口边缘留白，避免菜单贴边。 */
const VIEWPORT_MARGIN = 8
/** 锚点与菜单之间的间距。 */
const ANCHOR_GAP = 6

/**
 * 锚定弹出菜单。
 *
 * 需求 2.2「优先级选择器」明确的行为：
 * - 默认位于锚点下方并与锚点**左边缘对齐**；
 * - 下方空间不足时整体翻转到上方（不遮挡当前卡片内容）；
 * - 超出窗口边界时向内收缩；
 * - 支持键盘：↑/↓、Home/End 移动，Enter 提交，Esc 或点击外部取消；
 * - 关闭后焦点返回锚点元素。
 *
 * 菜单使用 position: fixed 定位（配合 .prio-group / .repeat-menu 样式），
 * 因此直接使用 getBoundingClientRect 的视口坐标，无需换算滚动偏移。
 */
export function useAnchoredMenu(optionCount: () => number): {
  visible: Ref<boolean>
  placement: Ref<MenuPlacement>
  activeIndex: Ref<number>
  menuRef: Ref<HTMLElement | null>
  style: ComputedRef<CSSProperties>
  open: (anchor: HTMLElement, initialIndex?: number) => Promise<void>
  close: () => void
  onKeydown: (event: KeyboardEvent, onSubmit: (index: number) => void) => void
} {
  const visible = ref(false)
  const placement = ref<MenuPlacement>('bottom')
  const activeIndex = ref(0)
  const menuRef = ref<HTMLElement | null>(null)

  const left = ref(0)
  const top = ref(0)
  /** 记录触发菜单的锚点，关闭时把焦点还回去。 */
  let anchorEl: HTMLElement | null = null

  const style = computed<CSSProperties>(() => ({
    left: `${left.value}px`,
    top: `${top.value}px`,
  }))

  /**
   * 计算菜单位置：先按下方左对齐摆放，再按需翻转与收缩。
   * 必须在菜单已渲染（拿得到真实尺寸）之后调用。
   */
  function reposition(): void {
    const menu = menuRef.value
    if (!menu || !anchorEl) return

    const anchor = anchorEl.getBoundingClientRect()
    const { offsetWidth: menuW, offsetHeight: menuH } = menu
    const { innerWidth: vw, innerHeight: vh } = window

    // 垂直：默认锚点下方；下方放不下且上方放得下时翻转。
    const spaceBelow = vh - anchor.bottom - ANCHOR_GAP
    const spaceAbove = anchor.top - ANCHOR_GAP
    const flip = spaceBelow < menuH && spaceAbove > spaceBelow
    placement.value = flip ? 'top' : 'bottom'
    top.value = flip ? anchor.top - ANCHOR_GAP - menuH : anchor.bottom + ANCHOR_GAP

    // 垂直兜底：极端情况下两侧都放不下，钳制在视口内。
    top.value = Math.max(VIEWPORT_MARGIN, Math.min(top.value, vh - menuH - VIEWPORT_MARGIN))

    // 水平：与锚点左边缘对齐，右侧溢出时向内收缩。
    left.value = Math.max(VIEWPORT_MARGIN, Math.min(anchor.left, vw - menuW - VIEWPORT_MARGIN))

    logger.debug('anchored-menu', `定位完成 placement=${placement.value} left=${left.value} top=${top.value}`)
  }

  /** 点击菜单与锚点之外的区域即关闭。 */
  function onDocumentPointerDown(event: MouseEvent): void {
    const target = event.target as Node | null
    if (!target) return
    if (menuRef.value?.contains(target)) return
    if (anchorEl?.contains(target)) return
    close()
  }

  /** 全局 Esc 兜底：焦点不在菜单内时也能关闭。 */
  function onDocumentKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.stopPropagation()
      close()
    }
  }

  function bindGlobalListeners(): void {
    // 用 capture 阶段，确保先于卡片自身的点击处理执行。
    document.addEventListener('pointerdown', onDocumentPointerDown, true)
    document.addEventListener('keydown', onDocumentKeydown, true)
  }

  function unbindGlobalListeners(): void {
    document.removeEventListener('pointerdown', onDocumentPointerDown, true)
    document.removeEventListener('keydown', onDocumentKeydown, true)
  }

  async function open(anchor: HTMLElement, initialIndex = 0): Promise<void> {
    logger.debug('anchored-menu', `打开菜单 initialIndex=${initialIndex}`)
    anchorEl = anchor
    activeIndex.value = initialIndex
    visible.value = true
    bindGlobalListeners()
    // 等菜单挂载出真实尺寸后再定位，否则 offsetHeight 为 0 会误判翻转。
    await nextTick()
    reposition()
    menuRef.value?.focus()
  }

  function close(): void {
    if (!visible.value) return
    logger.debug('anchored-menu', '关闭菜单')
    visible.value = false
    unbindGlobalListeners()
    // 焦点归还锚点，保证键盘用户不会丢失位置。
    anchorEl?.focus()
    anchorEl = null
  }

  /** 菜单内键盘导航；提交交由调用方处理具体语义。 */
  function onKeydown(event: KeyboardEvent, onSubmit: (index: number) => void): void {
    const count = optionCount()
    if (count <= 0) return

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        activeIndex.value = (activeIndex.value + 1) % count
        break
      case 'ArrowUp':
        event.preventDefault()
        activeIndex.value = (activeIndex.value - 1 + count) % count
        break
      case 'Home':
        event.preventDefault()
        activeIndex.value = 0
        break
      case 'End':
        event.preventDefault()
        activeIndex.value = count - 1
        break
      case 'Enter':
      case ' ':
        event.preventDefault()
        onSubmit(activeIndex.value)
        break
      case 'Escape':
        event.preventDefault()
        close()
        break
      default:
        break
    }
  }

  // 组件销毁时务必摘掉全局监听，否则会随窗口视图切换不断累积。
  onBeforeUnmount(unbindGlobalListeners)

  return { visible, placement, activeIndex, menuRef, style, open, close, onKeydown }
}
