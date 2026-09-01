import { ref, type Ref } from 'vue'
import { logger } from '@/service/logger'

/**
 * 删除二次确认。
 *
 * 需求 2.2：点击卡片右上角 ✕ 不直接删除，而是在**卡片上方**浮出确认框
 * （绝对定位，不占满卡片、不推挤布局，样式见 .card-confirm）。
 * 同一时刻只允许一个待确认项，避免出现多个确认框。
 */
export function useConfirmDelete(scope = 'confirm-delete'): {
  pendingId: Ref<string | null>
  isPending: (id: string) => boolean
  ask: (id: string) => void
  cancel: () => void
  confirm: () => string | null
} {
  const pendingId = ref<string | null>(null)

  return {
    pendingId,
    isPending: (id: string) => pendingId.value === id,

    /** 进入确认态；若已有其他待确认项则直接替换。 */
    ask: (id: string): void => {
      logger.debug(scope, `进入删除确认态 id=${id}`)
      pendingId.value = id
    },

    cancel: (): void => {
      if (pendingId.value) logger.debug(scope, `取消删除 id=${pendingId.value}`)
      pendingId.value = null
    },

    /** 确认删除，返回待删除 id 并复位；无待确认项时返回 null。 */
    confirm: (): string | null => {
      const id = pendingId.value
      if (id) logger.info(scope, `确认删除 id=${id}`)
      pendingId.value = null
      return id
    },
  }
}
