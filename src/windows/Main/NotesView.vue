<script setup lang="ts">
import { computed, ref } from 'vue'
import { vStaggerList } from '@/motion'
import NoteCard from '@/components/card/NoteCard.vue'
import TagManagerModal from '@/components/tag/TagManagerModal.vue'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import { useShakeConfirm } from '@/composables/useShakeConfirm'
import { useNotes } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Note } from '@/typings/domain'
import { mindmapAllText } from '@/utils/search'

/**
 * 归档 · 笔记页。
 *
 * 原型 renderArchive / #archive-notes：
 * - 工具栏 = 搜索框 + 「🧠 思维导图」新建入口（.note-arch-bar）；
 * - 搜索覆盖正文、标签与思维导图全部节点文本；置顶笔记优先排序；
 * - 卡片标签 ✕：首次点击进入抖动确认态，再次点击真正删除（写库），3 秒无操作自动退出；
 * - 两条编辑入口互不混用：「✏️ 编辑」→ 文本笔记回显到呼出面板（spec D16）/ 导图开独立窗口；标签区 → 标签管理弹窗。
 */
const { notes } = useNotes()
const { toast } = useToast()
const confirm = useConfirmDelete('notes-view')
/** 标签删除的抖动二次确认：键为「笔记 id:标签名」。 */
const shake = useShakeConfirm()

const keyword = ref('')
/** 正在管理标签的笔记。 */
const tagTarget = ref<Note | null>(null)
/**
 * 打开思维导图窗口。
 *
 * 思维导图走**独立窗口**而非弹窗：它需要大画布，MindMapEditor 带 flex: 1，
 * 放在高度不定的弹窗里会被无限拉伸成一块空白板。独立窗口还能与主窗口互不干扰
 * ——关掉主窗口它照常在，缩放也各自独立。
 */
function openMindmap(id?: string): void {
  logger.info('notes-view', `打开思维导图窗口 id=${id ?? '(新建)'}`)
  void api.windows.mindmapOpen(id).catch((error) => {
    logger.error('notes-view', '打开思维导图窗口失败', error)
    toast('打开思维导图失败')
  })
}

/** ✏️ 文本笔记 → 回显到呼出面板编辑（spec D16）：后端隐藏主窗口并呼出面板，面板取走意图后载入笔记。 */
function openInPanel(note: Note): void {
  logger.info('notes-view', `回显到面板 id=${note.id}`)
  void api.windows.panelOpenNote(note.id).catch((error) => {
    logger.error('notes-view', '回显到面板失败', error)
    toast('打开面板失败')
  })
}

/** 空列表提示（原型文案）：区分「筛没了」与「本来就没有」。 */
const emptyHint = computed(() => (keyword.value.trim() ? '未找到匹配的笔记' : '暂无笔记'))

const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return notes.value
    .filter((note) => {
      if (note.is_draft) return false
      if (!key) return true
      // 正文 + 标签 + 思维导图全部节点文本（原型 renderArchive 的匹配口径）
      return (
        note.content.toLowerCase().includes(key) ||
        note.tags.some((tag) => tag.toLowerCase().includes(key)) ||
        (note.editor_mode === 'mindmap' && mindmapAllText(note.mindmap_data).toLowerCase().includes(key))
      )
    })
    .sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1
      return (b.archived_at ?? b.created_at).localeCompare(a.archived_at ?? a.created_at)
    })
})

async function togglePin(note: Note): Promise<void> {
  try {
    await api.notes.pin(note.id, !note.pinned)
    // 置顶时同步开出桌面浮窗（需求 2.5）。
    if (!note.pinned) await api.windows.pinCreate('note', note.id)
  } catch (error) {
    logger.error('notes-view', '置顶切换失败', error)
    toast('操作失败')
  }
}

async function remove(note: Note): Promise<void> {
  if (!confirm.confirm()) return
  try {
    await api.notes.remove(note.id)
    toast('已删除')
  } catch (error) {
    logger.error('notes-view', '删除失败', error)
    toast('删除失败')
  }
}

/** 抖动确认键：同一张卡片上只有一个标签处于确认态。 */
function shakeKey(note: Note, tag: string): string {
  return `${note.id}:${tag}`
}

/** 卡片是否处于抖动确认态。 */
function isShaking(note: Note): boolean {
  return shake.shakingId.value?.startsWith(`${note.id}:`) ?? false
}

/** 该卡片上处于确认态的标签名；无则 null。 */
function shakingTagOf(note: Note): string | null {
  const id = shake.shakingId.value
  return id && id.startsWith(`${note.id}:`) ? id.slice(note.id.length + 1) : null
}

/**
 * 卡片标签 ✕（原型 data-tagdel）：首次点击进入抖动确认，再次点击真正删除。
 * 删除是写库操作，与原型一致——抖动确认 + 3 秒超时是唯一防误触。
 */
async function removeTag(note: Note, tag: string): Promise<void> {
  if (!shake.press(shakeKey(note, tag))) {
    toast('再次点击 ✕ 确认删除该标签')
    return
  }
  try {
    await api.notes.save({
      id: note.id,
      content: note.content,
      tags: note.tags.filter((item) => item !== tag),
      editorMode: note.editor_mode,
      mindmapData: note.mindmap_data,
      draft: false,
    })
    toast(`已删除标签 #${tag}`)
  } catch (error) {
    logger.error('notes-view', '删除标签失败', error)
    toast('删除标签失败')
  }
}

async function saveTags(tags: string[]): Promise<void> {
  const note = tagTarget.value
  if (!note) return
  try {
    await api.notes.save({
      id: note.id,
      content: note.content,
      tags,
      editorMode: note.editor_mode,
      mindmapData: note.mindmap_data,
      draft: false,
    })
    toast('标签已保存')
  } catch (error) {
    logger.error('notes-view', '保存标签失败', error)
    toast('保存失败')
  } finally {
    tagTarget.value = null
  }
}
</script>

<template>
  <div class="archive-page">
    <div class="note-arch-bar">
      <input v-model="keyword" class="search-input" placeholder="🔍 搜索笔记…（正文、标签与思维导图节点）" />
      <button type="button" class="btn tiny" title="新建思维导图（保存后作为笔记卡片入列表）" @click="openMindmap()">
        <span class="ix">🧠</span> 思维导图
      </button>
    </div>

    <div v-stagger-list>
      <NoteCard
        v-for="note in visible"
        :key="note.id"
        :note="note"
        :confirming="confirm.isPending(note.id)"
        :shaking="isShaking(note)"
        :shaking-tag="shakingTagOf(note)"
        @pin="togglePin(note)"
        @edit="note.editor_mode === 'mindmap' ? openMindmap(note.id) : openInPanel(note)"
        @open-tags="tagTarget = note"
        @remove-tag="removeTag(note, $event)"
        @ask-delete="confirm.ask(note.id)"
        @confirm-delete="remove(note)"
        @cancel-delete="confirm.cancel()"
      />
      <div v-if="!visible.length" class="todo-empty">{{ emptyHint }}</div>
    </div>

    <!-- 标签区：仅管理标签 -->
    <TagManagerModal
      v-if="tagTarget"
      :tags="tagTarget.tags"
      :max-length="5"
      subtitle="当前笔记的标签"
      @save="saveTags"
      @close="tagTarget = null"
    />
  </div>
</template>
