<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { logger } from '@/service/logger'
import { api, type LauncherHit } from '@/service/tauri'

/**
 * 启动器搜索窗口。
 *
 * 输入即搜（去抖 30ms → Rust 打分）；结果键盘导航。窗口失焦或 Esc 隐藏——
 * 由全局快捷键再次呼出。每次显示（visibilitychange / focus）清空输入并聚焦输入框。
 */
applyCachedTheme()
applyCachedGlass()
document.documentElement.dataset.window = 'launcher'

const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
watch(
  () => settings.value.theme,
  (t) => applyTheme(t),
  { immediate: true },
)
watch(
  () => settings.value.glass_level,
  (l) => applyGlass(l),
  { immediate: true },
)

const KIND_ICON: Record<LauncherHit['kind'], string> = {
  app: '🖥',
  uwp: '🧩',
  file: '📄',
  folder: '📁',
  command: '⚡',
}
/** 每条结果可执行的动作（Tab 循环）。 */
const ACTIONS = [
  { key: 'open', label: '打开', mode: 'open' as const },
  { key: 'admin', label: '管理员', mode: 'admin' as const },
  { key: 'reveal', label: '打开所在文件夹', mode: 'reveal' as const },
]

const input = ref<HTMLInputElement | null>(null)
const query = ref('')
const hits = ref<LauncherHit[]>([])
const active = ref(0)
const actionIndex = ref(0)

let debounce: ReturnType<typeof setTimeout> | null = null

watch(query, () => {
  if (debounce) clearTimeout(debounce)
  debounce = setTimeout(() => void runSearch(), 30)
})

async function runSearch(): Promise<void> {
  try {
    hits.value = await api.launcher.search(query.value)
    active.value = 0
    actionIndex.value = 0
  } catch (error) {
    logger.error('launcher', '搜索失败', error)
    hits.value = []
  }
}

const current = computed(() => hits.value[active.value] ?? null)
/** 命中项可用的动作：UWP / 命令没有「管理员」「打开所在文件夹」。 */
const actions = computed(() => {
  const hit = current.value
  if (!hit) return []
  if (hit.kind === 'command' || hit.kind === 'uwp') return ACTIONS.filter((a) => a.key === 'open')
  return ACTIONS
})

async function launch(mode: 'open' | 'admin' | 'reveal'): Promise<void> {
  const hit = current.value
  if (!hit) return
  try {
    await api.launcher.launch(hit.id, mode, query.value)
  } catch (error) {
    logger.error('launcher', '启动失败', error)
  }
}

function move(delta: number): void {
  if (!hits.value.length) return
  active.value = (active.value + delta + hits.value.length) % hits.value.length
  actionIndex.value = 0
}

function onKeydown(event: KeyboardEvent): void {
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      move(1)
      break
    case 'ArrowUp':
      event.preventDefault()
      move(-1)
      break
    case 'Tab':
      event.preventDefault()
      if (actions.value.length) actionIndex.value = (actionIndex.value + 1) % actions.value.length
      break
    case 'Enter': {
      event.preventDefault()
      // Ctrl+Enter 强制管理员；否则执行当前选中的动作
      const mode = event.ctrlKey ? 'admin' : (actions.value[actionIndex.value]?.mode ?? 'open')
      void launch(mode)
      break
    }
    case 'Escape':
      event.preventDefault()
      void api.launcher.hide()
      break
    default:
      // Alt+数字 直达
      if (event.altKey && /^[1-9]$/.test(event.key)) {
        const index = Number(event.key) - 1
        if (index < hits.value.length) {
          active.value = index
          void launch('open')
        }
        event.preventDefault()
      }
  }
}

/** 窗口每次显示：清空并聚焦。隐藏时也清空，避免下次残留上次结果。 */
function onFocus(): void {
  query.value = ''
  hits.value = []
  active.value = 0
  void nextTick(() => input.value?.focus())
  void runSearch() // 空查询 → 常用项
}

function onBlur(): void {
  // 失焦即隐藏（点了别处）。
  void api.launcher.hide()
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('focus', onFocus)
  window.addEventListener('blur', onBlur)
  onFocus()
  logger.info('launcher', '搜索窗口已挂载')
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('focus', onFocus)
  window.removeEventListener('blur', onBlur)
})

void getCurrentWindow()
</script>

<template>
  <div class="launcher">
    <div class="launcher-input-row">
      <span class="launcher-search-icon">🔍</span>
      <input
        ref="input"
        v-model="query"
        class="launcher-input"
        type="text"
        placeholder="搜索程序、文件、文件夹…"
        spellcheck="false"
      />
    </div>

    <ul v-if="hits.length" class="launcher-list">
      <li
        v-for="(hit, index) in hits"
        :key="hit.id"
        class="launcher-item"
        :class="{ active: index === active }"
        @mouseenter="active = index"
        @click="launch('open')"
      >
        <span class="launcher-kind">{{ KIND_ICON[hit.kind] }}</span>
        <div class="launcher-texts">
          <span class="launcher-name">{{ hit.name }}</span>
          <span v-if="hit.kind !== 'command'" class="launcher-path">{{ hit.path }}</span>
        </div>
        <kbd v-if="index < 9" class="launcher-hint">Alt+{{ index + 1 }}</kbd>
      </li>
    </ul>
    <div v-else-if="query" class="launcher-empty">没有匹配结果</div>

    <!-- 动作栏：显示当前选中项可执行的动作，Tab 循环 -->
    <div v-if="actions.length > 1" class="launcher-actions">
      <span
        v-for="(action, index) in actions"
        :key="action.key"
        class="launcher-action"
        :class="{ active: index === actionIndex }"
      >
        {{ action.label }}
      </span>
      <span class="launcher-actions-hint">Tab 切换 · Enter 执行 · Ctrl+Enter 管理员</span>
    </div>
  </div>
</template>
