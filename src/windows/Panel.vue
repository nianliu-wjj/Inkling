<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { api } from '@/service/tauri'
import type { ClipboardEntry, Priority, Todo } from '@/typings/domain'

type PanelMode = 'note' | 'clipboard' | 'todo'

const mode = ref<PanelMode>('note')
const content = ref('')
const tags = ref<string[]>([])
const tagInput = ref('')
const showTagEditor = ref(false)
const status = ref('已暂存')
const busy = ref(false)
const panelElement = ref<HTMLElement | null>(null)
const editorElement = ref<HTMLElement | null>(null)
const editorFocused = ref(true)
const clips = ref<ClipboardEntry[]>([])
const clipSearch = ref('')
const editingClipId = ref<string | null>(null)
const clipEditContent = ref('')
const todos = ref<Todo[]>([])
const todoSearch = ref('')
const todoPriority = ref<'all' | Priority>('all')
const activePriorityId = ref<string | null>(null)
const showTodoEditor = ref(false)
const todoContent = ref('')
const todoDueAt = ref(toLocalInput(new Date(Date.now() + 60 * 60 * 1000)))
const todoEditorPriority = ref<Priority>('medium')
let resizeObserver: ResizeObserver | undefined
let unlistenPanelShown: (() => void) | undefined
let saveTimer: number | undefined
let hideTimer: number | undefined
let lastHeight = 0

const modeItems: Array<{ value: PanelMode; symbol: string; label: string; shortcut: string }> = [
  { value: 'note', symbol: '🔴', label: '笔记', shortcut: '⌃1' },
  { value: 'clipboard', symbol: '🟡', label: '粘贴板', shortcut: '⌃2' },
  { value: 'todo', symbol: '🟢', label: '待办', shortcut: '⌃3' },
]

const filteredClips = computed(() => {
  const keyword = clipSearch.value.trim().toLowerCase()
  return [...clips.value]
    .filter((clip) => !keyword || `${clip.content} ${clip.preview}`.toLowerCase().includes(keyword))
    .sort((left, right) => Number(right.pinned) - Number(left.pinned) || right.copied_at.localeCompare(left.copied_at))
})

const filteredTodos = computed(() => {
  const keyword = todoSearch.value.trim().toLowerCase()
  const today = dateKey(new Date())
  return [...todos.value]
    .filter((todo) => {
      const dueDate = dateKey(new Date(todo.due_at))
      const isToday = dueDate === today
      const overdue = todo.status === 'open' && new Date(todo.due_at).getTime() < Date.now()
      const matchesDay = isToday || overdue
      const matchesPriority = todoPriority.value === 'all' || todo.priority === todoPriority.value
      const matchesKeyword =
        !keyword || `${todo.content} ${todo.remark} ${todo.tags.join(' ')}`.toLowerCase().includes(keyword)
      return matchesDay && matchesPriority && matchesKeyword
    })
    .sort(todoSort)
})

const isTodoFormValid = computed(() => todoContent.value.trim().length > 0 && Boolean(todoDueAt.value))

function toLocalInput(value: Date) {
  const offset = value.getTimezoneOffset()
  return new Date(value.getTime() - offset * 60_000).toISOString().slice(0, 16)
}

function dateKey(value: Date) {
  const year = value.getFullYear()
  const month = String(value.getMonth() + 1).padStart(2, '0')
  const day = String(value.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

function formatTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
}

function formatDue(todo: Todo) {
  const due = new Date(todo.due_at)
  if (Number.isNaN(due.getTime())) return '无时间'
  const overdue = todo.status === 'open' && due.getTime() < Date.now()
  return `${overdue ? '逾期 ' : ''}${formatTime(todo.due_at)}`
}

function isOverdue(todo: Todo) {
  return todo.status === 'open' && new Date(todo.due_at).getTime() < Date.now()
}

function todoSort(left: Todo, right: Todo) {
  const overdueDiff = Number(isOverdue(right)) - Number(isOverdue(left))
  if (overdueDiff) return overdueDiff
  const statusDiff = Number(left.status === 'done') - Number(right.status === 'done')
  if (statusDiff) return statusDiff
  return new Date(left.due_at).getTime() - new Date(right.due_at).getTime()
}

function escapeHtml(value: string) {
  return value.replace(
    /[&<>"']/g,
    (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character]!,
  )
}

function renderMarkdown(value: string) {
  return escapeHtml(value)
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\n/g, '<br>')
}

async function syncPanelHeight() {
  await nextTick()
  const height = Math.ceil(panelElement.value?.getBoundingClientRect().height || 0)
  if (!height || Math.abs(height - lastHeight) < 2) return
  lastHeight = height
  await api.windows.panelResize(height).catch(() => undefined)
}

function focusEditor() {
  editorFocused.value = true
  void nextTick(() => {
    editorElement.value?.focus()
    const selection = window.getSelection()
    const range = document.createRange()
    if (editorElement.value) {
      range.selectNodeContents(editorElement.value)
      range.collapse(false)
      selection?.removeAllRanges()
      selection?.addRange(range)
    }
  })
}

function setEditorContent(value: string) {
  content.value = value
  editorFocused.value = true
  void nextTick(() => {
    if (editorElement.value && editorElement.value.innerText !== value) editorElement.value.innerText = value
  })
}

function onEditorInput(event: Event) {
  content.value = (event.target as HTMLElement).innerText.replace(/\u00a0/g, ' ')
  status.value = '输入中…'
  window.clearTimeout(saveTimer)
  saveTimer = window.setTimeout(() => void saveDraft(), 500)
}

function onEditorBlur() {
  if (content.value.trim()) editorFocused.value = false
}

async function saveDraft() {
  if (!content.value.trim()) {
    status.value = '已暂存'
    return
  }
  await api.notes
    .save({ id: 'draft-main', content: content.value, tags: tags.value, draft: true })
    .then(() => {
      status.value = '已暂存'
    })
    .catch((error) => {
      status.value = `暂存失败：${String(error)}`
    })
}

async function loadDraft() {
  const draft = await api.notes.draft().catch(() => null)
  if (!draft) {
    setEditorContent('')
    tags.value = []
    status.value = '已暂存'
    focusEditor()
    return
  }
  setEditorContent(draft.content)
  tags.value = [...draft.tags]
  status.value = '已暂存'
  focusEditor()
}

async function archiveNote() {
  if (!content.value.trim() || busy.value) return
  busy.value = true
  status.value = '归档中…'
  try {
    await api.notes.save({ content: content.value.trim(), tags: tags.value, draft: false })
    await api.notes.remove('draft-main').catch(() => undefined)
    setEditorContent('')
    tags.value = []
    showTagEditor.value = false
    status.value = '念头已归档'
  } catch (error) {
    status.value = `归档失败：${String(error)}`
  } finally {
    busy.value = false
  }
}

function addTag() {
  const value = tagInput.value.trim().replace(/^#/, '')
  if (!value || tags.value.includes(value) || tags.value.length >= 8) return
  tags.value.push(value)
  tagInput.value = ''
  void saveDraft()
}

function removeTag(tag: string) {
  tags.value = tags.value.filter((item) => item !== tag)
  void saveDraft()
}

async function switchMode(nextMode: PanelMode) {
  mode.value = nextMode
  activePriorityId.value = null
  if (nextMode === 'clipboard') await loadClips()
  if (nextMode === 'todo') await loadTodos()
  if (nextMode === 'note') focusEditor()
  await syncPanelHeight()
}

async function loadClips() {
  clips.value = await api.clipboard.list().catch(() => [])
}

async function captureClipboard() {
  busy.value = true
  try {
    const clip = await api.clipboard.capture()
    if (clip) {
      clips.value = [clip, ...clips.value.filter((item) => item.id !== clip.id)]
      status.value = '剪贴板已捕获'
    } else {
      status.value = '剪贴板为空'
    }
  } catch (error) {
    status.value = `捕获失败：${String(error)}`
  } finally {
    busy.value = false
  }
}

async function writeClipboard(clip: ClipboardEntry, pin = false) {
  await api.clipboard.write(clip.id)
  if (pin && !clip.pinned) {
    await api.clipboard.pin(clip.id, true)
    clip.pinned = true
  }
  status.value = pin ? '已粘贴并置顶' : '已写回剪贴板'
}

async function toggleClipPin(clip: ClipboardEntry) {
  await api.clipboard.pin(clip.id, !clip.pinned)
  clip.pinned = !clip.pinned
}

function startClipEdit(clip: ClipboardEntry) {
  editingClipId.value = clip.id
  clipEditContent.value = clip.content
}

async function saveClipEdit(clip: ClipboardEntry) {
  const next = clipEditContent.value.trim()
  if (!next) return
  const updated = await api.clipboard.update(clip.id, next)
  Object.assign(clip, updated)
  editingClipId.value = null
  clipEditContent.value = ''
}

async function removeClip(clip: ClipboardEntry) {
  if (!window.confirm('确认删除该剪贴板条目？')) return
  await api.clipboard.remove(clip.id)
  clips.value = clips.value.filter((item) => item.id !== clip.id)
}

async function loadTodos() {
  todos.value = await api.todos.list().catch(() => [])
}

function cyclePriority(todo: Todo) {
  if (todo.status === 'done') return
  activePriorityId.value = activePriorityId.value === todo.id ? null : todo.id
}

async function selectPriority(todo: Todo, priority: Priority) {
  if (todo.status === 'done') return
  if (priority !== todo.priority) {
    const updated = await api.todos.priority(todo.id, priority)
    Object.assign(todo, updated)
  }
  activePriorityId.value = null
}

async function completeTodo(todo: Todo) {
  if (todo.status === 'done') return
  const updated = await api.todos.complete(todo.id, true)
  const changed = new Map(updated.map((item) => [item.id, item]))
  todos.value = todos.value.map((item) => changed.get(item.id) ?? item)
}

function openTodoEditor() {
  showTodoEditor.value = !showTodoEditor.value
  if (showTodoEditor.value) {
    todoContent.value = ''
    todoDueAt.value = toLocalInput(new Date(Date.now() + 60 * 60 * 1000))
    todoEditorPriority.value = 'medium'
    void nextTick(() => document.querySelector<HTMLInputElement>('.todo-create-input')?.focus())
  }
}

async function saveTodo() {
  if (!isTodoFormValid.value || busy.value) return
  busy.value = true
  try {
    const todo = await api.todos.save({
      content: todoContent.value.trim(),
      dueAt: new Date(todoDueAt.value).toISOString(),
      priority: todoEditorPriority.value,
      remark: '',
      tags: [],
      parentId: null,
      remindAt: null,
      repeatRule: null,
    })
    todos.value.push(todo)
    showTodoEditor.value = false
    todoContent.value = ''
  } finally {
    busy.value = false
  }
}

function close() {
  window.clearTimeout(hideTimer)
  void api.windows.panelHide()
}

function scheduleHide() {
  window.clearTimeout(hideTimer)
  hideTimer = window.setTimeout(close, 3000)
}

function cancelHide() {
  window.clearTimeout(hideTimer)
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    close()
    return
  }
  if (event.ctrlKey || event.metaKey) {
    const shortcut = event.code
    if (shortcut === 'Digit1') {
      event.preventDefault()
      void switchMode('note')
    } else if (shortcut === 'Digit2') {
      event.preventDefault()
      void switchMode('clipboard')
    } else if (shortcut === 'Digit3') {
      event.preventDefault()
      void switchMode('todo')
    }
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  resizeObserver = new ResizeObserver(() => void syncPanelHeight())
  if (panelElement.value) resizeObserver.observe(panelElement.value)
  void loadDraft()
  void listen('inkling://panel-shown', () => {
    cancelHide()
    void loadDraft()
  })
    .then((unlisten) => {
      unlistenPanelShown = unlisten
    })
    .catch(() => undefined)
  void syncPanelHeight()
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.clearTimeout(saveTimer)
  window.clearTimeout(hideTimer)
  resizeObserver?.disconnect()
  unlistenPanelShown?.()
})

watch([mode, showTagEditor, showTodoEditor], () => void syncPanelHeight())
</script>

<template>
  <section ref="panelElement" class="quick-panel" @mouseenter="cancelHide" @mouseleave="scheduleHide">
    <nav class="panel-nav" aria-label="快速捕获模式">
      <button
        v-for="item in modeItems"
        :key="item.value"
        class="nav-dot"
        :class="{ active: mode === item.value }"
        :title="`${item.label}（${item.shortcut}）`"
        @click="void switchMode(item.value)"
      >
        {{ item.symbol }}
      </button>
      <span class="panel-hint">Esc 收起</span>
    </nav>

    <section v-if="mode === 'note'" class="panel-page note-page">
      <div
        v-if="editorFocused || !content.trim()"
        ref="editorElement"
        class="editor"
        contenteditable="true"
        data-placeholder="此刻在想什么？直接写下来… 支持 **Markdown** 即时渲染，右下角管理标签"
        spellcheck="true"
        @input="onEditorInput"
        @focus="editorFocused = true"
        @blur="onEditorBlur"
        @keydown.enter.exact.prevent="void archiveNote()"
      />
      <div v-else class="editor markdown-preview" tabindex="0" @focus="focusEditor" v-html="renderMarkdown(content)" />
      <div class="editor-footer">
        <span class="save-state" :class="{ saving: status.includes('中') }">{{ status }}</span>
        <div class="editor-actions">
          <button class="tag-preview" title="点击管理标签" @click="showTagEditor = !showTagEditor">
            <span v-if="!tags.length" class="tag-empty">无标签</span>
            <span v-for="tag in tags.slice(0, 4)" :key="tag" class="tag-chip">#{{ tag }}</span>
            <span v-if="tags.length > 4" class="tag-more">+{{ tags.length - 4 }}</span>
          </button>
          <button class="btn primary" :disabled="busy || !content.trim()" @click="void archiveNote()">
            归档念头 ↵
          </button>
        </div>
      </div>
      <div v-if="showTagEditor" class="tag-editor">
        <div class="tag-list">
          <button v-for="tag in tags" :key="tag" class="tag-chip removable" @click="removeTag(tag)">
            #{{ tag }} ×
          </button>
        </div>
        <input
          v-model="tagInput"
          class="tag-input"
          maxlength="10"
          placeholder="添加标签，回车确认"
          @keydown.enter.prevent="addTag"
        />
      </div>
    </section>

    <section v-else-if="mode === 'clipboard'" class="panel-page clipboard-page">
      <div class="clipboard-toolbar">
        <input v-model="clipSearch" class="search-input" placeholder="搜索剪贴板历史…（双击条目 = 粘贴并置顶）" />
        <button class="btn tiny" :disabled="busy" title="捕获当前系统剪贴板" @click="void captureClipboard">
          捕获
        </button>
      </div>
      <ul v-if="filteredClips.length" class="clip-list">
        <li
          v-for="clip in filteredClips"
          :key="clip.id"
          class="clip-item"
          :class="{ pinned: clip.pinned }"
          @dblclick="void writeClipboard(clip, true)"
        >
          <div class="clip-head">
            <span class="clip-time"
              >{{ clip.pinned ? '📌 ' : '' }}{{ formatTime(clip.modified_at || clip.copied_at) }}</span
            >
            <span class="clip-type" :class="clip.content_type">{{
              clip.content_type === 'link'
                ? '链接'
                : clip.content_type === 'code'
                  ? '代码'
                  : clip.content_type === 'image'
                    ? '图片'
                    : '文本'
            }}</span>
          </div>
          <template v-if="editingClipId === clip.id">
            <textarea v-model="clipEditContent" class="clip-edit-input" rows="3" />
            <div class="clip-ops editing-ops">
              <button class="icon-btn" @click.stop="void saveClipEdit(clip)">保存</button>
              <button class="icon-btn" @click.stop="editingClipId = null">取消</button>
            </div>
          </template>
          <template v-else>
            <div class="clip-text">{{ clip.preview || clip.content }}</div>
            <div class="clip-ops">
              <button class="icon-btn" title="写回剪贴板" @click.stop="void writeClipboard(clip)">粘贴</button>
              <button class="icon-btn" title="编辑内容" @click.stop="startClipEdit(clip)">编辑</button>
              <button
                class="icon-btn"
                :class="{ active: clip.pinned }"
                :title="clip.pinned ? '取消置顶' : '置顶'"
                @click.stop="void toggleClipPin(clip)"
              >
                {{ clip.pinned ? '取消置顶' : '置顶' }}
              </button>
              <button class="icon-btn danger" title="删除条目" @click.stop="void removeClip(clip)">删除</button>
            </div>
          </template>
        </li>
      </ul>
      <div v-else class="empty-state">暂无剪贴板记录</div>
    </section>

    <section v-else class="panel-page todo-page">
      <div class="todo-input-row">
        <input v-model="todoSearch" class="search-input" placeholder="🔍 搜索当日待办…" />
        <select v-model="todoPriority" class="prio-select" title="按优先级过滤">
          <option value="all">全部</option>
          <option value="high">🔴 高</option>
          <option value="medium">🟡 中</option>
          <option value="low">🟢 低</option>
        </select>
        <button class="btn tiny" title="新增待办事项（可设置完成时间）" @click="openTodoEditor">📅</button>
      </div>
      <div class="todo-panel-hint">搜索 / 优先级过滤当日待办 · 逾期事项自动置顶 · 点击优先级徽章可调整</div>
      <form v-if="showTodoEditor" class="todo-create" @submit.prevent="void saveTodo()">
        <input v-model="todoContent" class="todo-create-input" placeholder="新增待办内容…" maxlength="200" />
        <input v-model="todoDueAt" type="datetime-local" class="todo-due-input" />
        <select v-model="todoEditorPriority" class="prio-select">
          <option value="high">🔴 高</option>
          <option value="medium">🟡 中</option>
          <option value="low">🟢 低</option>
        </select>
        <button class="btn primary tiny" type="submit" :disabled="busy || !isTodoFormValid">添加</button>
      </form>
      <ul v-if="filteredTodos.length" class="todo-list">
        <li
          v-for="todo in filteredTodos"
          :key="todo.id"
          class="todo-item"
          :class="{ done: todo.status === 'done', overdue: isOverdue(todo) }"
        >
          <button
            class="todo-check"
            :disabled="todo.status === 'done'"
            :title="todo.status === 'done' ? '已完成' : '标记完成'"
            @click="void completeTodo(todo)"
          >
            {{ todo.status === 'done' ? '✓' : '' }}
          </button>
          <div class="todo-main">
            <div class="todo-content">{{ todo.content }}</div>
            <div class="todo-meta">
              <span class="todo-due">{{ formatDue(todo) }}</span>
              <span v-if="todo.parent_id" class="todo-child">子任务</span>
            </div>
          </div>
          <div class="priority-wrap">
            <button
              class="priority-badge"
              :class="todo.priority"
              :disabled="todo.status === 'done'"
              @click="cyclePriority(todo)"
            >
              {{ todo.priority === 'high' ? '高' : todo.priority === 'low' ? '低' : '中' }}
            </button>
            <div v-if="activePriorityId === todo.id" class="priority-menu">
              <button
                v-for="priority in ['high', 'medium', 'low'] as Priority[]"
                :key="priority"
                :class="{ selected: todo.priority === priority }"
                @click="void selectPriority(todo, priority)"
              >
                {{ priority === 'high' ? '🔴 高' : priority === 'low' ? '🟢 低' : '🟡 中' }}
              </button>
            </div>
          </div>
        </li>
      </ul>
      <div v-else class="empty-state">今天没有待办事项</div>
    </section>
  </section>
</template>
