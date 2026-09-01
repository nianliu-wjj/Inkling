import { ref, type Ref } from 'vue'

/**
 * 卡片操作按钮的悬浮显隐。
 *
 * 需求 2.2「悬浮层级隔离」：同一时刻只能有一个卡片处于 hover 态。
 * 待办树中父子卡片在 DOM 上嵌套，若不阻止冒泡，鼠标移入子任务时
 * 父级也会收到 mouseenter，导致两层按钮同时显示、并在移动时抖动。
 * 因此 bind() 返回的处理器一律 stopPropagation。
 */
export function useHoverActions(): {
  hoveredId: Ref<string | null>
  isHovered: (id: string) => boolean
  bind: (id: string) => {
    onMouseenter: (event: MouseEvent) => void
    onMouseleave: (event: MouseEvent) => void
  }
} {
  const hoveredId = ref<string | null>(null)

  const bind = (id: string) => ({
    onMouseenter: (event: MouseEvent): void => {
      // 阻止冒泡到父级卡片，实现父子层级隔离。
      event.stopPropagation()
      hoveredId.value = id
    },
    onMouseleave: (event: MouseEvent): void => {
      event.stopPropagation()
      // 仅当离开的确实是当前高亮项时才清空，避免子级 leave 误清父级状态。
      if (hoveredId.value === id) hoveredId.value = null
    },
  })

  return {
    hoveredId,
    isHovered: (id: string) => hoveredId.value === id,
    bind,
  }
}
