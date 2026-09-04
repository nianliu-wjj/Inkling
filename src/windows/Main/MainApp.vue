<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import WindowControls from '@/components/base/WindowControls.vue'
import { controlsOnLeft } from '@/constants/platform'
import { useClips, useNotes, useSettings, useTodos } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { AppEvents, onAppEvent } from '@/service/events'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { ActivityDay, View } from '@/typings/domain'
import { todayKey } from '@/utils/datetime'
import ClipsView from './ClipsView.vue'
import DayView from './DayView.vue'
import NotesView from './NotesView.vue'
import Sidebar from './Sidebar.vue'
import SettingsView from './SettingsView.vue'
import StatsView from './StatsView.vue'
import TodosView from './TodosView.vue'

/**
 * 归档主窗口（v1.2 变更 #4：单窗口左右结构）。
 *
 * 左侧边栏切换视图，右侧主内容区展示；统计与偏好设置也在同一窗口内，
 * 不再弹独立窗口。响应托盘/快捷键发来的 inkling://navigate 事件。
 */

// 启动瞬间先用缓存主题上色，避免闪一下默认深色。
applyCachedTheme()
applyCachedGlass()

const { notes } = useNotes()
const { clips } = useClips()
const { todos } = useTodos()
const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()

const view = ref<View | 'day'>('notes')
const selectedDate = ref('')
const activity = ref<ActivityDay[]>([])

/** 侧边栏计数徽章：草稿不计入笔记数，子任务不计入待办数。 */
const counts = computed(() => ({
  notes: notes.value.filter((n) => !n.is_draft).length,
  clips: clips.value.length,
  todos: todos.value.filter((t) => !t.parent_id && t.status === 'open').length,
}))

/** 主题跟随设置变化。 */
watch(() => settings.value.theme, applyTheme, { immediate: true })
// 玻璃质感与主题同源：后端设置变化时一并同步。
watch(() => settings.value.glass_level, applyGlass, { immediate: true })

/** 毛玻璃开关：同步根属性，供 base.css 的降级规则使用。 */
watch(
  () => settings.value.main_acrylic,
  (enabled) => {
    const root = document.documentElement
    if (enabled) root.removeAttribute('data-acrylic')
    else root.setAttribute('data-acrylic', 'off')
  },
  { immediate: true },
)

/** 当月活跃度：供侧边栏迷你热力图使用。 */
async function loadActivity(): Promise<void> {
  try {
    activity.value = await api.stats.heatmap(62)
  } catch (error) {
    logger.error('main', '加载活跃度失败', error)
  }
}

function navigate(next: View | 'day'): void {
  logger.info('main', `切换视图 → ${next}`)
  view.value = next
}

function pickDate(dateKey: string): void {
  selectedDate.value = dateKey
  view.value = 'day'
}

onMounted(() => {
  void loadActivity()

  // 托盘菜单 / 快捷键请求切换视图。
  void onAppEvent<string>(AppEvents.navigate, (target) => {
    if (!target) return
    navigate(target as View)
  })

  // 数据变化后刷新热力图。
  void onAppEvent(AppEvents.statsChanged, () => void loadActivity())

  logger.info('main', '归档主窗口已挂载')
})
</script>

<template>
  <div id="mainWindow" class="app-window glass">
    <!-- 标题栏：控件位置随平台——macOS 三点在左，Windows 三键在右（见 constants/platform.ts）。
         整条是拖拽区，控件内部用 .no-drag 排除。 -->
    <div class="window-titlebar" :class="{ 'controls-left': controlsOnLeft }" data-tauri-drag-region>
      <WindowControls v-if="controlsOnLeft" />
      <span class="win-title"><span class="app-ico">✒️</span> Inkling</span>
      <WindowControls v-if="!controlsOnLeft" />
    </div>

    <div class="archive-layout">
      <Sidebar
        :view="view"
        :counts="counts"
        :activity="activity"
        :selected-date="selectedDate"
        @navigate="navigate"
        @pick-date="pickDate"
      />

      <main class="archive-main">
        <NotesView v-if="view === 'notes'" />
        <ClipsView v-else-if="view === 'clips'" />
        <TodosView v-else-if="view === 'todos'" />
        <StatsView v-else-if="view === 'stats'" />
        <SettingsView v-else-if="view === 'settings'" />
        <DayView v-else-if="view === 'day'" :date-key="selectedDate || todayKey()" />
      </main>
    </div>

    <ToastHost />
  </div>
</template>
