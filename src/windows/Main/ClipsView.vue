<script setup lang="ts">
import { computed, ref } from 'vue'
import ClipCard from '@/components/card/ClipCard.vue'
import ClipEditorModal from '@/components/clip/ClipEditorModal.vue'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import { useClips } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { ClipboardEntry } from '@/typings/domain'

/**
 * 归档 · 粘贴板页。
 *
 * 需求 2.2：类型以彩色徽章展示；置顶优先排序并高亮描边；
 * 链接类型提供「在默认浏览器中打开」；同样提供搜索框。
 */
const { clips } = useClips()
const { toast } = useToast()
const confirm = useConfirmDelete('clips-view')

const keyword = ref('')
const editing = ref<ClipboardEntry | null>(null)

const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return clips.value
    .filter((entry) => !key || entry.content.toLowerCase().includes(key))
    .sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1
      return (b.modified_at || b.copied_at).localeCompare(a.modified_at || a.copied_at)
    })
})

async function paste(entry: ClipboardEntry): Promise<void> {
  try {
    await api.clipboard.write(entry.id)
    toast('已粘贴到剪贴板')
  } catch (error) {
    logger.error('clips-view', '粘贴失败', error)
    toast('粘贴失败')
  }
}

async function togglePin(entry: ClipboardEntry): Promise<void> {
  try {
    await api.clipboard.pin(entry.id, !entry.pinned)
  } catch (error) {
    logger.error('clips-view', '置顶失败', error)
    toast('操作失败')
  }
}

async function openLink(entry: ClipboardEntry): Promise<void> {
  try {
    await api.system.openUrl(entry.content)
  } catch (error) {
    logger.error('clips-view', '打开链接失败', error)
    toast('打开链接失败')
  }
}

async function saveEdit(content: string): Promise<void> {
  const target = editing.value
  if (!target) return
  try {
    await api.clipboard.update(target.id, content)
    toast('已保存')
  } catch (error) {
    logger.error('clips-view', '保存失败', error)
    toast('保存失败')
  } finally {
    editing.value = null
  }
}

async function remove(entry: ClipboardEntry): Promise<void> {
  if (!confirm.confirm()) return
  try {
    await api.clipboard.remove(entry.id)
    toast('已删除')
  } catch (error) {
    logger.error('clips-view', '删除失败', error)
    toast('删除失败')
  }
}
</script>

<template>
  <div class="archive-page">
    <input v-model="keyword" class="search-input" placeholder="🔍 搜索粘贴板历史…" />

    <ul>
      <ClipCard
        v-for="entry in visible"
        :key="entry.id"
        :entry="entry"
        archive
        :confirming="confirm.isPending(entry.id)"
        @paste="paste(entry)"
        @pin="togglePin(entry)"
        @edit="editing = entry"
        @open-link="openLink(entry)"
        @ask-delete="confirm.ask(entry.id)"
        @confirm-delete="remove(entry)"
        @cancel-delete="confirm.cancel()"
      />
      <li v-if="!visible.length" class="tag-mgr-empty">
        {{ keyword ? '没有匹配的剪贴板记录' : '还没有剪贴板记录' }}
      </li>
    </ul>

    <ClipEditorModal v-if="editing" :content="editing.content" @save="saveEdit" @close="editing = null" />
  </div>
</template>
