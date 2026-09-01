<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { DayDetailItem } from '@/typings/domain'
import { formatClock, formatDateKeyLabel } from '@/utils/datetime'
import { renderMarkdownInline } from '@/utils/format'

/**
 * 归档 · 日期详情页。
 *
 * 需求 v1.2：点击侧边栏当月热力图某日 → 展示该日全部记录，
 * 按时间先后排序（待办取完成时间），可按类别筛选与搜索，悬浮卡片可编辑/删除。
 */
const props = defineProps<{ dateKey: string }>()

const { toast } = useToast()
const confirm = useConfirmDelete('day-view')

const items = ref<DayDetailItem[]>([])
const filter = ref<'all' | 'note' | 'clip' | 'todo'>('all')
const keyword = ref('')

const dateLabel = computed(() => formatDateKeyLabel(props.dateKey))

const FILTERS = [
  { key: 'all', label: '全部', cls: '' },
  { key: 'note', label: '📝 笔记', cls: 'f-note' },
  { key: 'clip', label: '📋 粘贴板', cls: 'f-clip' },
  { key: 'todo', label: '✅ 待办', cls: 'f-todo' },
] as const

/** 取条目正文，用于搜索与展示。 */
function textOf(item: DayDetailItem): string {
  if (item.kind === 'note') return item.note?.content ?? ''
  if (item.kind === 'clip') return item.clip?.preview || (item.clip?.content ?? '')
  return item.todo?.content ?? ''
}

/** 唯一标识，用于确认删除态与列表 key。 */
function idOf(item: DayDetailItem): string {
  return `${item.kind}:${item.note?.id ?? item.clip?.id ?? item.todo?.id ?? ''}`
}

const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return items.value.filter((item) => {
    if (filter.value !== 'all' && item.kind !== filter.value) return false
    return !key || textOf(item).toLowerCase().includes(key)
  })
})

async function load(): Promise<void> {
  try {
    items.value = await api.stats.day(props.dateKey)
    logger.info('day-view', `加载 ${props.dateKey} 的记录：${items.value.length} 条`)
  } catch (error) {
    logger.error('day-view', '加载日期详情失败', error)
    items.value = []
  }
}

watch(() => props.dateKey, load, { immediate: true })

async function remove(item: DayDetailItem): Promise<void> {
  if (!confirm.confirm()) return
  try {
    if (item.kind === 'note' && item.note) await api.notes.remove(item.note.id)
    else if (item.kind === 'clip' && item.clip) await api.clipboard.remove(item.clip.id)
    else if (item.kind === 'todo' && item.todo) await api.todos.remove(item.todo.id)
    await load()
    toast('已删除')
  } catch (error) {
    logger.error('day-view', '删除失败', error)
    toast(String(error))
  }
}
</script>

<template>
  <div class="archive-page">
    <div class="day-head">
      <span class="day-date">{{ dateLabel }}</span>
      <div class="day-filters">
        <span
          v-for="option in FILTERS"
          :key="option.key"
          class="day-filter"
          :class="[option.cls, { active: filter === option.key }]"
          @click="filter = option.key"
          >{{ option.label }}</span
        >
      </div>
      <input v-model="keyword" class="search-input day-search" placeholder="🔍 在该日记录中搜索…" />
    </div>

    <div class="day-hint">按时间先后排序（待办取完成时间）· 悬浮卡片可编辑 / 删除</div>

    <div>
      <div
        v-for="item in visible"
        :key="idOf(item)"
        class="day-item"
        :class="[item.kind, { done: item.todo?.status === 'done' }]"
      >
        <ConfirmPopover
          v-if="confirm.isPending(idOf(item))"
          text="⚠️ 确认删除该记录？"
          @confirm="remove(item)"
          @cancel="confirm.cancel()"
        />

        <span class="day-time">{{ formatClock(item.time) }}</span>
        <span class="day-badge" :class="item.kind">
          {{ item.kind === 'note' ? '笔记' : item.kind === 'clip' ? '复制' : '待办' }}
        </span>

        <div class="day-body">
          <div class="day-title-row">
            <span class="day-text d-clamp3" v-html="renderMarkdownInline(textOf(item))" />
          </div>
          <div v-if="item.todo?.tags?.length" class="day-tags">
            <span v-for="tag in item.todo.tags" :key="tag" class="tag-chip todo-tag">
              <span class="tag-name">{{ tag }}</span>
            </span>
          </div>
          <div v-if="item.todo?.remark" class="day-remark">{{ item.todo.remark }}</div>
        </div>

        <div class="day-ops">
          <IconBtn data-dayact="del" title="删除" @click="confirm.ask(idOf(item))">✕</IconBtn>
        </div>
      </div>

      <div v-if="!visible.length" class="tag-mgr-empty">该日期没有记录</div>
    </div>
  </div>
</template>
