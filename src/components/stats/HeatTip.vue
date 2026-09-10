<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from 'vue'
import { enter } from '@/motion'
import type { ActivityDay } from '@/typings/domain'

/**
 * 热力图悬浮明细（原型 showHeatTip）：fixed 定位在格子上方居中，上方放不下则翻到下方，
 * 左右夹在视口内；存在逾期时红边。由父组件 Teleport 到 body，避免被滚动容器裁剪。
 * 样式全部来自生成层的 #heatTip 规则。
 */
const props = defineProps<{ day: ActivityDay; anchor: DOMRect }>()

const el = ref<HTMLElement | null>(null)
const style = ref<{ top: string; left: string }>({ top: '0px', left: '0px' })

const WEEK = ['日', '一', '二', '三', '四', '五', '六']

/** 日期键 → 中文星期（本地构造，避免被当作 UTC 解析）。 */
function weekdayOf(key: string): string {
  const [year, month, day] = key.split('-').map(Number)
  return WEEK[new Date(year, month - 1, day).getDay()] ?? ''
}

/** 先渲染再量尺寸：位置依赖浮层自身宽高。 */
async function place(): Promise<void> {
  await nextTick()
  const tip = el.value
  if (!tip) return
  const rect = props.anchor
  const width = tip.offsetWidth
  const height = tip.offsetHeight
  let top = rect.top - height - 8
  if (top < 8) top = rect.bottom + 8
  const left = Math.min(Math.max(8, rect.left + rect.width / 2 - width / 2), window.innerWidth - width - 8)
  style.value = { top: `${top}px`, left: `${left}px` }
}

watch(
  () => [props.anchor.top, props.anchor.left, props.day.date],
  () => void place(),
)

onMounted(async () => {
  await place()
  if (el.value) void enter(el.value, { axis: 'y', distance: 4 })
})
</script>

<template>
  <div id="heatTip" ref="el" :class="{ ovd: props.day.overdue > 0 }" :style="style">
    <div class="tip-title">{{ props.day.date }} 周{{ weekdayOf(props.day.date) }}</div>
    <div class="tip-row">
      📝 笔记 <b>{{ props.day.notes }}</b> 条
    </div>
    <div class="tip-row">
      📋 复制项 <b>{{ props.day.clips }}</b> 条
    </div>
    <div class="tip-row">
      ✅ 待办 <b>{{ props.day.todos }}</b> 条 · 已完成 <b>{{ props.day.completed }}</b>
      <template v-if="props.day.overdue > 0">
        · <span class="ovd-red">逾期 {{ props.day.overdue }}</span>
      </template>
    </div>
  </div>
</template>
