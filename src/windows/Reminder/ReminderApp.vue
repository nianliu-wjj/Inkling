<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onMounted, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 待办提醒卡片（需求 2.2「提醒」）。
 *
 * 到期时在屏幕右上角弹出（非系统通知），支持：
 * - 直接关闭 = 稍后不再提醒（后端置 remind_off）；
 * - 下拉选择下次提醒时间（只改下一次 remind_at，不改计划完成时间）。
 */
applyCachedTheme()
applyCachedGlass()

const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
watch(() => settings.value.theme, applyTheme, { immediate: true })
// 玻璃质感与主题同源：后端设置变化时一并同步。
watch(() => settings.value.glass_level, applyGlass, { immediate: true })

const label = getCurrentWindow().label
/** 窗口 label 形如 reminder-{todoId}。 */
const todoId = computed(() => label.replace(/^reminder-/, ''))

const content = ref('')

/** 顺延选项：分钟数，或 tomorrow 表示明天上午 9:00。 */
const SNOOZE_OPTIONS = [
  { value: 'done', label: '已完成' },
  { value: '10', label: '10 分钟后' },
  { value: '30', label: '30 分钟后' },
  { value: '60', label: '1 小时后' },
  { value: '180', label: '3 小时后' },
  { value: 'tomorrow', label: '明天上午 9:00' },
] as const

async function load(): Promise<void> {
  try {
    const todos = await api.todos.list()
    content.value = todos.find((t) => t.id === todoId.value)?.content ?? '（该待办已被删除）'
    logger.info('reminder', `提醒卡片已加载 ${todoId.value}`)
  } catch (error) {
    logger.error('reminder', '加载待办失败', error)
  }
}

/** 关闭 = 稍后不再提醒。 */
async function dismiss(): Promise<void> {
  logger.info('reminder', `关闭提醒（不再提醒） ${todoId.value}`)
  try {
    await api.todos.dismissReminder(todoId.value)
  } catch (error) {
    logger.error('reminder', '关闭提醒失败', error)
  }
  await api.windows.reminderClose(todoId.value).catch(() => undefined)
}

async function snooze(event: Event): Promise<void> {
  const value = (event.target as HTMLSelectElement).value
  if (!value) return

  if (value === 'done') {
    try {
      await api.todos.complete(todoId.value, true)
    } catch (error) {
      logger.error('reminder', '完成待办失败', error)
    }
    await api.windows.reminderClose(todoId.value).catch(() => undefined)
    return
  }

  // 明天上午 9:00 换算成距现在的分钟数，统一走 snooze 接口。
  let minutes: number
  if (value === 'tomorrow') {
    const target = new Date()
    target.setDate(target.getDate() + 1)
    target.setHours(9, 0, 0, 0)
    minutes = Math.max(1, Math.round((target.getTime() - Date.now()) / 60_000))
  } else {
    minutes = Number(value)
  }

  logger.info('reminder', `顺延提醒 ${minutes} 分钟`)
  try {
    await api.todos.snooze(todoId.value, minutes)
  } catch (error) {
    logger.error('reminder', '顺延提醒失败', error)
  }
  await api.windows.reminderClose(todoId.value).catch(() => undefined)
}

onMounted(load)
</script>

<template>
  <div id="reminderCard" class="glass">
    <!-- 仅标题行可拖拽：drag 区域会被子元素继承，放在根元素上会让整窗按钮全部点不动 -->
    <div class="reminder-header" data-tauri-drag-region>
      <button type="button" class="icon-btn reminder-close no-drag" title="关闭（稍后不再提醒）" @click="dismiss">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </button>
      <span class="reminder-title">⏰ 待办提醒</span>
    </div>

    <div class="reminder-content">{{ content }}</div>

    <div class="reminder-actions no-drag">
      <select class="snooze-select" @change="snooze">
        <option value="">选择下次提醒时间…</option>
        <option v-for="option in SNOOZE_OPTIONS" :key="option.value" :value="option.value">
          {{ option.label }}
        </option>
      </select>
    </div>
  </div>
</template>
