<script setup lang="ts">
import { computed, ref } from 'vue'
import NoteCard from '@/components/card/NoteCard.vue'
import NoteEditModal from '@/components/note/NoteEditModal.vue'
import TagManagerModal from '@/components/tag/TagManagerModal.vue'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import { useNotes } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Note, NoteInput } from '@/typings/domain'

/**
 * 归档 · 笔记页。
 *
 * 需求 v1.2 变更 #13：笔记搜索覆盖**正文与标签**。
 * 置顶笔记优先排序。
 *
 * 两条编辑入口互不混用（需求 2.2）：
 * - 卡片底部右侧「✏️ 编辑」→ 编辑笔记正文 / 思维导图；
 * - 卡片左侧标签区 → 标签管理弹窗。
 */
const { notes } = useNotes()
const { toast } = useToast()
const confirm = useConfirmDelete('notes-view')

const keyword = ref('')
/** 类型筛选：思维导图与文本笔记混排，需要能只看一类。 */
const kindFilter = ref<'all' | 'text' | 'mindmap'>('all')
/** 正在管理标签的笔记。 */
const tagTarget = ref<Note | null>(null)
/** 正在编辑正文/思维导图的笔记。 */
const editTarget = ref<Note | null>(null)
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

/** 空列表提示：区分「筛没了」与「本来就没有」，否则用户以为数据丢了。 */
const emptyHint = computed(() => {
  if (keyword.value.trim()) return '没有匹配的笔记'
  if (kindFilter.value === 'mindmap') return '还没有思维导图，点右上角「新建导图」开始'
  if (kindFilter.value === 'text') return '还没有文本笔记'
  return '还没有归档的念头'
})

const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return notes.value
    .filter((note) => {
      if (note.is_draft) return false
      // 类型筛选：editor_mode 缺省视为文本笔记（v2 迁移前的历史数据）。
      if (kindFilter.value !== 'all') {
        const kind = note.editor_mode === 'mindmap' ? 'mindmap' : 'text'
        if (kind !== kindFilter.value) return false
      }
      if (!key) return true
      // 正文 + 标签双重匹配
      return note.content.toLowerCase().includes(key) || note.tags.some((tag) => tag.toLowerCase().includes(key))
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

/**
 * 保存笔记正文 / 思维导图（标签由 NoteEditModal 原样带回，不在此处改动）。
 *
 * 同时承担新建思维导图的落库：input 不带 id 时后端插入新记录。
 */
async function saveNote(input: NoteInput): Promise<void> {
  try {
    await api.notes.save(input)
    toast(input.id ? '已保存' : '思维导图已创建')
  } catch (error) {
    logger.error('notes-view', '保存笔记失败', error)
    toast('保存失败')
  } finally {
    editTarget.value = null
  }
}
</script>

<template>
  <div class="archive-page">
    <div class="notes-toolbar">
      <input v-model="keyword" class="search-input" placeholder="🔍 搜索笔记…（正文与标签）" />
      <select v-model="kindFilter" class="prio-select" title="按类型筛选">
        <option value="all">全部类型</option>
        <option value="text">📝 笔记</option>
        <option value="mindmap">🧠 思维导图</option>
      </select>
      <button type="button" class="btn tiny" title="新建思维导图（思维导图只能在此创建）" @click="openMindmap()">
        🧠 新建导图
      </button>
    </div>

    <div>
      <NoteCard
        v-for="note in visible"
        :key="note.id"
        :note="note"
        :confirming="confirm.isPending(note.id)"
        @pin="togglePin(note)"
        @edit="note.editor_mode === 'mindmap' ? openMindmap(note.id) : (editTarget = note)"
        @open-tags="tagTarget = note"
        @ask-delete="confirm.ask(note.id)"
        @confirm-delete="remove(note)"
        @cancel-delete="confirm.cancel()"
      />
      <div v-if="!visible.length" class="tag-mgr-empty">
        {{ emptyHint }}
      </div>
    </div>

    <!-- ✏️ 编辑：笔记正文 / 思维导图 -->
    <NoteEditModal v-if="editTarget" :note="editTarget" @save="saveNote" @close="editTarget = null" />

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
