<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onMounted, ref, watch } from 'vue'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { useSettings } from '@/composables/useData'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import { renderMarkdown } from '@/utils/format'

/**
 * 桌面置顶浮窗（需求 2.5）。
 *
 * 每个置顶项是一个独立窗口，label 形如 `pinned-{kind}-{id}`，
 * 前端据此解析自己该显示哪条内容。
 * 支持透明度调节；双击展开进入编辑态（由 Rust 侧调整窗口尺寸）。
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
/** 从窗口 label 解析出 kind 与 id：pinned-note-xxxx。 */
const parsed = computed(() => {
  const match = /^pinned-(note|todo|clip)-(.+)$/.exec(label)
  return match ? { kind: match[1], id: match[2] } : null
})

const content = ref('')
const opacity = ref(100)
const expanded = ref(false)

const html = computed(() => renderMarkdown(content.value))

/** 按 kind 拉取对应内容。 */
async function load(): Promise<void> {
  const target = parsed.value
  if (!target) {
    logger.error('pinned', `无法解析窗口 label: ${label}`)
    return
  }

  try {
    if (target.kind === 'note') {
      const notes = await api.notes.list()
      content.value = notes.find((n) => n.id === target.id)?.content ?? '（该笔记已被删除）'
    } else if (target.kind === 'todo') {
      const todos = await api.todos.list()
      content.value = todos.find((t) => t.id === target.id)?.content ?? '（该待办已被删除）'
    } else {
      const clips = await api.clipboard.list()
      content.value = clips.find((c) => c.id === target.id)?.content ?? '（该条目已被删除）'
    }
    logger.info('pinned', `已加载置顶内容 ${label}`)
  } catch (error) {
    logger.error('pinned', '加载置顶内容失败', error)
  }
}

/** 透明度实时作用于整个窗口。 */
watch(opacity, (value) => {
  document.documentElement.style.opacity = String(value / 100)
})

async function close(): Promise<void> {
  try {
    await api.windows.pinClose(label)
  } catch (error) {
    logger.error('pinned', '关闭置顶窗失败', error)
  }
}

/** 双击切换展开编辑态，窗口尺寸由 Rust 侧调整。 */
async function toggleExpand(): Promise<void> {
  expanded.value = !expanded.value
  try {
    await api.windows.pinSetEditing(label, expanded.value)
  } catch (error) {
    logger.error('pinned', '切换展开态失败', error)
  }
}

onMounted(load)
</script>

<template>
  <div id="pinnedWindow" class="glass">
    <!-- 仅标题行可拖拽：drag 区域会被子元素继承，放在根元素上会让整窗按钮全部点不动 -->
    <div class="pinned-header" data-tauri-drag-region>
      <span>📌 置顶</span>
      <span class="pinned-close no-drag" title="关闭" @click="close">✕</span>
    </div>

    <div
      class="pinned-body no-drag"
      :style="expanded ? { maxHeight: 'none' } : undefined"
      title="双击展开 / 收起"
      @dblclick="toggleExpand"
      v-html="html"
    />

    <div class="pinned-footer no-drag">
      <span>透明度</span>
      <input v-model.number="opacity" type="range" min="30" max="100" />
    </div>
  </div>
</template>
