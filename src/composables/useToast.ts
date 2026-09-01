import { ref, type Ref } from 'vue'
import { logger } from '@/service/logger'

/** 单条 toast 的默认展示时长。 */
const DEFAULT_DURATION_MS = 2000

/**
 * 轻提示。
 *
 * 每个窗口共用一份模块级状态，因此在任意组件中调用 useToast() 拿到的
 * 都是同一个 message，配合窗口根部的单个 <ToastHost /> 渲染。
 */
const message = ref<string>('')
const visible = ref(false)
let timer: ReturnType<typeof setTimeout> | null = null

export function useToast(): {
  message: Ref<string>
  visible: Ref<boolean>
  toast: (text: string, duration?: number) => void
} {
  function toast(text: string, duration = DEFAULT_DURATION_MS): void {
    logger.debug('toast', text)
    if (timer !== null) clearTimeout(timer)
    message.value = text
    visible.value = true
    timer = setTimeout(() => {
      visible.value = false
      timer = null
    }, duration)
  }

  return { message, visible, toast }
}
