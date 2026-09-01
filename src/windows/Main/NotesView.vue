<script setup lang="ts">
import { computed, ref } from 'vue'
import NoteCard from '@/components/card/NoteCard.vue'
import TagManagerModal from '@/components/tag/TagManagerModal.vue'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import { useNotes } from '@/composables/useData'
import { useExport } from '@/composables/useExport'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Note } from '@/typings/domain'

/**
 * 归档 · 笔记页。
 *
 * 需求 v1.2 变更 #13：笔记搜索覆盖**正文与标签**。
 * 置顶笔记优先排序。
 */
const { notes } = useNotes()
const { toast } = useToast()
const confirm = useConfirmDelete('notes-view')
const { exporting, exportOne, exportMany } = useExport()

const keyword = ref('')
/** 正在管理标签的笔记。 */
const tagTarget = ref<Note | null>(null)

const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return notes.value
    .filter((note) => {
      if (note.is_draft) return false
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
</script>

<template>
  <div class="archive-page">
    <input v-model="keyword" class="search-input" placeholder="🔍 搜索笔记…（正文与标签）" />

    <div>
      <NoteCard
        v-for="note in visible"
        :key="note.id"
        :note="note"
        :confirming="confirm.isPending(note.id)"
        @pin="togglePin(note)"
        @edit="tagTarget = note"
        @open-tags="tagTarget = note"
        @ask-delete="confirm.ask(note.id)"
        @confirm-delete="remove(note)"
        @cancel-delete="confirm.cancel()"
      />
      <div v-if="!visible.length" class="tag-mgr-empty">
        {{ keyword ? '没有匹配的笔记' : '还没有归档的念头' }}
      </div>
    </div>

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
