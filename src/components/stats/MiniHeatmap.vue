<script setup lang="ts">
import { computed } from 'vue'
import type { ActivityDay } from '@/typings/domain'
import { alphaOf, buildMonthGrid, type HeatLevel, type MonthCell } from '@/utils/heatmap'

/**
 * 侧边栏当月迷你热力图（原型 renderMiniHeat）：周一对齐、<3/<6/<10 四档、
 * 悬浮看明细（由父组件渲染 HeatTip）、点击查该日全部记录、选中日描边。
 * 折叠态由生成层 CSS（.archive-side.collapsed .mini-heat）隐藏。
 */
const props = defineProps<{ activity: readonly ActivityDay[]; selectedDate: string }>()

const emit = defineEmits<{
  (e: 'pick', key: string): void
  /** 悬浮进入格子时给出明细与格子矩形；离开整个热力图时两者为 null。 */
  (e: 'hover', day: ActivityDay | null, anchor: DOMRect | null): void
}>()

const now = new Date()
const title = `${now.getMonth() + 1}月活跃（悬浮明细 · 点击查当日）`

const cells = computed(() =>
  buildMonthGrid(props.activity, now.getFullYear(), now.getMonth(), { selected: props.selectedDate }),
)

/** 活跃度着色：透明度阶叠在主题令牌 --hm-base 上，色相随 32 套主题切换。 */
function background(level: HeatLevel): string | undefined {
  if (!level) return undefined
  const base = getComputedStyle(document.documentElement).getPropertyValue('--hm-base').trim() || '108,140,255'
  return `rgba(${base}, ${alphaOf(level)})`
}

/** 无记录的日期也要能悬浮：给一份全 0 的明细。 */
function dayOf(cell: MonthCell): ActivityDay {
  return cell.day ?? { date: cell.key, notes: 0, clips: 0, todos: 0, completed: 0, overdue: 0 }
}

function onEnter(event: MouseEvent, cell: MonthCell): void {
  emit('hover', dayOf(cell), (event.currentTarget as HTMLElement).getBoundingClientRect())
}
</script>

<template>
  <div class="mini-heat" @mouseleave="emit('hover', null, null)">
    <div class="mh-title">{{ title }}</div>
    <div class="mh-grid">
      <template v-for="cell in cells" :key="cell.key">
        <i v-if="cell.blank" class="mh-blank" />
        <span
          v-else
          class="heat-cell mh-cell"
          :class="{ ovd: cell.overdue, selected: cell.selected }"
          :style="background(cell.level) ? { background: background(cell.level) } : undefined"
          :data-date="cell.key"
          @mouseenter="onEnter($event, cell)"
          @click="emit('pick', cell.key)"
        />
      </template>
    </div>
  </div>
</template>
