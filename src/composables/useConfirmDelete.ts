import { ref, type Ref } from 'vue'
import { logger } from '@/service/logger'

/**
 * 删除二次确认态。
 *
 * 点击卡片右上角 ✕ 不直接删除，而是进入确认态：pendingId 记录待删卡片，
 * 浮层本体是 body 级锚定的 CardConfirm（原型 #cardConfirm，锚到卡片右侧带箭头）。
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
