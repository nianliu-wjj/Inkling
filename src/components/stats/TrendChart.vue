<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { logger } from '@/service/logger'
import type { MonthTrend } from '@/typings/domain'

/**
 * 近 6 个月趋势折线（原型 renderTrend 的内联 SVG）：
 * 620×190 视口，网格 4 条（0 / ⅓ / ⅔ / max，max 向上取整到 10），三条折线 + 数据点 title 悬浮，
 * 图例右对齐；颜色读主题令牌 --trend-*，主题切换后重取。不依赖图表库。
 */
const props = defineProps<{ months: readonly MonthTrend[] }>()

const { settings } = useSettings()

const W = 620
const H = 190
const P = { l: 40, r: 12, t: 24, b: 28 } as const
const iw = W - P.l - P.r
const ih = H - P.t - P.b

function themeVar(name: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback
}

function readColors() {
  return {
    note: themeVar('--trend-note', '#ff8a8a'),
    clip: themeVar('--trend-clip', '#ffd76e'),
    todo: themeVar('--trend-todo', '#7ee0a8'),
    dim: themeVar('--text-dim', 'rgba(255,255,255,.5)'),
    wsa: themeVar('--wsa', '255,255,255'),
  }
}

/** 颜色随主题变化：主题写入 DOM 后（post flush）再读令牌。 */
const colors = ref(readColors())
watch(
  () => settings.value.theme,
  (theme) => {
    colors.value = readColors()
    logger.debug('trend-chart', `主题 ${theme} → 重取折线颜色`)
  },
  { flush: 'post' },
)

type SeriesKey = 'notes' | 'clips' | 'todos'
const series = computed<{ key: SeriesKey; label: string; color: string }[]>(() => [
  { key: 'notes', label: '笔记', color: colors.value.note },
  { key: 'clips', label: '粘贴板', color: colors.value.clip },
  { key: 'todos', label: '待办', color: colors.value.todo },
])

/** y 轴上限：不低于 10，向上取整到 10 的倍数。 */
const niceMax = computed(() => {
  const max = Math.max(10, ...props.months.flatMap((m) => [m.notes, m.clips, m.todos]))
  return Math.ceil(max / 10) * 10
})

function x(index: number): number {
  const count = props.months.length
  return P.l + (count <= 1 ? iw / 2 : (index * iw) / (count - 1))
}
function y(value: number): number {
  return P.t + ih - (value / niceMax.value) * ih
}

const grid = computed(() =>
  [0, 1, 2, 3].map((g) => {
    const value = (niceMax.value * g) / 3
    return { value, y: y(value) }
  }),
)

function points(key: SeriesKey): string {
  return props.months.map((m, i) => `${x(i)},${y(m[key])}`).join(' ')
}

/** "2026-09" → "9月"。 */
function monthLabel(month: string): string {
  return `${Number(month.slice(5))}月`
}
</script>

<template>
  <div class="trend-chart">
    <div class="trend-legend">
      <span v-for="s in series" :key="s.key"><i :style="{ background: s.color }" />{{ s.label }}</span>
    </div>
    <svg :viewBox="`0 0 ${W} ${H}`" preserveAspectRatio="xMidYMid meet" role="img" aria-label="月度趋势折线图">
      <template v-for="g in grid" :key="g.value">
        <line :x1="P.l" :y1="g.y" :x2="W - P.r" :y2="g.y" :stroke="`rgba(${colors.wsa},.09)`" />
        <text :x="P.l - 8" :y="g.y + 3.5" text-anchor="end" font-size="10" :fill="colors.dim">
          {{ Math.round(g.value) }}
        </text>
      </template>
      <template v-for="s in series" :key="s.key">
        <polyline
          :points="points(s.key)"
          fill="none"
          :stroke="s.color"
          stroke-width="2"
          stroke-linejoin="round"
          stroke-linecap="round"
        />
        <circle v-for="(m, i) in props.months" :key="m.month" :cx="x(i)" :cy="y(m[s.key])" r="3.2" :fill="s.color">
          <title>{{ m.month }} · {{ s.label }}：{{ m[s.key] }}</title>
        </circle>
      </template>
      <text
        v-for="(m, i) in props.months"
        :key="m.month"
        :x="x(i)"
        :y="H - 8"
        text-anchor="middle"
        font-size="10.5"
        :fill="colors.dim"
      >
        {{ monthLabel(m.month) }}
      </text>
    </svg>
  </div>
</template>
