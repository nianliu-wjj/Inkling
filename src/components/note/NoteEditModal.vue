<script setup lang="ts">
import { computed, ref } from 'vue'
import ModalShell from '@/components/base/ModalShell.vue'
import NoteEditor from '@/editor/NoteEditor.vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import type { Note, NoteInput } from '@/typings/domain'

/**
 * 笔记内容编辑弹窗，兼作思维导图的新建入口。
 *
 * 需求 2.2：归档页笔记卡片底部右侧的「✏️ 编辑」用于修改**笔记正文 / 思维导图**；
 * 标签的增删改走另一条入口——点击卡片左侧的标签区打开标签管理弹窗。
 * 两者互不混用，因此本弹窗不提供标签编辑。
 *
 * `note` 为 null 时是**新建态**：由笔记列表的「新建思维导图」入口打开，
 * 模式固定为 mindmap，保存时不带 id，后端据此插入新笔记。
 * 思维导图只能在这里创建——面板不提供模式切换。
 */
const props = withDefaults(
  defineProps<{
    /** 编辑既有笔记时传入；新建时为 null。 */
    note?: Note | null
    /** 新建态的初始模式，目前只用于思维导图。 */
    initialMode?: 'text' | 'mindmap'
  }>(),
  { note: null, initialMode: 'mindmap' },
)

const emit = defineEmits<{
  (e: 'save', input: NoteInput): void
  (e: 'close'): void
}>()

const { toast } = useToast()

/** 新建态：没有既有笔记。 */
const isNew = computed(() => !props.note)

const title = computed(() =>
  isNew.value ? (props.initialMode === 'mindmap' ? '🧠 新建思维导图' : '📝 新建笔记') : '✏️ 编辑笔记',
)

// 编辑副本：保存时才回传，取消则丢弃。
const content = ref(props.note?.content ?? '')
const editorMode = ref<'text' | 'mindmap'>(props.note?.editor_mode ?? props.initialMode)
const mindmapData = ref<string | null>(props.note?.mindmap_data ?? null)

function save(): void {
  // 文本模式要求正文非空；思维导图模式以导图数据为准，正文可为空。
  if (editorMode.value === 'text' && !content.value.trim()) {
    toast('笔记内容不能为空')
    return
  }
  // 新建的思维导图至少要动过一个节点，否则会落下一条空记录。
  if (isNew.value && editorMode.value === 'mindmap' && !mindmapData.value) {
    toast('请先编辑思维导图内容')
    return
  }

  logger.info('note-edit', `保存笔记 id=${props.note?.id ?? '(新建)'} mode=${editorMode.value}`)
  emit('save', {
    // 新建时不带 id，后端据此插入新笔记。
    id: props.note?.id,
    content: content.value,
    // 标签保持原值——本弹窗不负责标签编辑；新建时为空。
    tags: [...(props.note?.tags ?? [])],
    editorMode: editorMode.value,
    mindmapData: mindmapData.value,
    draft: false,
  })
}
</script>

<template>
  <ModalShell overlay-id="clipEditorOverlay" modal-id="clipEditorModal" :title="title" @close="emit('close')">
    <NoteEditor
      v-model="content"
      v-model:editor-mode="editorMode"
      v-model:mindmap-data="mindmapData"
      :show-mode-bar="false"
      :placeholder="editorMode === 'mindmap' ? '可在此补充说明（选填）' : '编辑笔记内容…支持 Markdown 即时渲染'"
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
