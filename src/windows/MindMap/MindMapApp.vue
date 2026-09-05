<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onMounted, ref, watch } from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import TagList from '@/components/tag/TagList.vue'
import TagManagerModal from '@/components/tag/TagManagerModal.vue'
import { useNotes, useSettings } from '@/composables/useData'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { useToast } from '@/composables/useToast'
import MindMapEditor from '@/editor/MindMapEditor.vue'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 思维导图窗口。
 *
 * 独立顶层窗口，一个笔记一个（窗口 label 形如 `mindmap-<id>`，新建用 `mindmap-new`）。
 * 它不是主窗口的子窗口，因此关闭主窗口不影响它，缩放最大化也互不干扰。
 *
 * 画布铺满整个窗口——这也顺带解决了导图放在弹窗里时的老问题：
 * MindMapEditor 带 flex: 1，在高度不定的弹窗里会被无限拉伸成一块空白板。
 *
 * 打开参数（目标笔记 id，空串表示新建）在挂载时按窗口 label 向后端拉取；
 * 不走 URL 查询串，因为 `WebviewUrl::App` 收的是相对路径，`?` 会被转义掉。
 */

// 启动瞬间先用缓存的主题与玻璃质感上色，避免默认深色闪一下再跳变。
applyCachedTheme()
applyCachedGlass()

document.documentElement.dataset.window = 'mindmap'

const label = getCurrentWindow().label

const { notes } = useNotes()
const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
const { toast } = useToast()

/** 目标笔记 id；空串表示新建，保存后由后端返回的 id 填入。 */
const noteId = ref('')
/** 参数是否已拉取完成——未完成时不渲染画布，避免先建一个空导图再切数据。 */
const ready = ref(false)
const mindmapData = ref<string | null>(null)
const tags = ref<string[]>([])
const showTagManager = ref(false)
/** 是否有未保存的改动，用于标题栏提示与关闭前拦截。 */
const dirty = ref(false)

watch(
  () => settings.value.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
)
watch(
  () => settings.value.glass_level,
  (level) => applyGlass(level),
  { immediate: true },
)

const isNew = computed(() => !noteId.value)
const title = computed(() => (isNew.value ? '新建思维导图' : '编辑思维导图'))

/** 从最新列表里取目标笔记，保证拿到的是当前数据而非打开时的快照。 */
const note = computed(() => notes.value.find((item) => item.id === noteId.value) ?? null)

function onDataChange(value: string): void {
  mindmapData.value = value
  dirty.value = true
}

async function save(): Promise<void> {
  if (!mindmapData.value) {
    toast('请先编辑思维导图内容')
    return
  }
  try {
    const saved = await api.notes.save({
      // 新建时不带 id，后端插入新记录并回传 id，后续保存复用它。
      id: noteId.value || undefined,
      content: note.value?.content ?? '',
      tags: [...tags.value],
      editorMode: 'mindmap',
      mindmapData: mindmapData.value,
      draft: false,
    })
    noteId.value = saved.id
    dirty.value = false
    toast('已保存')
    logger.info('mindmap', `保存思维导图 id=${saved.id}`)
  } catch (error) {
    logger.error('mindmap', '保存思维导图失败', error)
    toast('保存失败')
  }
}

async function close(): Promise<void> {
  // 有未保存改动时先确认，避免一次误点丢掉整张导图。
  if (dirty.value && !window.confirm('思维导图尚未保存，确认关闭？')) return
  try {
    await api.windows.mindmapClose(label)
  } catch (error) {
    logger.error('mindmap', '关闭窗口失败', error)
  }
}

function saveTags(next: string[]): void {
  tags.value = next
  dirty.value = true
  showTagManager.value = false
}

/** ⌃/⌘+S 保存，与其他编辑面板一致。 */
function onKeydown(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    void save()
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)

  void api.windows
    .mindmapPayload(label)
    .then((id) => {
      noteId.value = id ?? ''
      ready.value = true
      logger.info('mindmap', `打开思维导图 label=${label} id=${noteId.value || '(新建)'}`)
    })
    .catch((error) => {
      logger.error('mindmap', '获取打开参数失败', error)
      // 拿不到参数时按新建处理，至少窗口是可用的。
      ready.value = true
    })
})

// 目标笔记加载完成后填入导图数据与标签（新建时列表里没有它，保持空白）。
watch(note, (value) => {
  if (!value || dirty.value) return
  mindmapData.value = value.mindmap_data
  tags.value = [...value.tags]
})
</script>

<template>
  <div class="mindmap-window">
    <header class="mindmap-bar">
      <span class="mindmap-title">🧠 {{ title }}<em v-if="dirty" class="mindmap-dirty">未保存</em></span>
      <div class="mindmap-actions">
        <div class="tag-preview" title="点击管理标签">
          <TagList :tags="tags" :max="3" @open="showTagManager = true" />
        </div>
        <button type="button" class="btn" @click="close">关闭</button>
        <button type="button" class="btn primary" @click="save">保存 ⌃S</button>
      </div>
    </header>

    <!-- 画布铺满窗口余下空间；参数未就绪时不渲染，避免先建空导图再换数据 -->
    <MindMapEditor v-if="ready" :model-value="mindmapData" placeholder="中心主题" @update:model-value="onDataChange" />

    <TagManagerModal
      v-if="showTagManager"
      :tags="tags"
      :max-length="5"
      subtitle="当前思维导图的标签"
      @save="saveTags"
      @close="showTagManager = false"
    />

    <ToastHost />
  </div>
</template>
