<script setup lang="ts">
import { ref } from 'vue'
import ModalShell from '@/components/base/ModalShell.vue'
import NoteEditor from '@/editor/NoteEditor.vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import type { Note, NoteInput } from '@/typings/domain'

/**
 * 笔记正文编辑弹窗。
 *
 * 需求 2.2：归档页笔记卡片底部右侧的「✏️ 编辑」用于修改**笔记正文**；
 * 标签的增删改走另一条入口——点击卡片左侧的标签区打开标签管理弹窗。
 * 两者互不混用，因此本弹窗不提供标签编辑。
 *
 * **只处理文本笔记**：思维导图的新建与编辑都在独立窗口里进行（见 NotesView 的
 * openMindmap）。导图需要大画布，而 MindMapEditor 带 flex: 1，
 * 放在高度不定的弹窗里会被拉伸成一块空白板。
 */
const props = defineProps<{ note: Note }>()

const emit = defineEmits<{
  (e: 'save', input: NoteInput): void
  (e: 'close'): void
}>()

const { toast } = useToast()

/** 新建态：没有既有笔记。 */
// 编辑副本：保存时才回传，取消则丢弃。
const content = ref(props.note.content)

function save(): void {
  if (!content.value.trim()) {
    toast('笔记内容不能为空')
    return
  }

  logger.info('note-edit', `保存笔记正文 id=${props.note.id}`)
  emit('save', {
    id: props.note.id,
    content: content.value,
    // 标签保持原值——本弹窗不负责标签编辑。
    tags: [...props.note.tags],
    // 模式与导图数据原样带回，避免编辑正文时把它们清空。
    editorMode: props.note.editor_mode,
    mindmapData: props.note.mindmap_data,
    draft: false,
  })
}
</script>

<template>
  <ModalShell overlay-id="clipEditorOverlay" modal-id="clipEditorModal" title="✏️ 编辑笔记" @close="emit('close')">
    <NoteEditor
      v-model="content"
      editor-mode="text"
      :show-mode-bar="false"
      placeholder="编辑笔记内容…支持 Markdown 即时渲染"
      @submit="save"
    />

    <template #footer>
      <span class="clip-editor-hint">标签请点击卡片上的标签区管理（⌃/⌘+Enter 快捷保存）</span>
      <div class="clip-editor-actions">
        <button type="button" class="btn ghost" @click="emit('close')">取消</button>
        <button type="button" class="btn primary" @click="save">保存</button>
      </div>
    </template>
  </ModalShell>
</template>

<style scoped>
/* 弹窗内的编辑器需要一个可滚动的固定高度，避免长笔记把弹窗撑破。 */
:deep(.note-editor-shell) {
  min-height: 220px;
  max-height: 46vh;
  overflow-y: auto;
}
</style>
