import { onBeforeUnmount, ref, type Ref } from 'vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'

/**
 * 全局快捷键录制（偏好设置页的面板快捷键、启动台页的启动器快捷键共用）。
 *
 * 进入录制态后捕获下一次「修饰键 + 主键」组合，规范化为 Tauri 接受的 `Ctrl+Shift+Space` 形式，
 * 先交给后端改绑（后端返回实际生效的组合），成功后再持久化；录制结束即移除键盘监听。
 *
 * `persist` 返回**是否已落库**：改绑成功但设置没保存时，后端已经生效、设置里却还是旧值，
 * 这时不能再报「已设为 X」（4C 验收记录 §5.1）。保存失败的具体提示由 `persist` 自己给
 * （两处调用方的 `patch` 都会弹「保存设置失败」），本组合式负责**不报假成功**，并补一条
 * 「已生效但未能保存」把「改绑成功、只是没存下」这层意思说清楚。
 */
export function useShortcutRecorder(options: {
  /** 用于日志与提示的名称，如「面板」「启动器」。 */
  label: string
  /** 把组合交给后端改绑，返回实际生效的组合。 */
  apply: (combo: string) => Promise<string>
  /** 改绑成功后持久化到设置，返回是否保存成功。 */
  persist: (applied: string) => Promise<boolean>
}): { recording: Ref<boolean>; start: () => void } {
  const { toast } = useToast()
  const recording = ref(false)

  function stop(): void {
    recording.value = false
    window.removeEventListener('keydown', onKeydown)
  }

  async function rebind(combo: string): Promise<void> {
    logger.info('shortcut-recorder', `重新绑定${options.label}快捷键 ${combo}`)
    try {
      const applied = await options.apply(combo)
      // 保存失败时不报「已设为 X」：`persist` 内部已弹过「保存设置失败」，这里再补一条说明
      // 「后端已改绑、只是没存进设置」——否则用户只看到保存失败，会以为改绑也没生效。
      if (!(await options.persist(applied))) {
        toast(`${options.label}快捷键已生效，但未能保存到设置`)
        return
      }
      toast(`${options.label}快捷键已设为 ${applied}`)
    } catch (error) {
      logger.error('shortcut-recorder', `${options.label}快捷键绑定失败`, error)
      toast(`快捷键绑定失败：${String(error)}`)
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (!recording.value) return
    event.preventDefault()

    // 只按下修饰键时继续等待主键。
    const key = event.key
    if (['Control', 'Shift', 'Alt', 'Meta'].includes(key)) return

    const parts: string[] = []
    if (event.ctrlKey) parts.push('Ctrl')
    if (event.shiftKey) parts.push('Shift')
    if (event.altKey) parts.push('Alt')
    if (event.metaKey) parts.push('Super')
    parts.push(key === ' ' ? 'Space' : key.length === 1 ? key.toUpperCase() : key)

    stop()
    void rebind(parts.join('+'))
  }

  function start(): void {
    if (recording.value) return
    recording.value = true
    toast('请按下新的快捷键组合')
    window.addEventListener('keydown', onKeydown)
  }

  onBeforeUnmount(stop)

  return { recording, start }
}
