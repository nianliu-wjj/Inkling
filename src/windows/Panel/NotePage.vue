<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import TagList from '@/components/tag/TagList.vue'
import TagManagerModal from '@/components/tag/TagManagerModal.vue'
import NoteEditor from '@/editor/NoteEditor.vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Note } from '@/typings/domain'

/**
 * 面板 · 笔记模式（原型 #page-note）。
 *
 * - Markdown 即时渲染（由 NoteEditor 提供）；新建态下输入停止 500ms 自动暂存为草稿；
 * - 右下角标签区在归档按钮左侧，点击弹标签管理弹窗；「归档念头 ↵」正式落盘；
 * - **回显编辑态**（spec D16，原型 openNoteInPanel / hidePanel）：主窗口 ✏️ → `loadNote(id)`
 *   先落库当前草稿再载入笔记，按钮变「保存修改 ✓」，编辑态**不自动暂存**；
 *   收面板（`onPanelHide`）即丢弃未保存修改并恢复草稿；
 * - 底栏「退出 Zen」按钮显隐完全交给生成层 `#zenExit` / `#panel.zen-mode #zenExit`。
 *
 * 面板**只写文本笔记**：思维导图统一在归档页创建、在独立窗口里编辑（见 NotesView），
 * 因此不从草稿恢复 mindmap 模式，也不渲染模式切换条。
 */
const emit = defineEmits<{
  (e: 'modal', open: boolean): void
  /** 用户点击底栏「退出 Zen」。 */
  (e: 'zen-exit'): void
  /** 归档 / 保存修改成功：PanelApp 据此在 250ms 后收面板（原型 archiveNote 的 setTimeout(hidePanel, 250)）。 */
  (e: 'archived'): void
}>()

const { toast } = useToast()

const editor = ref<InstanceType<typeof NoteEditor> | null>(null)
const content = ref('')
const tags = ref<string[]>([])
/** 草稿 id：首次暂存后由后端返回，后续复用以免产生多条草稿。 */
const draftId = ref<string | undefined>(undefined)
/** 正在回显编辑的笔记 id；null = 新建态。 */
const editingNoteId = ref<string | null>(null)
/** 回显时的笔记快照：保存修改时把 editor_mode / mindmap_data 原样带回，避免被清空。 */
const editingSnapshot = ref<Note | null>(null)
const showTagManager = ref(false)
/** 暂存状态：idle「已暂存」（初始 / 恢复草稿后）· typing「输入中…」· saved「已暂存 SQLite」· failed「暂存失败」。 */
const saveState = ref<'idle' | 'typing' | 'saved' | 'failed'>('idle')

/** 是否处于回显编辑态（供 PanelApp 决定 Zen 入口显隐）。 */
const isEditing = computed(() => editingNoteId.value !== null)
/** 底栏主按钮文案（原型 #btnArchive）。 */
const archiveLabel = computed(() => (isEditing.value ? '保存修改 ✓' : '归档念头 ↵'))
/** 标签管理弹窗副标题（原型 openTagManager 的 draft / note 两种目标）。 */
const tagSubtitle = computed(() => (isEditing.value ? '当前笔记的标签' : '当前正在编写的念头（未归档）的标签'))

/** 500ms 防抖的暂存定时器。 */
let debounceTimer: ReturnType<typeof setTimeout> | null = null
/** 程序性写入 content / tags 时置位，让紧随其后的 watch 回调跳过自动暂存。 */
let suppressAutosave = false

/** 底栏暂存文案（原型 #saveState 的三段：已暂存 → 输入中… → 已暂存 SQLite；失败态为项目扩展）。 */
const saveLabel = computed(() => {
  if (saveState.value === 'typing') return '输入中…'
  if (saveState.value === 'saved') return '已暂存 SQLite'
  if (saveState.value === 'failed') return '暂存失败'
  return '已暂存'
})

function cancelPendingAutosave(): void {
  if (debounceTimer !== null) {
    clearTimeout(debounceTimer)
    debounceTimer = null
  }
}

/**
 * 程序性写入编辑区（恢复草稿 / 回显笔记 / 归档后清空），不触发自动暂存。
 * watch 在下一次 pre-flush 执行，等 nextTick 再解除抑制；无变化时 watch 不触发，同样在此解除。
 */
async function setLocal(nextContent: string, nextTags: string[]): Promise<void> {
  cancelPendingAutosave()
  suppressAutosave = true
  content.value = nextContent
  tags.value = nextTags
  await nextTick()
  suppressAutosave = false
}

/** 拉取既有草稿到编辑区；没有可用草稿时清空（退出编辑态时也靠它把笔记内容换掉）。 */
async function loadDraft(): Promise<void> {
  try {
    const draft = await api.notes.draft()
    // 思维导图草稿不在面板里恢复：面板没有导图编辑能力，恢复它只会得到一块无法输入的空白。
    if (!draft || draft.editor_mode === 'mindmap') {
      if (draft) logger.info('panel-note', `跳过思维导图草稿 id=${draft.id}，请在归档页编辑`)
      draftId.value = undefined
      await setLocal('', [])
      saveState.value = 'idle'
      return
    }
    draftId.value = draft.id
    await setLocal(draft.content, [...draft.tags])
    // 恢复草稿后显示初始文案「已暂存」，与原型一致。
    saveState.value = 'idle'
    logger.info('panel-note', `恢复草稿 id=${draft.id}`)
  } catch (error) {
    logger.error('panel-note', '加载草稿失败', error)
    // 拉取失败也要把编辑区清掉：调用方多半刚退出编辑态，不能让旧笔记的正文留在「归档念头」按钮下。
    draftId.value = undefined
    await setLocal('', [])
    saveState.value = 'idle'
  }
}
void loadDraft()

/** 暂存草稿（不广播 notes-changed，后端对草稿不发事件）。编辑态不暂存。 */
async function persistDraft(): Promise<void> {
  if (editingNoteId.value) return
  if (!content.value.trim() && !tags.value.length) {
    // 正文与标签都已清空：无需落库，但要把 watch 打上的「输入中…」复位，否则底栏会一直脉动。
    saveState.value = 'idle'
    return
  }
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
    saveState.value = 'failed'
    logger.error('panel-note', '暂存失败', error)
  }
}

// 新建态：输入停止 500ms 后自动暂存（需求 2.2「混合存储策略」）；编辑态与程序性写入跳过。
watch([content, tags], () => {
  if (suppressAutosave) return
  if (editingNoteId.value) return
  // 原型：一有输入就切「输入中…」+ .saving，落库后再切「已暂存 SQLite」。
  saveState.value = 'typing'
  cancelPendingAutosave()
  debounceTimer = setTimeout(() => {
    debounceTimer = null
    void persistDraft()
  }, 500)
})

/**
 * 回显一条笔记进入编辑态（原型 openNoteInPanel）。
 * 先把当前草稿落库——回显不能覆盖用户还没归档的念头；已在编辑另一条时直接替换。
 */
async function loadNote(id: string): Promise<void> {
  logger.info('panel-note', `回显笔记 id=${id}`)
  if (!editingNoteId.value) {
    cancelPendingAutosave()
    await persistDraft()
  }
  let note: Note | null
  try {
    note = await api.notes.get(id)
  } catch (error) {
    logger.error('panel-note', '读取笔记失败', error)
    toast('读取笔记失败')
    return
  }
  if (!note) {
    toast('笔记不存在')
    return
  }
  editingNoteId.value = id
  editingSnapshot.value = note
  await setLocal(note.content, [...note.tags])
  // 编辑态不自动暂存，底栏回到初始文案「已暂存」（描述的是草稿已落库，不是本次修改）。
  saveState.value = 'idle'
  toast('内容已回显，修改后点击「保存修改」')
  focus()
}

function exitEditing(): void {
  editingNoteId.value = null
  editingSnapshot.value = null
}

/** 归档：新建态把草稿提升为正式笔记；编辑态更新原笔记并恢复草稿（原型 archiveNote）。 */
async function archive(): Promise<void> {
  if (!content.value.trim()) {
    toast('先写点什么吧')
    return
  }
  const snapshot = editingSnapshot.value
  if (editingNoteId.value && snapshot) {
    logger.info('panel-note', `保存修改 id=${editingNoteId.value}`)
    try {
      await api.notes.save({
        id: editingNoteId.value,
        content: content.value,
        tags: [...tags.value],
        editorMode: snapshot.editor_mode,
        mindmapData: snapshot.mindmap_data,
        draft: false,
      })
    } catch (error) {
      logger.error('panel-note', '保存修改失败', error)
      toast('保存失败')
      return
    }
    // 先把草稿换进编辑区再退出编辑态：setLocal 无论编辑态与否都抑制自动暂存，
    // 而反过来先退出会让 watch 在草稿写入前的窗口期把笔记正文当成新草稿。
    await loadDraft()
    exitEditing()
    toast('修改已保存 ✔')
    emit('archived')
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
  } catch (error) {
    logger.error('panel-note', '归档失败', error)
    toast('归档失败')
    return
  }
  // 归档后清空面板，进入下一次捕获。
  draftId.value = undefined
  await setLocal('', [])
  saveState.value = 'idle'
  toast('念头已归档 ✔')
  emit('archived')
}

/** 面板即将隐藏：编辑态视为放弃修改，恢复草稿（原型 hidePanel）。 */
async function onPanelHide(): Promise<void> {
  if (!editingNoteId.value) return
  logger.info('panel-note', `收面板，丢弃未保存修改 id=${editingNoteId.value}`)
  // 同 archive：先恢复草稿再退出编辑态，见那里的说明。
  await loadDraft()
  exitEditing()
}

/** 把光标放进编辑器（呼出 / 回显 / 进入 Zen 后）。 */
function focus(): void {
  editor.value?.focus()
}

function openTagManager(): void {
  showTagManager.value = true
  emit('modal', true)
}

function closeTagManager(): void {
  showTagManager.value = false
  emit('modal', false)
}

/** 关闭本页浮层：标签管理弹窗开着就关掉并返回 true（切页副作用用）。 */
function dismissOverlays(): boolean {
  if (!showTagManager.value) return false
  closeTagManager()
  return true
}

function saveTags(next: string[]): void {
  tags.value = next
  closeTagManager()
}

defineExpose({ archive, focus, dismissOverlays, loadNote, onPanelHide, isEditing })
</script>

<template>
  <section class="panel-page">
    <NoteEditor ref="editor" v-model="content" editor-mode="text" :show-mode-bar="false" @submit="archive" />

    <div class="editor-footer">
      <span class="save-state" :class="{ saving: saveState === 'typing' }">{{ saveLabel }}</span>
      <div class="editor-actions">
        <!-- 标签区位于归档按钮左侧（需求 2.2 指定的两处展示位置之一） -->
        <div class="tag-preview" title="点击管理标签" @click="openTagManager">
          <TagList :tags="tags" :max="3" more-action="open" />
        </div>
        <!-- 原型 #zenExit：显隐由生成层按 #panel.zen-mode 控制，不传 prop -->
        <button id="zenExit" type="button" class="btn ghost" title="退出 Zen 专注模式（Esc）" @click="emit('zen-exit')">
          <span class="ix">🧘</span> 退出 Zen
        </button>
        <button type="button" class="btn primary" @click="archive">{{ archiveLabel }}</button>
      </div>
    </div>

    <TagManagerModal
      v-if="showTagManager"
      :tags="tags"
      :max-length="5"
      :subtitle="tagSubtitle"
      @save="saveTags"
      @close="closeTagManager"
    />
  </section>
</template>
