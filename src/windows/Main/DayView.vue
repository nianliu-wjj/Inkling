<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { vStaggerList } from '@/motion'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import Icon from '@/components/base/Icon.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import ClipEditorModal from '@/components/clip/ClipEditorModal.vue'
import ClipTypeBadge from '@/components/clip/ClipTypeBadge.vue'
import NoteEditModal from '@/components/note/NoteEditModal.vue'
import PriorityBadge from '@/components/todo/PriorityBadge.vue'
import TodoEditorModal from '@/components/todo/TodoEditorModal.vue'
import { useTodos } from '@/composables/useData'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import { useToast } from '@/composables/useToast'
import { remindOffsetLabel } from '@/constants/reminder'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { ClipboardEntry, DayDetailItem, Note, NoteInput, Priority, Todo, TodoInput } from '@/typings/domain'
import { formatClock, formatDateKey } from '@/utils/datetime'
import { renderMarkdownInline } from '@/utils/format'
import { mindmapRootText } from '@/utils/search'
import { isOverdue } from '@/utils/todo'

/**
 * 归档 · 日期详情页（原型 #archive-day / renderDayDetail）。
 *
 * 点击侧边栏当月热力图某日 → 展示该日全部记录，按时间先后排序（待办取完成时间）；
 * 类别筛选 chip（role="button" + aria-pressed）与关键字搜索；卡片按原型顺序渲染：
 * 时间 · 类型徽章 · 正文行（优先级 / 类型 / 导图徽章 + 正文 + 逾期）· 提醒与重复行 · 标签 · 备注 ·
 * 子任务归属行 · 悬浮操作（编辑 / 删除）。编辑入口按类别分流：文本笔记弹窗、导图开独立窗口、
 * 粘贴板弹窗、待办弹窗（已完成待办拦截）。
 */
const props = defineProps<{ dateKey: string }>()

const { toast } = useToast()
const { todos } = useTodos()
const confirm = useConfirmDelete('day-view')

const items = ref<DayDetailItem[]>([])
const filter = ref<'all' | 'note' | 'clip' | 'todo'>('all')
const keyword = ref('')

/** 标题按原型：YYYY-MM-DD，今天追加「 · 今天」。 */
const dateLabel = computed(() => formatDateKey(props.dateKey, { todaySuffix: true }))

const FILTERS = [
  { key: 'all', label: '全部', icon: '', cls: '' },
  { key: 'note', label: '笔记', icon: '📝', cls: 'f-note' },
  { key: 'clip', label: '粘贴板', icon: '📋', cls: 'f-clip' },
  { key: 'todo', label: '待办', icon: '✅', cls: 'f-todo' },
] as const

/** 类型徽章文案（原型 TYPE 常量）。 */
const TYPE_LABEL: Record<string, string> = { note: '📝 笔记', clip: '📋 粘贴板', todo: '✅ 待办' }

/** 空态文案（原型）：按当前筛选类别措辞。 */
const EMPTY_BY_FILTER: Record<typeof filter.value, string> = {
  all: '记录',
  note: '笔记',
  clip: '粘贴板条目',
  todo: '待办事项',
}

/** 取条目正文，用于搜索与展示；导图笔记显示根节点文字。 */
function textOf(item: DayDetailItem): string {
  if (item.kind === 'note') {
    return item.note?.editor_mode === 'mindmap' ? mindmapRootText(item.note.mindmap_data) : (item.note?.content ?? '')
  }
  if (item.kind === 'clip') return item.clip?.preview || (item.clip?.content ?? '')
  return item.todo?.content ?? ''
}

/** 搜索匹配文本（原型 hay）：正文 + 标签 + 备注。 */
function haystackOf(item: DayDetailItem): string {
  const tags = item.note?.tags ?? item.todo?.tags ?? []
  return `${textOf(item)} ${tags.join(' ')} ${item.todo?.remark ?? ''}`.toLowerCase()
}

/** 唯一标识，用于确认删除态与列表 key。 */
function idOf(item: DayDetailItem): string {
  return `${item.kind}:${item.note?.id ?? item.clip?.id ?? item.todo?.id ?? ''}`
}

/** 待办条目是否逾期：未完成且完成时刻已过。 */
function overdueOf(item: DayDetailItem): boolean {
  return item.kind === 'todo' && item.todo ? isOverdue(item.todo) : false
}

/** 子任务归属的父待办文本。 */
function parentTextOf(todo: Todo): string {
  return todos.value.find((t) => t.id === todo.parent_id)?.content ?? ''
}

function repeatLabel(todo: Todo): string {
  if (todo.repeat_rule === 'daily') return '每天重复'
  if (todo.repeat_rule === 'weekly') return '每周重复'
  return ''
}

const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return items.value.filter((item) => {
    if (filter.value !== 'all' && item.kind !== filter.value) return false
    return !key || haystackOf(item).includes(key)
  })
})

const emptyHint = computed(() =>
  keyword.value.trim() ? `未找到匹配「${keyword.value.trim()}」的记录` : `该日暂无${EMPTY_BY_FILTER[filter.value]}`,
)

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

/** 筛选 chip 的键盘触发（role="button" 的 span 原生不响应 Enter / Space）。 */
function onFilterKey(event: KeyboardEvent, key: typeof filter.value): void {
  event.preventDefault()
  filter.value = key
}

// ── 编辑：按类别分流到各自的弹窗 / 窗口 ──

const editNote = ref<Note | null>(null)
const editClip = ref<ClipboardEntry | null>(null)
const editTodo = ref<Todo | null>(null)

function edit(item: DayDetailItem): void {
  if (item.kind === 'note' && item.note) {
    // 思维导图笔记：弹出独立编辑窗口（与笔记归档列表行为一致）。
    if (item.note.editor_mode === 'mindmap') {
      logger.info('day-view', `打开思维导图窗口 id=${item.note.id}`)
      void api.windows.mindmapOpen(item.note.id).catch((error) => {
        logger.error('day-view', '打开思维导图窗口失败', error)
        toast('打开思维导图失败')
      })
      return
    }
    editNote.value = item.note
  } else if (item.kind === 'clip' && item.clip) {
    editClip.value = item.clip
  } else if (item.kind === 'todo' && item.todo) {
    if (item.todo.status === 'done') {
      toast('已完成的待办不允许修改')
      return
    }
    editTodo.value = item.todo
  }
}

async function saveNote(input: NoteInput): Promise<void> {
  try {
    await api.notes.save(input)
    toast('已保存')
    await load()
  } catch (error) {
    logger.error('day-view', '保存笔记失败', error)
    toast('保存失败')
  } finally {
    editNote.value = null
  }
}

async function saveClip(content: string): Promise<void> {
  const target = editClip.value
  if (!target) return
  try {
    await api.clipboard.update(target.id, content)
    toast('已保存')
    await load()
  } catch (error) {
    logger.error('day-view', '保存剪贴板内容失败', error)
    toast('保存失败')
  } finally {
    editClip.value = null
  }
}

async function saveTodo(input: TodoInput): Promise<void> {
  try {
    await api.todos.save(input)
    toast('已保存')
    await load()
  } catch (error) {
    logger.error('day-view', '保存待办失败', error)
    toast(String(error))
  } finally {
    editTodo.value = null
  }
}

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
      <div class="day-filters" role="group" aria-label="按类别筛选当日记录">
        <span
          v-for="option in FILTERS"
          :key="option.key"
          class="day-filter"
          :class="[option.cls, { active: filter === option.key }]"
          role="button"
          tabindex="0"
          :aria-pressed="filter === option.key"
          @click="filter = option.key"
          @keydown.enter="onFilterKey($event, option.key)"
          @keydown.space="onFilterKey($event, option.key)"
        >
          <template v-if="option.icon"
            ><span class="ix">{{ option.icon }}</span> </template
          >{{ option.label }}
        </span>
      </div>
      <input v-model="keyword" class="search-input day-search" placeholder="🔍 在该日记录中搜索…" />
    </div>

    <div class="day-hint">按时间先后排序（待办取完成时间）· 悬浮卡片可编辑 / 删除</div>

    <div v-stagger-list>
      <div
        v-for="item in visible"
        :key="idOf(item)"
        class="day-item"
        :class="[item.kind, { done: item.todo?.status === 'done' }]"
        :data-kind="item.kind"
      >
        <ConfirmPopover
          v-if="confirm.isPending(idOf(item))"
          text="⚠️ 确认删除该记录？"
          @confirm="remove(item)"
          @cancel="confirm.cancel()"
        />

        <span class="day-time">{{ formatClock(item.time) }}</span>
        <span class="day-badge" :class="item.kind">{{ TYPE_LABEL[item.kind] ?? item.kind }}</span>

        <div class="day-body">
          <div class="day-title-row">
            <PriorityBadge
              v-if="item.kind === 'todo' && item.todo"
              :priority="item.todo.priority as Priority"
              readonly
            />
            <ClipTypeBadge v-if="item.kind === 'clip' && item.clip" :content-type="item.clip.content_type" />
            <span v-if="item.note?.editor_mode === 'mindmap'" class="clip-type mindmap"
              ><span class="ix">🧠</span> 思维导图</span
            >
            <div
              class="day-text"
              :class="item.kind === 'note' ? 'd-clamp4' : 'd-clamp3'"
              :title="textOf(item)"
              v-html="renderMarkdownInline(textOf(item))"
            />
            <span v-if="overdueOf(item)" class="day-overdue">逾期</span>
          </div>

          <div
            v-if="item.todo && (item.todo.remind_offset_minutes !== null || repeatLabel(item.todo))"
            class="day-meta"
          >
            <span v-if="item.todo.remind_offset_minutes !== null" class="todo-meta"
              ><span class="ix">⏰</span> {{ remindOffsetLabel(item.todo.remind_offset_minutes) }}</span
            >
            <span v-if="repeatLabel(item.todo)" class="todo-meta"
              ><span class="ix">🔁</span> {{ repeatLabel(item.todo) }}</span
            >
          </div>

          <div v-if="(item.note?.tags ?? item.todo?.tags ?? []).length" class="day-tags">
            <span v-for="tag in item.note?.tags ?? item.todo?.tags ?? []" :key="tag" class="tag-chip todo-tag">
              <span class="tag-name">{{ tag }}</span>
            </span>
          </div>

          <div v-if="item.todo?.remark" class="day-remark" :title="item.todo.remark">{{ item.todo.remark }}</div>
          <div v-if="item.todo?.parent_id" class="day-sub">子任务 · 属于「{{ parentTextOf(item.todo) }}」</div>
        </div>

        <div class="day-ops">
          <IconBtn data-dayact="edit" title="编辑" @click="edit(item)"><Icon name="edit" /></IconBtn>
          <IconBtn data-dayact="del" title="删除" @click="confirm.ask(idOf(item))"><Icon name="close" /></IconBtn>
        </div>
      </div>

      <div v-if="!visible.length" class="todo-empty">{{ emptyHint }}</div>
    </div>

    <!-- 编辑弹窗：按类别只会打开其一 -->
    <NoteEditModal v-if="editNote" :note="editNote" @save="saveNote" @close="editNote = null" />
    <ClipEditorModal v-if="editClip" :content="editClip.content" @save="saveClip" @close="editClip = null" />
    <TodoEditorModal v-if="editTodo" mode="edit" :todo="editTodo" @save="saveTodo" @close="editTodo = null" />
  </div>
</template>
