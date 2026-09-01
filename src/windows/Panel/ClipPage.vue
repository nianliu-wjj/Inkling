<script setup lang="ts">
import { computed, ref } from 'vue'
import ClipEditorModal from '@/components/clip/ClipEditorModal.vue'
import ClipCard from '@/components/card/ClipCard.vue'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import { useClips } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { ClipboardEntry } from '@/typings/domain'

/**
 * 面板 · 粘贴板模式。
 *
 * 需求 2.2：实时模糊搜索；双击条目 = 粘贴并置顶；置顶条目金色高亮并优先排序。
 */
const emit = defineEmits<{
  /** 弹窗开合需要通知面板，弹窗打开期间禁止失焦收起。 */
  (e: 'modal', open: boolean): void
}>()

const { clips } = useClips()
const { toast } = useToast()
const confirm = useConfirmDelete('panel-clip')

const keyword = ref('')
/** 正在编辑的条目；非空即弹出编辑框。 */
const editing = ref<ClipboardEntry | null>(null)

/** 置顶优先，其次按时间倒序。 */
const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return clips.value
    .filter((entry) => !key || entry.content.toLowerCase().includes(key))
    .sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1
      return (b.modified_at || b.copied_at).localeCompare(a.modified_at || a.copied_at)
    })
})

/**
 * 粘贴到光标处并置顶（需求 2.2「双击操作」）。
 *
 * 后端会写入剪贴板 → 收起面板把焦点交还给用户原本所在的应用 →
 * 模拟 Ctrl/Cmd+V，因此内容落在光标处而不是只进剪贴板。
 * 置顶先于粘贴执行，避免面板收起后组件已卸载导致后续调用丢失。
 */
async function paste(entry: ClipboardEntry): Promise<void> {
  logger.info('panel-clip', `粘贴到光标处并置顶 id=${entry.id}`)
  try {
    if (!entry.pinned) await api.clipboard.pin(entry.id, true)
    await api.clipboard.paste(entry.id)
  } catch (error) {
    logger.error('panel-clip', '粘贴失败', error)
    toast('粘贴失败')
  }
}

async function togglePin(entry: ClipboardEntry): Promise<void> {
  try {
    await api.clipboard.pin(entry.id, !entry.pinned)
  } catch (error) {
    logger.error('panel-clip', '置顶切换失败', error)
    toast('操作失败')
  }
}

async function openLink(entry: ClipboardEntry): Promise<void> {
  try {
    await api.system.openUrl(entry.content)
  } catch (error) {
    logger.error('panel-clip', '打开链接失败', error)
    toast('打开链接失败')
  }
}

function startEdit(entry: ClipboardEntry): void {
  editing.value = entry
  emit('modal', true)
}

function closeEdit(): void {
  editing.value = null
  emit('modal', false)
}

async function saveEdit(content: string): Promise<void> {
  const target = editing.value
  if (!target) return
  try {
    await api.clipboard.update(target.id, content)
    toast('已保存')
  } catch (error) {
    logger.error('panel-clip', '保存失败', error)
    toast('保存失败')
  } finally {
    closeEdit()
  }
}

async function remove(entry: ClipboardEntry): Promise<void> {
  if (!confirm.confirm()) return
  try {
    await api.clipboard.remove(entry.id)
    toast('已删除')
  } catch (error) {
    logger.error('panel-clip', '删除失败', error)
    toast('删除失败')
  }
}
</script>

<template>
  <section class="panel-page">
    <input v-model="keyword" class="search-input" placeholder="搜索剪贴板历史…（双击条目 = 粘贴并置顶）" />

    <ul class="clip-list">
      <ClipCard
        v-for="entry in visible"
        :key="entry.id"
        :entry="entry"
        :confirming="confirm.isPending(entry.id)"
        @paste="paste(entry)"
        @pin="togglePin(entry)"
        @edit="startEdit(entry)"
        @open-link="openLink(entry)"
        @ask-delete="confirm.ask(entry.id)"
        @confirm-delete="remove(entry)"
        @cancel-delete="confirm.cancel()"
      />
      <li v-if="!visible.length" class="tag-mgr-empty">
        {{ keyword ? '没有匹配的剪贴板记录' : '还没有剪贴板记录' }}
      </li>
    </ul>

    <ClipEditorModal v-if="editing" :content="editing.content" @save="saveEdit" @close="closeEdit" />
  </section>
</template>
