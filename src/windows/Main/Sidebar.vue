<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { logger } from '@/service/logger'
import type { ActivityDay, View } from '@/typings/domain'
import { toDateKey } from '@/utils/datetime'

/**
 * 归档主窗口侧边栏。
 *
 * 需求 v1.2 变更 #4/#5：
 * - 三个页签（笔记/粘贴板/待办）+ 计数徽章 + 选中指示条，色系分别为蓝/金/绿；
 * - 底部左 ⚙️ 偏好设置、右 📊 统计，同样在右侧主内容区展示；
 * - 支持拖动分隔条实时调宽（110~280px）；拖至阈值以下自动折叠为 52px 图标窄栏；
 * - 当月迷你热力图，悬浮看明细、点击查该日全部记录。
 */
const props = defineProps<{
  view: View | 'day'
  counts: { notes: number; clips: number; todos: number }
  /** 当月活跃度，用于迷你热力图。 */
  activity: readonly ActivityDay[]
  /** 当前选中的日期（日期详情视图）。 */
  selectedDate: string
}>()

const emit = defineEmits<{
  (e: 'navigate', view: View | 'day'): void
  (e: 'pick-date', dateKey: string): void
}>()

/** 宽度范围与折叠阈值（需求指定）。 */
const MIN_WIDTH = 110
const MAX_WIDTH = 280
const DEFAULT_WIDTH = 150
const COLLAPSED_WIDTH = 52
const STORAGE_KEY = 'inkling-sidebar-width'

const width = ref(DEFAULT_WIDTH)
const collapsed = ref(false)
const dragging = ref(false)

/** 折叠态用固定窄栏宽度，展开态用用户设定宽度。 */
const style = computed(() => ({ width: `${collapsed.value ? COLLAPSED_WIDTH : width.value}px` }))

const NAV: readonly { key: View; icon: string; label: string; countKey: keyof typeof props.counts }[] = [
  { key: 'notes', icon: '📝', label: '笔记', countKey: 'notes' },
  { key: 'clips', icon: '📋', label: '粘贴板', countKey: 'clips' },
  { key: 'todos', icon: '✅', label: '待办', countKey: 'todos' },
]

/** 恢复上次的宽度与折叠态。 */
function restore(): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return
    const value = Number(raw)
    if (value === COLLAPSED_WIDTH) {
      collapsed.value = true
      return
    }
    if (value >= MIN_WIDTH && value <= MAX_WIDTH) width.value = value
  } catch (error) {
    logger.warn('sidebar', '读取侧边栏宽度失败', error)
  }
}
restore()

function persist(): void {
  try {
    localStorage.setItem(STORAGE_KEY, String(collapsed.value ? COLLAPSED_WIDTH : width.value))
  } catch (error) {
    logger.warn('sidebar', '写入侧边栏宽度失败', error)
  }
}

function onDragMove(event: MouseEvent): void {
  const next = event.clientX
  // 拖到阈值以下自动折叠为图标窄栏。
  if (next < MIN_WIDTH - 20) {
    collapsed.value = true
    return
  }
  collapsed.value = false
  width.value = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, next))
}

function onDragEnd(): void {
  dragging.value = false
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup', onDragEnd)
  document.body.style.cursor = ''
  persist()
  logger.debug('sidebar', `拖宽结束 width=${width.value} collapsed=${collapsed.value}`)
}

function onDragStart(): void {
  dragging.value = true
  document.addEventListener('mousemove', onDragMove)
  document.addEventListener('mouseup', onDragEnd)
  // 拖拽期间锁定光标，避免掠过文本时变成 I 形。
  document.body.style.cursor = 'col-resize'
}

/** « / » 按钮：折叠或恢复默认宽度。 */
function toggleCollapse(): void {
  collapsed.value = !collapsed.value
  if (!collapsed.value) width.value = DEFAULT_WIDTH
  persist()
}

// ── 当月迷你热力图 ──

/**
 * 活跃度着色：四档透明度叠加在主题的 --hm-base 上。
 * 与原型一致——用内联背景而非预设类，因为色相要跟随 30 套主题切换。
 * total 为 0 时返回空串，落回 CSS 里 .heat-cell 的默认底色。
 */
function heatBackground(total: number): string {
  if (total === 0) return ''
  const base = getComputedStyle(document.documentElement).getPropertyValue('--hm-base').trim()
  const alpha = total <= 2 ? 0.28 : total <= 5 ? 0.48 : total <= 9 ? 0.7 : 0.9
  return `rgba(${base || '108,140,255'}, ${alpha})`
}

/** 当月第一天是星期几（0=周日），用于补齐首列空格。 */
const monthGrid = computed(() => {
  const now = new Date()
  const first = new Date(now.getFullYear(), now.getMonth(), 1)
  const daysInMonth = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate()

  const byDate = new Map(props.activity.map((day) => [day.date, day]))
  const cells: { key: string; blank: boolean; background: string; overdue: boolean }[] = []

  // 首列补空：让第一天落在正确的星期行上。
  for (let i = 0; i < first.getDay(); i += 1) {
    cells.push({ key: `blank-${i}`, blank: true, background: '', overdue: false })
  }

  for (let day = 1; day <= daysInMonth; day += 1) {
    const key = toDateKey(new Date(now.getFullYear(), now.getMonth(), day))
    const record = byDate.get(key)
    const total = record ? record.notes + record.clips + record.todos : 0
    cells.push({
      key,
      blank: false,
      background: heatBackground(total),
      overdue: (record?.overdue ?? 0) > 0,
    })
  }

  return cells
})

const monthLabel = computed(() => `${new Date().getMonth() + 1} 月活跃度`)

onMounted(() => logger.info('sidebar', '侧边栏已挂载'))
onBeforeUnmount(onDragEnd)
</script>

<template>
  <aside class="archive-side" :class="{ collapsed }" :style="style">
    <div class="side-nav">
      <div
        v-for="item in NAV"
        :key="item.key"
        class="side-item"
        :class="{ active: props.view === item.key }"
        :data-view="item.key"
        :title="item.label"
        @click="emit('navigate', item.key)"
      >
        <span class="si-icon">{{ item.icon }}</span>
        <span class="si-label">{{ item.label }}</span>
        <span class="si-count">{{ props.counts[item.countKey] || '' }}</span>
      </div>
    </div>

    <!-- 当月迷你热力图：折叠态由 CSS 隐藏 -->
    <div class="mini-heat">
      <div class="mh-title">{{ monthLabel }}</div>
      <div class="mh-grid">
        <template v-for="cell in monthGrid" :key="cell.key">
          <span v-if="cell.blank" class="mh-blank" />
          <span
            v-else
            class="heat-cell"
            :class="{ ovd: cell.overdue, selected: props.selectedDate === cell.key }"
            :style="cell.background ? { background: cell.background } : undefined"
            :title="cell.key"
            @click="emit('pick-date', cell.key)"
          />
        </template>
      </div>
    </div>

    <div class="side-foot">
      <button
        type="button"
        class="icon-btn side-btn"
        :class="{ active: props.view === 'settings' }"
        title="偏好设置"
        @click="emit('navigate', 'settings')"
      >
        ⚙️
      </button>
      <button
        type="button"
        class="icon-btn side-btn"
        :class="{ active: props.view === 'stats' }"
        title="统计数据"
        @click="emit('navigate', 'stats')"
      >
        📊
      </button>
    </div>
  </aside>

  <!-- 拖宽分隔条 + 折叠按钮 -->
  <div
    class="side-resizer"
    :class="{ dragging }"
    title="拖动调整宽度（拖过阈值自动折叠）"
    @mousedown.prevent="onDragStart"
  >
    <button
      type="button"
      class="icon-btn side-toggle"
      :title="collapsed ? '展开侧边栏' : '折叠侧边栏'"
      @click.stop="toggleCollapse"
    >
      {{ collapsed ? '»' : '«' }}
    </button>
  </div>
</template>
