import { onBeforeUnmount, ref, type Ref } from 'vue'
import { logger } from '@/service/logger'

/** 无操作自动退出抖动态的超时（需求 2.2：3 秒）。 */
const AUTO_DISARM_MS = 3000

/**
 * 标签删除的「抖动二次确认」。
 *
 * 需求 2.2：首次点击标签的 ✕ 进入抖动确认态——目标标签以约 0.7s/次
 * 的频率左右抖动并变红（样式 .tag-chip.shaking，动画 tagShake）；
 * 再次点击才真正删除；3 秒无操作自动退出。
 * 「悬浮 ✕ 时暂停抖动」由 CSS 的 :has(.tag-del:hover) 规则负责，此处不涉及。
 */
export function useShakeConfirm(): {
  shakingId: Ref<string | null>
  isArmed: (id: string) => boolean
  /** 点击 ✕ 的统一入口：未进入确认态则进入并返回 false；已在确认态则返回 true 表示应执行删除。 */
  press: (id: string) => boolean
  disarm: () => void
} {
  const shakingId = ref<string | null>(null)
  let timer: ReturnType<typeof setTimeout> | null = null

  function clearTimer(): void {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }

  function disarm(): void {
    clearTimer()
    shakingId.value = null
  }

  function press(id: string): boolean {
    if (shakingId.value === id) {
      logger.info('shake-confirm', `二次确认通过，执行删除 id=${id}`)
      disarm()
      return true
    }
    logger.debug('shake-confirm', `进入抖动确认态 id=${id}`)
    clearTimer()
    shakingId.value = id
    timer = setTimeout(() => {
      logger.debug('shake-confirm', `确认超时自动退出 id=${id}`)
      shakingId.value = null
      timer = null
    }, AUTO_DISARM_MS)
    return false
  }

  onBeforeUnmount(clearTimer)

  return {
    shakingId,
    isArmed: (id: string) => shakingId.value === id,
    press,
    disarm,
  }
}
