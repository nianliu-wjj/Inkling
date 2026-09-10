import { computed, onBeforeUnmount, ref, watch, type ComputedRef, type Ref } from 'vue'
import { logger } from '@/service/logger'
import { api, type LauncherHit } from '@/service/tauri'

/** 命中类型 → 原型 .li-cat 分类文案。 */
export const LAUNCHER_KIND_LABEL: Record<LauncherHit['kind'], string> = {
  app: '应用',
  uwp: '应用',
  file: '文件',
  folder: '文件夹',
  command: '命令',
}

/** 命中类型 → 图标（与浮窗启动台一致）。 */
export const LAUNCHER_KIND_ICON: Record<LauncherHit['kind'], string> = {
  app: '🖥',
  uwp: '🧩',
  file: '📄',
  folder: '📁',
  command: '⚡',
}

/**
 * 启动台内嵌搜索（主窗口启动台页；与浮窗启动台共用同一份 Rust 索引）：
 * 输入去抖 30ms → api.launcher.search；↑↓ 循环选择；Enter / 点击执行「打开」。
 * 空查询时后端返回常用项，因此挂载即搜一次。
 */
export function useLauncherSearch(): {
  query: Ref<string>
  hits: Ref<LauncherHit[]>
  active: Ref<number>
  current: ComputedRef<LauncherHit | null>
  move: (delta: number) => void
  run: (index?: number) => Promise<void>
  onKeydown: (event: KeyboardEvent) => void
} {
  const query = ref('')
  const hits = ref<LauncherHit[]>([])
  const active = ref(0)
  let debounce: ReturnType<typeof setTimeout> | null = null

  async function search(): Promise<void> {
    try {
      hits.value = await api.launcher.search(query.value)
      active.value = 0
    } catch (error) {
      logger.error('launcher-page', '搜索失败', error)
      hits.value = []
    }
  }

  watch(
    query,
    () => {
      if (debounce) clearTimeout(debounce)
      debounce = setTimeout(() => void search(), 30)
    },
    { immediate: true },
  )

  const current = computed(() => hits.value[active.value] ?? null)

  function move(delta: number): void {
    if (!hits.value.length) return
    active.value = (active.value + delta + hits.value.length) % hits.value.length
  }

  async function run(index = active.value): Promise<void> {
    const hit = hits.value[index]
    if (!hit) return
    logger.info('launcher-page', `执行 ${hit.kind} ${hit.name}`)
    try {
      await api.launcher.launch(hit.path, hit.kind, 'open', query.value)
    } catch (error) {
      logger.error('launcher-page', '执行失败', error)
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      move(1)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      move(-1)
    } else if (event.key === 'Enter') {
      event.preventDefault()
      void run()
    }
  }

  onBeforeUnmount(() => {
    if (debounce) clearTimeout(debounce)
  })

  return { query, hits, active, current, move, run, onKeydown }
}
