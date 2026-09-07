<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { resolveIslandPlugins, type IslandItem } from '@/island-plugins'
import { AppEvents, onAppEvent } from '@/service/events'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 灵动岛根组件（spec D6/D7/D8）。
 *
 * - 条目来自启用插件的 `useItems()` 拼接，按 `island_cycle_seconds` 轮播；悬停时暂停；
 * - 悬停 / 点击不用 DOM 鼠标事件，统一订阅后端推送的 island-hover / island-click：
 *   穿透模式下窗口收不到鼠标事件，只有这一条来源；非穿透模式沿用同一来源保证行为一致；
 * - 悬停展开：请求 Rust 拉高窗口，渲染当前条目的详情组件；
 * - 点击：由 Rust 直接呼出面板并切到待办页，这里只做一次视觉反馈。
 */
applyCachedTheme()
applyCachedGlass()
document.documentElement.dataset.window = 'island'

const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()

watch(
  () => settings.value.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
)
watch(
  () => settings.value.glass_level,
  (level) => applyGlass(level),
  { immediate: true },
)

// 插件在 setup 内各调用一次 useItems（组合式函数，不能放进 computed 里反复调用）。
const pluginItems = resolveIslandPlugins(settings.value.island_plugins).map((plugin) => ({
  plugin,
  items: plugin.useItems(),
}))
const enabledIds = computed(() => resolveIslandPlugins(settings.value.island_plugins).map((p) => p.id))
/** 全部条目：按设置里的插件顺序拼接。 */
const items = computed<IslandItem[]>(() =>
  enabledIds.value.flatMap((id) => pluginItems.find((entry) => entry.plugin.id === id)?.items.value ?? []),
)
const emptyText = computed(
  () => pluginItems.find((entry) => enabledIds.value.includes(entry.plugin.id))?.plugin.emptyText ?? '暂无内容',
)

const index = ref(0)
const expanded = ref(false)
const clicked = ref(false)
const current = computed<IslandItem | null>(() => items.value[index.value % Math.max(items.value.length, 1)] ?? null)

// —— 轮播 ——
let timer: ReturnType<typeof setInterval> | null = null
function restartCycle(): void {
  if (timer) clearInterval(timer)
  const seconds = Math.min(30, Math.max(2, settings.value.island_cycle_seconds || 4))
  timer = setInterval(() => {
    // 悬停时暂停，让用户看清详情。
    if (expanded.value || items.value.length <= 1) return
    index.value = (index.value + 1) % items.value.length
  }, seconds * 1000)
}
watch(() => settings.value.island_cycle_seconds, restartCycle)
watch(
  () => items.value.length,
  (length) => {
    if (index.value >= length) index.value = 0
  },
)

// —— 悬停 / 点击（后端推送）——
async function setExpanded(next: boolean): Promise<void> {
  if (expanded.value === next) return
  expanded.value = next
  try {
    await api.island.expand(next)
  } catch (error) {
    logger.error('island', '切换展开状态失败', error)
  }
}

onMounted(() => {
  restartCycle()
  void onAppEvent<boolean>(AppEvents.islandHover, (inside) => {
    logger.debug('island', inside ? '光标进入' : '光标离开')
    void setExpanded(inside)
  })
  void onAppEvent(AppEvents.islandClick, () => {
    logger.info('island', '点击，面板已由后端呼出')
    clicked.value = true
    setTimeout(() => (clicked.value = false), 300)
  })
})

onBeforeUnmount(() => {
  if (timer) clearInterval(timer)
})

/** 背景不透明度：CSS 变量，文字始终不透明。 */
const alpha = computed(() => Math.min(1, Math.max(0.3, settings.value.island_opacity || 0.85)))
</script>

<template>
  <div class="island" :class="{ expanded, clicked, empty: !current }" :style="{ '--island-alpha': alpha }">
    <!-- 收起态：单行胶囊，条目切换时向上滚动 -->
    <div v-if="!expanded" class="island-line">
      <Transition name="island-roll" mode="out-in">
        <div v-if="current" :key="current.id" class="island-row">
          <i class="island-dot" :style="{ background: current.dot ?? 'var(--text-dim)' }" />
          <span class="island-title">{{ current.title }}</span>
          <span v-if="current.meta" class="island-meta">{{ current.meta }}</span>
          <span v-if="items.length > 1" class="island-count">{{ (index % items.length) + 1 }}/{{ items.length }}</span>
        </div>
        <div v-else key="empty" class="island-row island-empty">
          <span class="island-title">{{ emptyText }}</span>
        </div>
      </Transition>
    </div>

    <!-- 展开态：当前条目的详情组件 -->
    <div v-else class="island-expanded">
      <component v-if="current?.detail" :is="current.detail" :item="current" />
      <div v-else class="island-detail">
        <div class="island-detail-title">{{ current?.title ?? emptyText }}</div>
      </div>
      <div class="island-hint">左键点击打开待办面板</div>
    </div>
  </div>
</template>
