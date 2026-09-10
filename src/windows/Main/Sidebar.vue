<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import HeatTip from '@/components/stats/HeatTip.vue'
import MiniHeatmap from '@/components/stats/MiniHeatmap.vue'
import { navigationItems } from '@/constants/navigation'
import { logger } from '@/service/logger'
import type { ActivityDay, View } from '@/typings/domain'

/**
 * 归档主窗口侧边栏（原型 #archiveSide）。
 *
 * - 五个页签（笔记 / 粘贴板 / 待办 / 启动台 / 灵动岛）+ 计数徽章 + 选中指示条，
 *   分类色由生成层按 data-view 提供；页签为 role="tab"，Enter / Space 可触发；
 * - 底部左 ⚙️ 偏好设置、右 📊 统计，同样在右侧主内容区展示；
 * - 支持拖动分隔条实时调宽（110~280px）；拖至阈值以下自动折叠为 52px 图标窄栏；
 * - 当月迷你热力图（MiniHeatmap）：悬浮看明细（HeatTip 传送到 body）、点击查该日全部记录。
 */
const props = defineProps<{
  view: View | 'day'
  /** 计数口径与原型一致：笔记总数（不含草稿）/ 粘贴板总数 / 待办含子任务与已完成。 */
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

/** 键盘可达：role="tab" 的 div 原生不响应 Enter / Space，这里补上（原型 keydown 委托）。 */
function onKeyActivate(event: KeyboardEvent, view: View): void {
  event.preventDefault()
  emit('navigate', view)
}

/** 迷你热力图悬浮明细：由本组件 Teleport 到 body 渲染，避免被侧栏裁剪。 */
const tip = ref<{ day: ActivityDay; anchor: DOMRect } | null>(null)
function onHeatHover(day: ActivityDay | null, anchor: DOMRect | null): void {
  tip.value = day && anchor ? { day, anchor } : null
}

onMounted(() => logger.info('sidebar', '侧边栏已挂载'))
onBeforeUnmount(onDragEnd)
</script>

<template>
  <aside id="archiveSide" class="archive-side" :class="{ collapsed }" :style="style">
    <div class="side-nav" role="tablist" aria-label="数据视图切换">
      <div
        v-for="item in navigationItems"
        :key="item.key"
        class="side-item"
        :class="{ active: props.view === item.key }"
        :data-view="item.key"
        role="tab"
        tabindex="0"
        :aria-selected="props.view === item.key"
        :title="item.label"
        @click="emit('navigate', item.key)"
        @keydown.enter="onKeyActivate($event, item.key)"
        @keydown.space="onKeyActivate($event, item.key)"
      >
        <span class="si-icon"
          ><span class="ix">{{ item.icon }}</span></span
        >
        <span class="si-label">{{ item.label }}</span>
        <span class="si-count">{{ item.countKey ? props.counts[item.countKey] || '' : '' }}</span>
      </div>
    </div>

    <!-- 当月迷你热力图：折叠态由 CSS 隐藏 -->
    <MiniHeatmap
      :activity="props.activity"
      :selected-date="props.selectedDate"
      @pick="emit('pick-date', $event)"
      @hover="onHeatHover"
    />

    <div class="side-foot">
      <button
        type="button"
        class="icon-btn side-btn"
        :class="{ active: props.view === 'settings' }"
        title="偏好设置"
        @click="emit('navigate', 'settings')"
      >
        <span class="ix">⚙️</span>
      </button>
      <button
        type="button"
        class="icon-btn side-btn"
        :class="{ active: props.view === 'stats' }"
        title="统计数据"
        @click="emit('navigate', 'stats')"
      >
        <span class="ix">📊</span>
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
      :title="collapsed ? '展开侧边栏（恢复默认宽度）' : '折叠侧边栏'"
      @click.stop="toggleCollapse"
    >
      {{ collapsed ? '»' : '«' }}
    </button>
  </div>

  <Teleport to="body">
    <HeatTip v-if="tip" :day="tip.day" :anchor="tip.anchor" />
  </Teleport>
</template>
