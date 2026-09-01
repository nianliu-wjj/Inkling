<script setup lang="ts">
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { ActivityDay, MonthTrend } from '@/typings/domain'
import { toDateKey } from '@/utils/datetime'

/**
 * 归档 · 统计页。
 *
 * 需求 v1.2 变更 #3：
 * - 日历格子热力图（列=周、行=星期），顶部标注月份范围；
 * - 悬浮显示日期与明细（笔记 / 复制项 / 待办含已完成与逾期）；
 * - 存在逾期的日期格子以红色边框标识（.ovd）；
 * - 趋势图为折线图；统计页支持滚动。
 */
echarts.use([LineChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer])

/** 热力图覆盖的天数，与后端 stats_heatmap 的默认值一致。 */
const HEATMAP_DAYS = 182

const activity = ref<ActivityDay[]>([])
const trend = ref<MonthTrend[]>([])

const trendHost = ref<HTMLElement | null>(null)
let chart: echarts.ECharts | null = null

/** 悬浮提示：位置与内容。 */
const tip = ref<{ x: number; y: number; day: ActivityDay } | null>(null)

/** 读取主题令牌，保证图表颜色跟随 30 套主题。 */
function themeVar(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return value || fallback
}

/**
 * 组装日历网格：列为周、行为星期（GitHub 风格）。
 * 起点回退到最早日期所在周的周日，保证每列都是完整的一周。
 */
const grid = computed(() => {
  const byDate = new Map(activity.value.map((day) => [day.date, day]))
  const end = new Date()
  const start = new Date()
  start.setDate(end.getDate() - HEATMAP_DAYS + 1)
  // 回退到所在周的周日
  start.setDate(start.getDate() - start.getDay())

  const base = themeVar('--hm-base', '108,140,255')
  const cells: { key: string; day: ActivityDay | null; background: string }[] = []
  const months: { label: string; column: number }[] = []

  let column = 0
  const cursor = new Date(start)
  while (cursor <= end) {
    const key = toDateKey(cursor)
    const record = byDate.get(key) ?? null
    const total = record ? record.notes + record.clips + record.todos : 0
    const alpha = total === 0 ? 0 : total <= 2 ? 0.28 : total <= 5 ? 0.48 : total <= 9 ? 0.7 : 0.9

    // 每月第一次出现时记录列号，供顶部月份标签定位。
    if (cursor.getDate() <= 7 && cursor.getDay() === 0) {
      months.push({ label: `${cursor.getMonth() + 1}月`, column })
    }

    cells.push({
      key,
      day: record,
      background: alpha === 0 ? '' : `rgba(${base}, ${alpha})`,
    })

    if (cursor.getDay() === 6) column += 1
    cursor.setDate(cursor.getDate() + 1)
  }

  return { cells, months }
})

function showTip(event: MouseEvent, day: ActivityDay | null, key: string): void {
  tip.value = {
    x: event.clientX + 14,
    y: event.clientY + 14,
    day: day ?? { date: key, notes: 0, clips: 0, todos: 0, completed: 0, overdue: 0 },
  }
}

/** 渲染折线趋势图。 */
function renderTrend(): void {
  if (!trendHost.value) return
  chart ??= echarts.init(trendHost.value)

  const axisColor = themeVar('--text-dim', 'rgba(255,255,255,.5)')
  chart.setOption({
    grid: { left: 38, right: 16, top: 28, bottom: 24 },
    tooltip: { trigger: 'axis' },
    legend: {
      data: ['笔记', '粘贴板', '待办'],
      textStyle: { color: axisColor },
      right: 0,
      top: 0,
    },
    xAxis: {
      type: 'category',
      data: trend.value.map((m) => m.month),
      axisLine: { lineStyle: { color: axisColor } },
      axisLabel: { color: axisColor },
    },
    yAxis: {
      type: 'value',
      splitLine: { lineStyle: { color: 'rgba(128,128,128,.15)' } },
      axisLabel: { color: axisColor },
    },
    series: [
      {
        name: '笔记',
        type: 'line',
        smooth: true,
        data: trend.value.map((m) => m.notes),
        itemStyle: { color: themeVar('--trend-note', '#ff8a8a') },
      },
      {
        name: '粘贴板',
        type: 'line',
        smooth: true,
        data: trend.value.map((m) => m.clips),
        itemStyle: { color: themeVar('--trend-clip', '#ffd76e') },
      },
      {
        name: '待办',
        type: 'line',
        smooth: true,
        data: trend.value.map((m) => m.todos),
        itemStyle: { color: themeVar('--trend-todo', '#7ee0a8') },
      },
    ],
  })
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

function onResize(): void {
  chart?.resize()
}

onMounted(async () => {
  await load()
  renderTrend()
  window.addEventListener('resize', onResize)
})

watch(trend, renderTrend)

onBeforeUnmount(() => {
  window.removeEventListener('resize', onResize)
  chart?.dispose()
  chart = null
})
</script>

<template>
  <div class="archive-page">
    <div class="page-title">📊 使用统计</div>

    <div class="stats-legend">每日活跃度热力图（悬浮查看当日明细 · 红框 = 存在逾期待办）</div>

    <div class="heatmap-wrap">
      <!-- 顶部月份范围标签，与周列对齐 -->
      <div class="heat-months">
        <span v-for="m in grid.months" :key="m.label + m.column" :style="{ left: `${m.column * 17}px` }">
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
            :class="{ ovd: (cell.day?.overdue ?? 0) > 0 }"
            :style="cell.background ? { background: cell.background } : undefined"
            @mouseenter="showTip($event, cell.day, cell.key)"
            @mouseleave="tip = null"
          />
        </div>
      </div>
    </div>

    <div class="stats-legend">近 6 个月趋势（各模块使用量折线）</div>
    <div ref="trendHost" class="trend-chart" style="height: 220px" />

    <!-- 悬浮明细 -->
    <div
      v-if="tip"
      id="heatTip"
      :class="{ ovd: tip.day.overdue > 0 }"
      :style="{ left: `${tip.x}px`, top: `${tip.y}px` }"
    >
      <div class="tip-title">{{ tip.day.date }}</div>
      <div class="tip-row">
        笔记 <b>{{ tip.day.notes }}</b> 条
      </div>
      <div class="tip-row">
        复制项 <b>{{ tip.day.clips }}</b> 条
      </div>
      <div class="tip-row">
        待办 <b>{{ tip.day.todos }}</b> 条（已完成 <b>{{ tip.day.completed }}</b>
        <span v-if="tip.day.overdue > 0" class="ovd-red"> · 逾期 {{ tip.day.overdue }}</span
        >）
      </div>
    </div>
  </div>
</template>
