<script setup lang="ts">
import { computed } from 'vue'
import { useSettings, useTodos } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { builtinIslandPlugins, resolveIslandPlugins, serializeIslandPlugins } from '@/island-plugins'
import { pickIslandTodos } from '@/utils/island'
import { logger } from '@/service/logger'
import type { IslandScope, Settings } from '@/typings/domain'
import { todayKey } from '@/utils/datetime'

/**
 * 归档 · 灵动岛页（原型 #archive-island）。
 *
 * 标题 + 说明 + 静态预览（与屏幕顶部胶囊同源：展示当前播放的第一条待办，复用原型 .di-item 类）
 * + 状态行 + 「灵动岛设置」八项（原型顺序：启用 / 停留 / 播放范围 / 透明度 / 点击穿透 / 悬停穿透 / 流光 / 全屏隐藏）
 * + 项目扩展（宽 / 高 / 插件，排在其后）。预览口径与胶囊同源（utils/island.ts）。
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

/** 播放口径与胶囊同源（utils/island.ts：含子任务、按播放范围、逾期优先）。 */
const islandRows = computed(() => pickIslandTodos(todos.value, settings.value.island_scope))
const first = computed(() => islandRows.value[0] ?? null)
/** 预览里非当天条目带 MM-DD 徽章（原型 .di-date）。 */
const firstDate = computed(() => (first.value && first.value.date !== todayKey() ? first.value.date.slice(5) : ''))

/** 状态行（原型 islandPageStat：播放条数 · 停留 · 范围 · 停用 · 点击穿透 · 悬停穿透）。 */
const stat = computed(() => {
  const parts = [
    `当前播放 ${islandRows.value.length} 条待办`,
    `每条停留 ${settings.value.island_cycle_seconds} 秒`,
    `范围：${settings.value.island_scope === 'all' ? '全部未完成待办' : '仅当天待办'}`,
  ]
  if (!settings.value.island_enabled) parts.push('灵动岛已停用')
  if (settings.value.island_click_through) parts.push('点击穿透')
  if (settings.value.island_pass_hover) parts.push('悬停穿透')
  return parts.join(' · ')
})

/** 灵动岛数值项的允许范围（原型 DI.W_MIN/W_MAX/H_MIN/H_MAX 与停留 1–10；与 Rust island_clamp 一致，前端先钳一遍避免来回抖动）。 */
const ISLAND_LIMITS = {
  width: { min: 200, max: 480 },
  height: { min: 32, max: 56 },
  opacity: { min: 0.3, max: 1 },
  cycle: { min: 1, max: 10 },
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
            <span class="di-text"
              ><span v-if="firstDate" class="di-date">{{ firstDate }}</span
              >{{ first.text }}</span
            >
            <span class="di-time" :class="{ ovd: first.overdue }">{{
              first.overdue ? `逾期 ${first.time}` : first.time
            }}</span>
          </template>
          <template v-else>
            <span class="di-dot idle" />
            <span class="di-text dim">{{
              settings.island_scope === 'all' ? '没有未完成待办 🎉' : '今日待办已全部完成 🎉'
            }}</span>
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
        <span>播放范围</span>
        <select
          :value="settings.island_scope"
          @change="patch({ island_scope: ($event.target as HTMLSelectElement).value as IslandScope })"
        >
          <option value="today">仅当天待办（默认）</option>
          <option value="all">全部未完成待办</option>
        </select>
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
        <span>悬停穿透（悬停不展开详情、不暂停轮播）</span>
        <input
          type="checkbox"
          :checked="settings.island_pass_hover"
          @change="patch({ island_pass_hover: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>流光边框</span>
        <input
          type="checkbox"
          :checked="settings.island_glow"
          @change="patch({ island_glow: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>全屏自动隐藏</span>
        <input
          type="checkbox"
          :checked="settings.island_auto_hide"
          @change="patch({ island_auto_hide: ($event.target as HTMLInputElement).checked })"
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
