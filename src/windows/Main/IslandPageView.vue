<script setup lang="ts">
import { computed } from 'vue'
import { useSettings, useTodos } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { builtinIslandPlugins, resolveIslandPlugins, serializeIslandPlugins } from '@/island-plugins'
import { pickTodayTodos } from '@/island-plugins/today-todos'
import { logger } from '@/service/logger'
import type { Settings } from '@/typings/domain'
import { formatClock } from '@/utils/datetime'
import { isOverdue } from '@/utils/todo'

/**
 * 归档 · 灵动岛页（原型 #archive-island）。
 *
 * 标题 + 说明 + 静态预览（与屏幕顶部胶囊同源：展示当前播放的第一条待办，复用原型 .di-item 类）
 * + 状态行 + 「灵动岛设置」：启用 / 轮播间隔 / 透明度 / 点击穿透 / 宽高 / 插件（从偏好设置页迁入）。
 * 原型中的播放范围、悬停穿透、流光边框、全屏自动隐藏当前后端没有对应能力，不渲染（spec D10，留阶段四）。
 */
const { settings, save } = useSettings()
const { todos } = useTodos()
const { toast } = useToast()

/** 统一的保存入口：局部覆盖后整体写回。 */
async function patch(partial: Partial<Settings>): Promise<void> {
  const next: Settings = { ...settings.value, ...partial }
  try {
    await save(next)
  } catch {
    toast('保存设置失败')
  }
}

/** 当日待办口径与胶囊一致（顶级、未完成、今天或逾期，按完成时间升序）。 */
const todayTodos = computed(() => pickTodayTodos(todos.value))
const first = computed(() => todayTodos.value[0] ?? null)

/** 状态行（原型 islandPageStat）。 */
const stat = computed(() => {
  const parts = [`当前播放 ${todayTodos.value.length} 条待办`, `每条停留 ${settings.value.island_cycle_seconds} 秒`]
  if (!settings.value.island_enabled) parts.push('灵动岛已停用')
  if (settings.value.island_click_through) parts.push('点击穿透')
  return parts.join(' · ')
})

/** 灵动岛尺寸等数值项的允许范围（与 Rust 侧 island_clamp 一致，前端先钳一遍避免来回抖动）。 */
const ISLAND_LIMITS = {
  width: { min: 200, max: 800 },
  height: { min: 28, max: 72 },
  opacity: { min: 0.3, max: 1 },
  cycle: { min: 2, max: 30 },
} as const

function clampNumber(value: number, range: { min: number; max: number }, fallback: number): number {
  if (!Number.isFinite(value)) return fallback
  return Math.min(range.max, Math.max(range.min, value))
}

/** 灵动岛数值设置：解析输入、钳制范围后保存。 */
function patchIslandNumber(
  key: 'island_width' | 'island_height' | 'island_opacity' | 'island_cycle_seconds',
  raw: string,
): void {
  const range =
    key === 'island_width'
      ? ISLAND_LIMITS.width
      : key === 'island_height'
        ? ISLAND_LIMITS.height
        : key === 'island_opacity'
          ? ISLAND_LIMITS.opacity
          : ISLAND_LIMITS.cycle
  const value = clampNumber(Number(raw), range, settings.value[key])
  logger.info('island-page', `灵动岛 ${key} = ${value}`)
  void patch({ [key]: value } as Partial<Settings>)
}

/** 当前启用的灵动岛插件 id 集合。 */
const enabledIslandPluginIds = computed(
  () => new Set(resolveIslandPlugins(settings.value.island_plugins).map((plugin) => plugin.id)),
)

/** 启用 / 禁用某个灵动岛插件；至少保留一个（空串会被 resolveIslandPlugins 兜回全部启用）。 */
function toggleIslandPlugin(id: string, enabled: boolean): void {
  const next = new Set(enabledIslandPluginIds.value)
  if (enabled) next.add(id)
  else next.delete(id)
  if (next.size === 0) {
    toast('至少需要启用一个灵动岛插件')
    return
  }
  const ordered = builtinIslandPlugins.filter((plugin) => next.has(plugin.id))
  logger.info('island-page', `灵动岛插件启用列表 = ${serializeIslandPlugins(ordered)}`)
  void patch({ island_plugins: serializeIslandPlugins(ordered) })
}
</script>

<template>
  <div class="archive-page">
    <div class="page-title">🏝️ 灵动岛</div>
    <div class="stats-legend">
      主屏顶部居中的胶囊：轮播当日待办，悬停查看详情，左键点击唤出面板到待办页；开启点击穿透后点击直接穿到桌面
    </div>

    <!-- 预览：与真实胶囊同源渲染当前播放的第一条 -->
    <div class="island-preview-wrap">
      <div class="island-preview" id="islandPreview">
        <div class="di-item island-preview-item">
          <template v-if="first">
            <span class="di-dot" :class="first.priority" />
            <span class="di-text">{{ first.content }}</span>
            <span class="di-time" :class="{ ovd: isOverdue(first) }">{{ formatClock(first.due_at) }}</span>
          </template>
          <template v-else>
            <span class="di-dot idle" />
            <span class="di-text dim">今日待办已全部完成 🎉</span>
          </template>
        </div>
      </div>
      <div class="island-preview-hint">
        预览与屏幕顶部胶囊同源渲染（展示当前播放的第一条）；实际胶囊位于屏幕顶部中央
      </div>
      <div class="island-page-stat">{{ stat }}</div>
    </div>

    <div class="page-title page-sub-title">⚙️ 灵动岛设置</div>
    <div class="settings-body">
      <label class="setting-row">
        <span>启用灵动岛</span>
        <input
          type="checkbox"
          :checked="settings.island_enabled"
          @change="patch({ island_enabled: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>滚动停留时间（秒，{{ ISLAND_LIMITS.cycle.min }}–{{ ISLAND_LIMITS.cycle.max }}）</span>
        <input
          type="number"
          :min="ISLAND_LIMITS.cycle.min"
          :max="ISLAND_LIMITS.cycle.max"
          :value="settings.island_cycle_seconds"
          @change="patchIslandNumber('island_cycle_seconds', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label class="setting-row">
        <span>透明度 {{ Math.round(settings.island_opacity * 100) }}%</span>
        <input
          type="range"
          :min="ISLAND_LIMITS.opacity.min"
          :max="ISLAND_LIMITS.opacity.max"
          step="0.05"
          :value="settings.island_opacity"
          @change="patchIslandNumber('island_opacity', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label class="setting-row">
        <span>点击穿透（点击落到下层应用，悬停仍有效）</span>
        <input
          type="checkbox"
          :checked="settings.island_click_through"
          @change="patch({ island_click_through: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>宽度（{{ ISLAND_LIMITS.width.min }}–{{ ISLAND_LIMITS.width.max }}）</span>
        <input
          type="number"
          :min="ISLAND_LIMITS.width.min"
          :max="ISLAND_LIMITS.width.max"
          :value="settings.island_width"
          @change="patchIslandNumber('island_width', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label class="setting-row">
        <span>高度（{{ ISLAND_LIMITS.height.min }}–{{ ISLAND_LIMITS.height.max }}）</span>
        <input
          type="number"
          :min="ISLAND_LIMITS.height.min"
          :max="ISLAND_LIMITS.height.max"
          :value="settings.island_height"
          @change="patchIslandNumber('island_height', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label v-for="plugin in builtinIslandPlugins" :key="plugin.id" class="setting-row">
        <span>🧩 {{ plugin.label }}</span>
        <input
          type="checkbox"
          :checked="enabledIslandPluginIds.has(plugin.id)"
          @change="toggleIslandPlugin(plugin.id, ($event.target as HTMLInputElement).checked)"
        />
      </label>
    </div>
  </div>
</template>
