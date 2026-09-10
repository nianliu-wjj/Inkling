<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import HeatTip from '@/components/stats/HeatTip.vue'
import TrendChart from '@/components/stats/TrendChart.vue'
import { useSettings } from '@/composables/useData'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { ActivityDay, MonthTrend } from '@/typings/domain'
import { alphaOf, buildRangeGrid, type HeatLevel, type RangeCell } from '@/utils/heatmap'

/**
 * 归档 · 统计页（原型 #archive-stats / renderHeatmap + renderTrend）。
 *
 * - 日历格子热力图：列为周（周一起）、行为星期，近 182 天，顶部标注月份范围，
 *   档位 <5 / <10 / <18 四档叠在主题令牌 --hm-base 上；存在逾期的日期红框（.ovd）；
 * - 悬浮明细 HeatTip 定位在格子上方居中（Teleport 到 body）；
 * - 图例「少 … 多 · 存在逾期」；
 * - 趋势图为原型的内联 SVG 折线（TrendChart），不依赖图表库。
 */

/** 热力图覆盖的天数，与后端 stats_heatmap 的默认值一致。 */
const HEATMAP_DAYS = 182
/** 一列（周）的像素宽：14px 格子 + 3px 间距（原型 STEP）。 */
const COLUMN_STEP = 17

const { settings } = useSettings()

const activity = ref<ActivityDay[]>([])
const trend = ref<MonthTrend[]>([])

/** 悬浮明细：格子矩形 + 当日数据（无记录的日期给全 0）。 */
const tip = ref<{ day: ActivityDay; anchor: DOMRect } | null>(null)

function readHmBase(): string {
  return getComputedStyle(document.documentElement).getPropertyValue('--hm-base').trim() || '108,140,255'
}

/** 热力图基色随主题变化：主题写入 DOM 后（post flush）再读令牌。 */
const hmBase = ref(readHmBase())
watch(
  () => settings.value.theme,
  () => {
    hmBase.value = readHmBase()
  },
  { flush: 'post' },
)

const grid = computed(() => buildRangeGrid(activity.value, { days: HEATMAP_DAYS }))

function background(level: HeatLevel): string | undefined {
  return level ? `rgba(${hmBase.value}, ${alphaOf(level)})` : undefined
}

function legendColor(level: HeatLevel): string {
  return `rgba(${hmBase.value}, ${alphaOf(level)})`
}

function dayOf(cell: RangeCell): ActivityDay {
  return cell.day ?? { date: cell.key, notes: 0, clips: 0, todos: 0, completed: 0, overdue: 0 }
}

function onEnter(event: MouseEvent, cell: RangeCell): void {
  tip.value = { day: dayOf(cell), anchor: (event.currentTarget as HTMLElement).getBoundingClientRect() }
}

async function load(): Promise<void> {
  try {
    const [days, months] = await Promise.all([api.stats.heatmap(HEATMAP_DAYS), api.stats.trend()])
    activity.value = days
    trend.value = months
    logger.info('stats', `加载统计数据：${days.length} 天 / ${months.length} 月`)
  } catch (error) {
    logger.error('stats', '加载统计数据失败', error)
  }
}

onMounted(load)
</script>

<template>
  <div class="archive-page">
    <div class="page-title">📊 使用统计</div>

    <div class="stats-legend">每日活跃度热力图（悬浮查看当日明细 · 红框 = 存在逾期待办）</div>

    <div class="heatmap-wrap" @mouseleave="tip = null">
      <!-- 顶部月份范围标签，与周列对齐 -->
      <div class="heat-months">
        <span v-for="m in grid.months" :key="m.label + m.column" :style="{ left: `${m.column * COLUMN_STEP}px` }">
          {{ m.label }}
        </span>
      </div>
      <div class="heat-flex">
        <div class="heat-weekdays"><span>一</span><span /><span /><span>四</span><span /><span /><span>日</span></div>
        <div class="heat-grid">
          <div
            v-for="cell in grid.cells"
            :key="cell.key"
            class="heat-cell"
            :class="{ ovd: cell.overdue }"
            :style="background(cell.level) ? { background: background(cell.level) } : undefined"
            :data-date="cell.key"
            @mouseenter="onEnter($event, cell)"
          />
        </div>
      </div>
      <div class="heat-legend">
        少
        <i v-for="level in [1, 2, 3, 4]" :key="level" :style="{ background: legendColor(level as HeatLevel) }" />
        多 <i class="lg-ovd" :style="{ background: `rgba(${hmBase}, .3)` }" /> 存在逾期
      </div>
    </div>

    <div class="stats-legend">近 6 个月趋势（各模块使用量折线）</div>
    <TrendChart :months="trend" />

    <Teleport to="body">
      <HeatTip v-if="tip" :day="tip.day" :anchor="tip.anchor" />
    </Teleport>
  </div>
</template>
