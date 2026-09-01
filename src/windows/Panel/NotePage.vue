<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import TagList from '@/components/tag/TagList.vue'
import TagManagerModal from '@/components/tag/TagManagerModal.vue'
import NoteEditor from '@/editor/NoteEditor.vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 面板 · 笔记模式。
 *
 * 需求 2.2：
 * - Markdown 即时渲染��由 NoteEditor 提供）；
 * - 输入停止 500ms 自动暂存为草稿；
 * - 右下角标签区在归档按钮左侧，点击弹标签管理弹窗；
 * - 点击「归档念头」正式落盘。
 */
const emit = defineEmits<{ (e: 'modal', open: boolean): void }>()

const { toast } = useToast()

const content = ref('')
const editorMode = ref<'text' | 'mindmap'>('text')
const mindmapData = ref<string | null>(null)
const tags = ref<string[]>([])
/** 草稿 id：首次暂存后由后端返回，后续复用以免产生多条草稿。 */
const draftId = ref<string | undefined>(undefined)
const showTagManager = ref(false)
const saveState = ref<'idle' | 'saving' | 'saved'>('idle')

/** 500ms 防抖的暂存定时器。 */
let debounceTimer: ReturnType<typeof setTimeout> | null = null

const saveLabel = computed(() => {
  if (saveState.value === 'saving') return '暂存中…'
  if (saveState.value === 'saved') return '已暂存'
  return '未保存'
})

/** 拉取既有草稿，保证面板重开后内容不丢。 */
async function loadDraft(): Promise<void> {
  try {
    const draft = await api.notes.draft()
    if (!draft) return
    draftId.value = draft.id
    content.value = draft.content
    editorMode.value = draft.editor_mode
    mindmapData.value = draft.mindmap_data
    tags.value = [...draft.tags]
    saveState.value = 'saved'
    logger.info('panel-note', `恢复草稿 id=${draft.id}`)
  } catch (error) {
    logger.error('panel-note', '加载草稿失败', error)
  }
}
void loadDraft()

/** 暂存草稿（不广播 notes-changed，后端对草稿不发事件）。 */
async function persistDraft(): Promise<void> {
  if (!content.value.trim() && !tags.value.length && !mindmapData.value) return
  saveState.value = 'saving'
  try {
    const note = await api.notes.save({
      id: draftId.value,
      content: content.value,
      tags: [...tags.value],
      editorMode: editorMode.value,
      mindmapData: mindmapData.value,
      draft: true,
    })
    draftId.value = note.id
    saveState.value = 'saved'
    logger.debug('panel-note', `草稿已暂存 id=${note.id}`)
  } catch (error) {
    saveState.value = 'idle'
    logger.error('panel-note', '暂存失败', error)
  }
}

// 输入停止 500ms 后自动暂存（需求 2.2「混合存储策略」）。
watch([content, tags, editorMode, mindmapData], () => {
  saveState.value = 'idle'
  if (debounceTimer !== null) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    debounceTimer = null
    void persistDraft()
  }, 500)
})

/** 归档：把草稿提升为正式笔记。 */
async function archive(): Promise<void> {
  if (editorMode.value === 'text' && !content.value.trim()) {
    toast('还没有写下任何内容')
    return
  }
  if (editorMode.value === 'mindmap' && !mindmapData.value) {
    toast('请先创建思维导图内容')
    return
  }

  logger.info('panel-note', '归档念头')
  try {
    await api.notes.save({
      id: draftId.value,
      content: content.value,
      tags: [...tags.value],
      editorMode: editorMode.value,
      mindmapData: mindmapData.value,
      draft: false,
    })
    // 归档后清空面板，进入下一次捕获。
    content.value = ''
    editorMode.value = 'text'
    mindmapData.value = null
    tags.value = []
    draftId.value = undefined
    saveState.value = 'idle'
    toast('已归档')
  } catch (error) {
    logger.error('panel-note', '归档失败', error)
    toast('归档失败')
  }
}

function openTagManager(): void {
  showTagManager.value = true
  emit('modal', true)
}

function closeTagManager(): void {
  showTagManager.value = false
  emit('modal', false)
}

function saveTags(next: string[]): void {
  tags.value = next
  closeTagManager()
}

defineExpose({ archive })
</script>

<template>
  <section class="panel-page">
    <NoteEditor
      v-model="content"
      v-model:editor-mode="editorMode"
      v-model:mindmap-data="mindmapData"
      @submit="archive"
    />

    <div class="editor-footer">
      <span class="save-state" :class="{ saving: saveState === 'saving' }">{{ saveLabel }}</span>
      <div class="editor-actions">
        <!-- 标签区位于归档按钮左侧（需求 2.2 指定的两处展示位置之一） -->
        <div class="tag-preview" title="点击管理标签">
          <TagList :tags="tags" :max="3" @open="openTagManager" />
        </div>
        <button type="button" class="btn primary" @click="archive">归档念头 ↵</button>
      </div>
    </div>

    <TagManagerModal
      v-if="showTagManager"
      :tags="tags"
      :max-length="5"
      subtitle="当前笔记的标签"
      @save="saveTags"
      @close="closeTagManager"
    />
  </section>
</template>
