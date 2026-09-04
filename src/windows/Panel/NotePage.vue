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
/**
 * 面板**只写文本笔记**，模式恒为 text。
 *
 * 思维导图统一在归档页创建、在独立窗口里编辑（见 NotesView）：它需要大画布与
 * 反复调整，与面板「极速捕获」的节奏不同，塞进 480px 面板里容器高度也会失控。
 * 这里不再从草稿恢复 mindmap 模式——否则一旦存在思维导图草稿，
 * 面板就会渲染导图编辑器，而没有切换条可以切回文本，文本输入被彻底锁死。
 */
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
    // 思维导图草稿不在面板里恢复：面板没有导图编辑能力，恢复它只会得到一块
    // 无法输入的空白。此类草稿留给归档页的思维导图窗口处理。
    if (draft.editor_mode === 'mindmap') {
      logger.info('panel-note', `跳过思维导图草稿 id=${draft.id}，请在归档页编辑`)
      return
    }
    draftId.value = draft.id
    content.value = draft.content
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
  if (!content.value.trim() && !tags.value.length) return
  saveState.value = 'saving'
  try {
    const note = await api.notes.save({
      id: draftId.value,
      content: content.value,
      tags: [...tags.value],
      editorMode: 'text',
      mindmapData: null,
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
watch([content, tags], () => {
  saveState.value = 'idle'
  if (debounceTimer !== null) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    debounceTimer = null
    void persistDraft()
  }, 500)
})

/** 归档：把草稿提升为正式笔记。 */
async function archive(): Promise<void> {
  if (!content.value.trim()) {
    toast('还没有写下任何内容')
    return
  }
  logger.info('panel-note', '归档念头')
  try {
    await api.notes.save({
      id: draftId.value,
      content: content.value,
      tags: [...tags.value],
      editorMode: 'text',
      mindmapData: null,
      draft: false,
    })
    // 归档后清空面板，进入下一次捕获。
    content.value = ''
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
    <!-- 不给模式切换条：思维导图统一在归档页的笔记列表创建（见 NotesView）。
         面板是「极速捕获」的入口，思维导图需要大画布与反复调整，两者节奏不同。
         已有的思维导图草稿仍会按其自身模式渲染，不会因此丢数据。 -->
    <NoteEditor v-model="content" editor-mode="text" :show-mode-bar="false" @submit="archive" />

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
