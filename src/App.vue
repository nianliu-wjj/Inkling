<template>
  <div class="app-shell" :data-theme="settings.theme">
    <header class="titlebar">
      <div class="brand"><span class="brand-mark">✒</span><span>Inkling</span><small>念头捕手</small></div>
      <div class="mode-switcher" role="tablist" aria-label="快速捕获模式">
        <button
          v-for="item in captureModes"
          :key="item.key"
          :class="['mode-dot', item.key, { active: captureMode === item.key }]"
          @click="switchCapture(item.key)"
          :title="item.label"
        >
          {{ item.icon }}
        </button>
      </div>
      <div class="titlebar-actions">
        <span class="save-indicator">{{ loading ? '同步中…' : message || '本地已保存' }}</span
        ><button class="icon-button" title="隐藏窗口" @click="api.windows.hideMain()">⌃</button>
      </div>
    </header>

    <main class="workspace">
      <aside class="sidebar">
        <button
          v-for="item in navItems"
          :key="item.key"
          :class="['nav-item', item.key, { active: view === item.key }]"
          @click="view = item.key"
        >
          <span>{{ item.icon }}</span
          ><span>{{ item.label }}</span
          ><b>{{ item.count }}</b>
        </button>
        <div class="sidebar-spacer" />
        <div class="quick-capture"><span>快捷捕获</span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>Space</kbd></div>
        <button class="nav-item settings" :class="{ active: view === 'settings' }" @click="view = 'settings'">
          <span>⚙</span><span>偏好设置</span>
        </button>
      </aside>

      <section class="content">
        <div v-if="view === 'notes'" class="view-page">
          <section class="capture-card note-capture">
            <div class="section-title">
              <div>
                <span class="eyebrow red">RED · NOTE</span>
                <h1>此刻在想什么？</h1>
              </div>
              <button class="link-button" @click="newNote">清空</button>
            </div>
            <textarea v-model="noteDraft" placeholder="写下一个念头，支持 Markdown…" @input="scheduleDraft"></textarea>
            <div class="capture-footer">
              <input v-model="noteTags" placeholder="标签，以逗号分隔" /><button
                class="primary-button"
                :disabled="!noteDraft.trim()"
                @click="archiveNote"
              >
                归档念头 ↵
              </button>
            </div>
          </section>
          <div class="page-heading">
            <div>
              <span class="eyebrow">ARCHIVE</span>
              <h2>最近归档</h2>
            </div>
            <input v-model="search" class="search" placeholder="搜索笔记…" />
          </div>
          <div class="card-grid">
            <article v-for="note in filteredNotes" :key="note.id" class="data-card note-card">
              <div class="card-actions">
                <button @click="api.windows.pinCreate('note', note.id)" title="桌面置顶">📌</button
                ><button :class="{ pinned: note.pinned }" @click="toggleNotePin(note)" title="置顶">
                  {{ note.pinned ? '★' : '☆' }}</button
                ><button @click="removeNote(note)" title="删除">×</button>
              </div>
              <div class="card-content markdown" v-html="renderMarkdown(note.content)"></div>
              <div class="card-meta">
                <time>{{ formatTime(note.updated_at) }}</time
                ><span v-for="tag in note.tags.slice(0, 3)" :key="tag" class="tag">#{{ tag }}</span
                ><span v-if="note.tags.length > 3" class="tag">+{{ note.tags.length - 3 }}</span
                ><button class="mini-action" @click="editNote(note)">编辑</button>
              </div>
            </article>
            <div v-if="!filteredNotes.length" class="empty">还没有归档笔记，先捕获一个念头吧。</div>
          </div>
        </div>

        <div v-else-if="view === 'clips'" class="view-page">
          <div class="page-heading">
            <div>
              <span class="eyebrow yellow">YELLOW · CLIPBOARD</span>
              <h2>剪贴板历史</h2>
            </div>
            <div class="heading-actions">
              <input v-model="search" class="search" placeholder="搜索剪贴板…" /><button
                class="secondary-button"
                @click="captureClipboard"
              >
                捕获当前剪贴板
              </button>
            </div>
          </div>
          <div class="stack-list">
            <article
              v-for="clip in filteredClips"
              :key="clip.id"
              class="data-card clip-card"
              @dblclick="pasteClip(clip)"
            >
              <div class="clip-type">
                {{ clip.content_type === 'link' ? '↗' : clip.content_type === 'code' ? '代码' : '▤' }}
              </div>
              <div class="clip-body">
                <div class="clip-preview">{{ clip.preview }}</div>
                <time>{{ formatTime(clip.modified_at) }}</time>
              </div>
              <div class="card-actions visible">
                <button :class="{ pinned: clip.pinned }" @click.stop="toggleClipPin(clip)">
                  {{ clip.pinned ? '★' : '☆' }}</button
                ><button @click.stop="editClip(clip)">编辑</button><button @click.stop="removeClip(clip)">×</button>
              </div>
            </article>
            <div v-if="!filteredClips.length" class="empty">剪贴板暂无记录。</div>
          </div>
        </div>

        <div v-else-if="view === 'todos'" class="view-page">
          <div class="page-heading">
            <div>
              <span class="eyebrow green">GREEN · TODO</span>
              <h2>待办事项</h2>
            </div>
            <div class="heading-actions">
              <input v-model="todoDate" class="date-filter" type="date" aria-label="选择待办日期" />
              <input v-model="search" class="search" placeholder="搜索待办（跨日期）…" />
              <button class="primary-button" @click="openTodoEditor()">＋ 新建待办</button>
            </div>
          </div>
          <div class="todo-list">
            <article
              v-for="todo in filteredRootTodos"
              :key="todo.id"
              :class="['data-card', 'todo-card', { done: todo.status === 'done', overdue: isOverdue(todo) }]"
              @click="activeTodo = activeTodo === todo.id ? null : todo.id"
            >
              <div class="check" :class="{ checked: todo.status === 'done' }" @click.stop="toggleTodo(todo)">
                {{ todo.status === 'done' ? '✓' : '' }}
              </div>
              <div class="todo-body">
                <div class="todo-title">
                  <span>{{ todo.content }}</span
                  ><span :class="['priority', todo.priority]" @click.stop="changePriority(todo)">{{
                    priorityLabel(todo.priority)
                  }}</span>
                  <div v-if="activePriority === todo.id" class="priority-popover" @click.stop>
                    <button
                      v-for="option in priorityOptions"
                      :key="option.value"
                      :class="{ selected: todo.priority === option.value }"
                      @click="selectPriority(todo, option.value)"
                    >
                      {{ option.label }}<b v-if="todo.priority === option.value">✓</b>
                    </button>
                  </div>
                </div>
                <div class="todo-meta">
                  <span
                    class="due-badge"
                    :class="{ overdue: isOverdue(todo) }"
                    title="修改计划完成时间"
                    @click.stop="openDueEditor(todo)"
                  >
                    📅 {{ formatDue(todo.due_at) }}
                  </span>
                  <span v-if="isOverdue(todo)" class="overdue-text">逾期</span>
                  <span v-if="todo.remind_at">⏰ {{ formatTime(todo.remind_at) }}</span>
                  <span v-for="tag in todo.tags" :key="tag" class="tag">#{{ tag }}</span>
                </div>
                <div
                  v-if="todo.remark && settings.remark_style !== 'icon'"
                  class="remark"
                  :class="{ muted: settings.remark_style === 'text' }"
                >
                  ▣ {{ todo.remark }}
                </div>
                <div
                  v-else-if="todo.remark && settings.remark_style === 'icon'"
                  class="remark-icon"
                  :title="todo.remark"
                >
                  ▣
                </div>
                <div v-if="displayChildren(todo).length" class="children">
                  <div
                    v-for="child in displayChildren(todo)"
                    :key="child.id"
                    class="child-row"
                    :class="{ done: child.status === 'done', overdue: isOverdue(child) }"
                  >
                    <span
                      class="child-check"
                      :class="{ checked: child.status === 'done' }"
                      @click.stop="toggleTodo(child)"
                      >{{ child.status === 'done' ? '✓' : '' }}</span
                    ><span :class="{ strike: child.status === 'done' }">{{ child.content }}</span
                    ><span :class="['priority', child.priority]" @click.stop="changePriority(child)">{{
                      priorityLabel(child.priority)
                    }}</span>
                    <div v-if="activePriority === child.id" class="priority-popover child-popover" @click.stop>
                      <button
                        v-for="option in priorityOptions"
                        :key="option.value"
                        :class="{ selected: child.priority === option.value }"
                        @click="selectPriority(child, option.value)"
                      >
                        {{ option.label }}<b v-if="child.priority === option.value">✓</b>
                      </button>
                    </div>
                    <div class="child-meta">
                      <span class="due-badge" :class="{ overdue: isOverdue(child) }" @click.stop="openDueEditor(child)">
                        📅 {{ formatDue(child.due_at) }}
                      </span>
                      <span v-for="tag in child.tags" :key="tag" class="tag">#{{ tag }}</span>
                      <span v-if="child.remind_at">⏰ {{ formatTime(child.remind_at) }}</span>
                      <span
                        v-if="child.remark && settings.remark_style === 'icon'"
                        class="remark-icon"
                        :title="child.remark"
                        >▣</span
                      ><span v-else-if="child.remark" class="remark muted">▣ {{ child.remark }}</span>
                    </div>
                    <div class="child-actions">
                      <button v-if="child.status !== 'done'" @click.stop="openTodoEditor(child)">编辑</button>
                      <button v-if="child.status !== 'done'" @click.stop="removeTodo(child)">×</button>
                    </div>
                  </div>
                </div>
              </div>
              <div class="card-actions visible">
                <button @click.stop="api.windows.pinCreate('todo', todo.id)" title="桌面置顶">📌</button
                ><button v-if="todo.status === 'done'" @click.stop="addChild(todo)">＋ 子任务</button
                ><button v-else @click.stop="openTodoEditor(todo)">编辑</button
                ><button v-if="todo.status !== 'done'" @click.stop="removeTodo(todo)">×</button>
              </div>
            </article>
            <div v-if="!filteredRootTodos.length" class="empty">{{ search ? '没有匹配的待办。' : '暂无待办。' }}</div>
          </div>
        </div>

        <div v-else-if="view === 'stats'" class="view-page">
          <div class="page-heading">
            <div>
              <span class="eyebrow blue">ACTIVITY</span>
              <h2>使用统计</h2>
            </div>
          </div>
          <div class="stats-summary">
            <div>
              <b>{{ totals.notes }}</b
              ><span>笔记归档</span>
            </div>
            <div>
              <b>{{ totals.clips }}</b
              ><span>剪贴板捕获</span>
            </div>
            <div>
              <b>{{ totals.todos }}</b
              ><span>待办创建</span>
            </div>
            <div>
              <b>{{ totals.completed }}</b
              ><span>待办完成</span>
            </div>
          </div>
          <div class="stats-summary compact-summary">
            <div>
              <b>{{ summary.overdue }}</b
              ><span>当前逾期</span>
            </div>
          </div>
          <section class="data-card heatmap-card">
            <div class="section-title">
              <h3>近 180 天活跃度</h3>
              <span class="muted">点击日期查看详情</span>
            </div>
            <div class="heatmap">
              <button
                v-for="day in activity"
                :key="day.date"
                :class="[
                  'heat-cell',
                  heatLevel(day),
                  { overdue: day.overdue > 0, selected: selectedStatsDate === day.date },
                ]"
                :title="`${day.date}：笔记 ${day.notes}，剪贴板 ${day.clips}，待办 ${day.todos}，完成 ${day.completed}，逾期 ${day.overdue}`"
                @click="loadDayDetails(day.date)"
              />
            </div>
            <div class="legend">
              <span>少</span><i class="heat-cell level-0" /><i class="heat-cell level-1" /><i
                class="heat-cell level-2"
              /><i class="heat-cell level-3" /><span>多</span><em>红框表示存在逾期</em>
            </div>
          </section>
          <section class="data-card trend-card">
            <div class="section-title">
              <h3>近 6 个月趋势</h3>
              <div class="heading-actions">
                <button class="secondary-button" @click="exportSelected('md')">导出 Markdown</button
                ><button class="secondary-button" @click="exportSelected('html')">导出 HTML</button>
              </div>
            </div>
            <div class="trend-list">
              <div v-for="item in trend" :key="item.month" class="trend-row">
                <strong>{{ item.month }}</strong
                ><span>笔记 {{ item.notes }}</span
                ><span>剪贴板 {{ item.clips }}</span
                ><span>待办 {{ item.todos }}</span
                ><span>完成 {{ item.completed }}</span>
              </div>
            </div>
          </section>
          <section class="data-card day-detail-card">
            <div class="section-title">
              <h3>{{ selectedStatsDate }} 详情</h3>
              <span class="muted">{{ dayDetails.length }} 条记录</span>
            </div>
            <div v-if="dayDetails.length" class="day-detail-list">
              <div
                v-for="item in dayDetails"
                :key="`${item.kind}-${item.time}-${item.note?.id || item.clip?.id || item.todo?.id}`"
                class="day-detail-row"
              >
                <span class="detail-kind">{{
                  item.kind === 'note' ? '笔记' : item.kind === 'clip' ? '剪贴板' : '待办'
                }}</span
                ><time>{{ formatTime(item.time) }}</time
                ><span class="detail-content">{{
                  item.note?.content || item.clip?.preview || item.todo?.content
                }}</span>
              </div>
            </div>
            <div v-else class="empty">这一天没有记录。</div>
          </section>
        </div>

        <div v-else class="view-page">
          <div class="page-heading">
            <div>
              <span class="eyebrow">SETTINGS</span>
              <h2>偏好设置</h2>
            </div>
          </div>
          <section class="settings-card data-card">
            <label
              >主题<select v-model="settings.theme" @change="saveSettings">
                <option v-for="theme in themes" :key="theme.key" :value="theme.key">{{ theme.label }}</option>
              </select></label
            ><label
              >失焦收起<select v-model="settings.collapse_policy" @change="saveSettings">
                <option value="immediate">立即收起</option>
                <option value="3s">延迟 3 秒</option>
                <option value="never">固定不收起</option>
              </select></label
            ><label
              >剪贴板保留天数<input
                v-model.number="settings.clipboard_retention_days"
                type="number"
                min="1"
                max="365"
                @change="saveSettings" /></label
            ><label
              >备注展示样式<select v-model="settings.remark_style" @change="saveSettings">
                <option value="mixed">混合模式</option>
                <option value="icon">图标徽章 + 悬浮</option>
                <option value="text">置灰文本行</option>
              </select></label
            ><label>全局快捷键<input v-model="settings.shortcut" @change="saveSettings" /></label
            ><label class="switch-line"
              ><span>开机静默自启动</span
              ><input v-model="settings.start_on_boot" type="checkbox" @change="saveSettings"
            /></label>
          </section>
        </div>
      </section>
    </main>

    <div v-if="todoEditor" class="modal-backdrop" @click.self="todoEditor = false">
      <form class="modal" @submit.prevent="saveTodo">
        <div class="modal-head">
          <h3>{{ todoForm.parent_id ? '新增子任务' : editingTodoRef ? '编辑待办' : '新建待办' }}</h3>
          <button type="button" @click="todoEditor = false">×</button>
        </div>
        <label>内容<input v-model="todoForm.content" autofocus required /></label>
        <div class="form-grid">
          <label>计划完成时间<input v-model="todoForm.due_at" type="datetime-local" required /></label
          ><label
            >优先级<select v-model="todoForm.priority">
              <option value="high">高</option>
              <option value="medium">中</option>
              <option value="low">低</option>
            </select></label
          >
        </div>
        <div class="form-grid">
          <label>提醒时间<input v-model="todoForm.remind_at" type="datetime-local" /></label
          ><label
            >重复提醒<select v-model="todoForm.repeat_rule">
              <option value="">不重复</option>
              <option value="daily">每天</option>
              <option value="weekly">每周</option>
            </select></label
          >
        </div>
        <label>标签<input v-model="todoForm.tagsText" placeholder="最多 3 个，每个不超过 10 字" /></label
        ><label
          >备注<textarea v-model="todoForm.remark" rows="3" maxlength="200" /><small class="field-hint"
            >{{ todoForm.remark.length }}/200</small
          ></label
        >
        <div class="modal-actions">
          <button type="button" class="secondary-button" @click="todoEditor = false">取消</button
          ><button class="primary-button">保存</button>
        </div>
      </form>
    </div>
    <div v-if="dueEditor" class="modal-backdrop" @click.self="dueEditor = null">
      <form class="modal compact-modal" @submit.prevent="saveDueEdit">
        <div class="modal-head">
          <h3>修改计划完成时间</h3>
          <button type="button" @click="dueEditor = null">×</button>
        </div>
        <label>完成日期与时刻<input v-model="dueForm" type="datetime-local" required /></label>
        <div class="modal-actions">
          <button type="button" class="secondary-button" @click="dueEditor = null">取消</button
          ><button class="primary-button">保存</button>
        </div>
      </form>
    </div>
    <div v-if="clipEditor" class="modal-backdrop" @click.self="clipEditor = null">
      <form class="modal" @submit.prevent="saveClipEdit">
        <div class="modal-head">
          <h3>编辑剪贴板条目</h3>
          <button type="button" @click="clipEditor = null">×</button>
        </div>
        <textarea v-model="clipEditor.content" rows="10" autofocus />
        <div class="modal-actions">
          <button type="button" class="secondary-button" @click="clipEditor = null">取消</button
          ><button class="primary-button">保存</button>
        </div>
      </form>
    </div>
    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { api } from '@/service/tauri'
import { listen } from '@tauri-apps/api/event'
import { captureModes } from '@/constants/capture'
import { navigationItems } from '@/constants/navigation'
import { themes } from '@/constants/themes'
import { formatDue, formatTime, renderMarkdown, toDateKey, toLocalInput } from '@/utils/format'
import type {
  ActivityDay,
  CaptureMode,
  ClipboardEntry,
  DayDetailItem,
  MonthTrend,
  Note,
  Priority,
  Settings,
  StatsSummary,
  Todo,
  View,
} from '@/typings/domain'

const captureMode = ref<CaptureMode>('note')
const view = ref<View>('notes')
const loading = ref(false)
const message = ref('')
const toast = ref('')
const search = ref('')
const notes = ref<Note[]>([])
const clips = ref<ClipboardEntry[]>([])
const todos = ref<Todo[]>([])
const activity = ref<ActivityDay[]>([])
const trend = ref<MonthTrend[]>([])
const summary = ref<StatsSummary>({ notes: 0, clips: 0, todos: 0, completed: 0, overdue: 0 })
const dayDetails = ref<DayDetailItem[]>([])
const selectedStatsDate = ref(toDateKey(new Date()))
const noteDraft = ref('')
let clipboardTimer: number | undefined
let lastClipboard = ''
const noteTags = ref('')
let draftTimer: number | undefined
const todoEditor = ref(false)
const dueEditor = ref<Todo | null>(null)
const dueForm = ref('')
const activeTodo = ref<string | null>(null)
const activePriority = ref<string | null>(null)
const clipEditor = ref<ClipboardEntry | null>(null)
const settings = reactive<Settings>({
  collapse_policy: '3s',
  clipboard_retention_days: 30,
  start_on_boot: false,
  shortcut: 'Ctrl+Shift+Space',
  remark_style: 'mixed',
  theme: 'dark',
})
const todoDate = ref(toDateKey(new Date()))
const todoForm = reactive({
  content: '',
  due_at: toLocalInput(new Date(Date.now() + 3600000)),
  remind_at: '',
  repeat_rule: '' as '' | 'daily' | 'weekly',
  priority: 'medium' as Priority,
  remark: '',
  tagsText: '',
  parent_id: null as string | null,
})
const priorityOptions = [
  { value: 'high' as Priority, label: '高优先级' },
  { value: 'medium' as Priority, label: '中优先级' },
  { value: 'low' as Priority, label: '低优先级' },
]
const navItems = computed(() =>
  navigationItems.map((item) => ({
    ...item,
    count:
      item.key === 'notes'
        ? notes.value.length
        : item.key === 'clips'
          ? clips.value.length
          : item.key === 'todos'
            ? todos.value.filter((x) => x.status === 'open').length
            : 0,
  })),
)
const filteredNotes = computed(() =>
  notes.value.filter(
    (x) =>
      !search.value ||
      x.content.toLowerCase().includes(search.value.toLowerCase()) ||
      x.tags.join(' ').toLowerCase().includes(search.value.toLowerCase()),
  ),
)
const filteredClips = computed(() =>
  clips.value.filter((x) => !search.value || x.content.toLowerCase().includes(search.value.toLowerCase())),
)
const todoSearch = computed(() => search.value.trim().toLowerCase())
const todoMatches = (todo: Todo) => {
  if (!todoSearch.value) return true
  return [todo.content, todo.remark, todo.tags.join(' ')].some((value) =>
    value.toLowerCase().includes(todoSearch.value),
  )
}
const todayDate = computed(() => toDateKey(new Date()))
const visibleTodoDate = (todo: Todo) => {
  const date = toDateKey(new Date(todo.due_at))
  if (date === todoDate.value) return true
  return todoDate.value === todayDate.value && todo.status === 'open' && date < todayDate.value
}
const childTodos = (id: string): Todo[] => todos.value.filter((todo) => todo.parent_id === id)
const hasMatchingChild = (todo: Todo) => childTodos(todo.id).some(todoMatches)
const displayChildren = (todo: Todo) => {
  const children = childrenOf(todo.id)
  return todoSearch.value && !todoMatches(todo) ? children.filter(todoMatches) : children
}
const hasVisibleChild = (todo: Todo) => childTodos(todo.id).some(visibleTodoDate)
const filteredRootTodos = computed(() => {
  const candidates = todos.value.filter((todo) => {
    if (todo.parent_id) return false
    const matchesDate = visibleTodoDate(todo) || hasVisibleChild(todo)
    const matchesSearch = !todoSearch.value || todoMatches(todo) || hasMatchingChild(todo)
    return matchesDate && matchesSearch
  })
  return [...candidates].sort(todoSort)
})
const totals = computed(() =>
  activity.value.reduce(
    (sum, x) => ({
      notes: sum.notes + x.notes,
      clips: sum.clips + x.clips,
      todos: sum.todos + x.todos,
      completed: sum.completed + x.completed,
    }),
    { notes: 0, clips: 0, todos: 0, completed: 0 },
  ),
)
const editingTodo = computed(() => (todoForm.parent_id ? null : editingTodoRef.value))
const editingTodoRef = ref<Todo | null>(null)

async function run<T>(job: () => Promise<T>, success = '') {
  loading.value = true
  try {
    const result = await job()
    if (success) notify(success)
    return result
  } catch (error) {
    notify(String(error).replace(/^Error: /, ''))
    throw error
  } finally {
    loading.value = false
  }
}
function notify(text: string) {
  message.value = text
  toast.value = text
  window.setTimeout(() => {
    if (toast.value === text) toast.value = ''
  }, 2200)
}
async function refresh() {
  await run(async () => {
    ;[notes.value, clips.value, todos.value, activity.value, trend.value, summary.value] = await Promise.all([
      api.notes.list(),
      api.clipboard.list(),
      api.todos.list(),
      api.stats.heatmap(),
      api.stats.trend(),
      api.stats.summary(),
    ])
    Object.assign(settings, await api.settings.get())
    await loadDayDetails(selectedStatsDate.value)
  })
}
function scheduleDraft() {
  window.clearTimeout(draftTimer)
  draftTimer = window.setTimeout(() => {
    if (noteDraft.value.trim())
      void run(() =>
        api.notes.save({ id: 'draft-main', content: noteDraft.value, tags: parseTags(noteTags.value), draft: true }),
      )
  }, 500)
}
async function archiveNote() {
  if (!noteDraft.value.trim()) return
  await run(async () => {
    const note = await api.notes.save({
      id: editingNoteId.value,
      content: noteDraft.value,
      tags: parseTags(noteTags.value),
      draft: false,
    })
    const index = notes.value.findIndex((x) => x.id === note.id)
    if (index >= 0) notes.value[index] = note
    else notes.value.unshift(note)
    noteDraft.value = ''
    noteTags.value = ''
    editingNoteId.value = undefined
    await api.notes.remove('draft-main').catch(() => undefined)
  }, '念头已归档')
}
function newNote() {
  noteDraft.value = ''
  noteTags.value = ''
  editingNoteId.value = undefined
}
function editNote(note: Note) {
  view.value = 'notes'
  noteDraft.value = note.content
  noteTags.value = note.tags.join(',')
  editingNoteId.value = note.id
}
async function toggleNotePin(note: Note) {
  const updated = await run(() => api.notes.pin(note.id, !note.pinned), note.pinned ? '笔记已取消置顶' : '笔记已置顶')
  Object.assign(note, updated)
}
const editingNoteId = ref<string | undefined>()
async function removeNote(note: Note) {
  if (!window.confirm('确认删除该笔记？')) return
  await run(async () => {
    await api.notes.remove(note.id)
    notes.value = notes.value.filter((x) => x.id !== note.id)
  }, '笔记已删除')
}
async function captureClipboard() {
  const clip = await run(() => api.clipboard.capture(), '已捕获当前剪贴板')
  if (!clip) {
    notify('当前剪贴板没有可读取的文本')
    return
  }
  clips.value = [clip, ...clips.value.filter((x) => x.id !== clip.id)]
}
async function pasteClip(clip: ClipboardEntry) {
  await run(() => api.clipboard.write(clip.id), '已复制到系统剪贴板')
}
async function toggleClipPin(clip: ClipboardEntry) {
  await run(() => api.clipboard.pin(clip.id, !clip.pinned))
  clip.pinned = !clip.pinned
}
function editClip(clip: ClipboardEntry) {
  clipEditor.value = { ...clip }
}
async function saveClipEdit() {
  if (!clipEditor.value) return
  await run(async () => {
    const updated = await api.clipboard.update(clipEditor.value!.id, clipEditor.value!.content)
    const current = clips.value.find((x) => x.id === updated.id)
    if (current) Object.assign(current, updated)
    clipEditor.value = null
  }, '剪贴板条目已更新')
}
async function removeClip(clip: ClipboardEntry) {
  if (!window.confirm('确认删除该剪贴板条目？')) return
  await api.clipboard.remove(clip.id)
  clips.value = clips.value.filter((x) => x.id !== clip.id)
}
function openTodoEditor(todo?: Todo, parentId: string | null = null) {
  editingTodoRef.value = todo || null
  Object.assign(
    todoForm,
    todo
      ? {
          content: todo.content,
          due_at: toLocalInput(new Date(todo.due_at)),
          remind_at: todo.remind_at ? toLocalInput(new Date(todo.remind_at)) : '',
          repeat_rule: todo.repeat_rule || '',
          priority: todo.priority,
          remark: todo.remark,
          tagsText: todo.tags.join(','),
          parent_id: todo.parent_id,
        }
      : {
          content: '',
          due_at: toLocalInput(new Date(Date.now() + 3600000)),
          remind_at: '',
          repeat_rule: '',
          priority: 'medium',
          remark: '',
          tagsText: '',
          parent_id: parentId,
        },
  )
  todoEditor.value = true
}
async function saveTodo() {
  const tags = parseTags(todoForm.tagsText)
  if (tags.length > 3 || tags.some((tag) => tag.length > 10)) {
    notify('待办最多 3 个标签，每个标签不超过 10 个字')
    return
  }
  if (todoForm.remark.length > 200) {
    notify('待办备注最多 200 个字')
    return
  }
  const input = {
    id: editingTodoRef.value?.id,
    content: todoForm.content,
    due_at: new Date(todoForm.due_at).toISOString(),
    remind_at: todoForm.remind_at ? new Date(todoForm.remind_at).toISOString() : null,
    priority: todoForm.priority,
    remark: todoForm.remark,
    tags,
    parent_id: todoForm.parent_id,
    repeat_rule: todoForm.repeat_rule || null,
  }
  await run(async () => {
    const todo = await api.todos.save(input)
    const index = todos.value.findIndex((x) => x.id === todo.id)
    if (index >= 0) todos.value[index] = todo
    else todos.value.push(todo)
    todoEditor.value = false
  }, '待办已保存')
}
async function toggleTodo(todo: Todo) {
  if (todo.status === 'done') {
    notify('已完成待办不可取消完成')
    return
  }
  const updated = await run(() => api.todos.complete(todo.id, true))
  todos.value = [...todos.value.filter((item) => !updated.some((changed) => changed.id === item.id)), ...updated]
  notify('待办已完成')
}
function addChild(todo: Todo) {
  if (childrenOf(todo.id).length >= 5) {
    notify('一个待办最多只能有 5 个子任务')
    return
  }
  openTodoEditor(undefined, todo.id)
}
function openDueEditor(todo: Todo) {
  if (todo.status === 'done') {
    notify('已完成待办不可修改完成时间')
    return
  }
  dueEditor.value = todo
  dueForm.value = toLocalInput(new Date(todo.due_at))
}
async function saveDueEdit() {
  const todo = dueEditor.value
  if (!todo || !dueForm.value) return
  const updated = await run(() => api.todos.due(todo.id, new Date(dueForm.value).toISOString()), '完成时间已更新')
  const index = todos.value.findIndex((item) => item.id === updated.id)
  if (index >= 0) todos.value[index] = updated
  dueEditor.value = null
}
function changePriority(todo: Todo) {
  if (todo.status === 'done') {
    notify('已完成待办不可变更优先级')
    return
  }
  activePriority.value = activePriority.value === todo.id ? null : todo.id
}
async function selectPriority(todo: Todo, priority: Priority) {
  if (priority === todo.priority) {
    activePriority.value = null
    return
  }
  await run(() => api.todos.priority(todo.id, priority), '优先级已更新')
  todo.priority = priority
  activePriority.value = null
}
async function removeTodo(todo: Todo) {
  if (!window.confirm('确认删除该待办？')) return
  await api.todos.remove(todo.id)
  todos.value = todos.value.filter((x) => x.id !== todo.id && x.parent_id !== todo.id)
}
async function saveSettings() {
  await run(() => api.settings.save({ ...settings }), '设置已保存')
  if (settings.shortcut) {
    await run(async () => {
      settings.shortcut = await api.shortcut.rebind(settings.shortcut)
    }, '快捷键已更新').catch(() => undefined)
  }
}
function todoSort(left: Todo, right: Todo): number {
  const leftOverdue = isOverdue(left) || childTodos(left.id).some(isOverdue)
  const rightOverdue = isOverdue(right) || childTodos(right.id).some(isOverdue)
  if (leftOverdue !== rightOverdue) return leftOverdue ? -1 : 1
  if (left.status !== right.status) return left.status === 'open' ? -1 : 1
  const dueDiff = new Date(left.due_at).getTime() - new Date(right.due_at).getTime()
  if (dueDiff !== 0) return dueDiff
  const priorityOrder: Record<Priority, number> = { high: 0, medium: 1, low: 2 }
  const priorityDiff = priorityOrder[left.priority] - priorityOrder[right.priority]
  return priorityDiff || left.created_at.localeCompare(right.created_at)
}
function childrenOf(id: string): Todo[] {
  return childTodos(id).sort(todoSort)
}
function parseTags(input: string) {
  return [
    ...new Set(
      input
        .split(/[,，]/)
        .map((item) => item.trim())
        .filter(Boolean),
    ),
  ]
}
function priorityLabel(priority: Priority) {
  return ({ high: '高', medium: '中', low: '低' } as Record<Priority, string>)[priority]
}
function isOverdue(todo: Todo) {
  return todo.status === 'open' && new Date(todo.due_at).getTime() < Date.now()
}
async function pollClipboard() {
  const latest = await api.clipboard.list().catch(() => [])
  clips.value = latest
}
async function loadDayDetails(date: string) {
  selectedStatsDate.value = date
  dayDetails.value = await api.stats.day(date).catch(() => [])
}
async function exportSelected(format: string) {
  const refs = [
    ...notes.value.map((item) => `note:${item.id}`),
    ...todos.value.map((item) => `todo:${item.id}`),
    ...clips.value.map((item) => `clip:${item.id}`),
  ]
  if (!refs.length) {
    notify('当前没有可导出的数据')
    return
  }
  const path = await run(() => api.exportItems(refs, format), `已导出为 ${format.toUpperCase()}`)
  notify(`导出文件：${path}`)
}
function heatLevel(day: ActivityDay) {
  const total = day.notes + day.clips + day.todos
  return `level-${total === 0 ? 0 : total < 3 ? 1 : total < 7 ? 2 : 3}`
}
function switchCapture(mode: CaptureMode) {
  captureMode.value = mode
  view.value = mode === 'note' ? 'notes' : mode === 'clipboard' ? 'clips' : 'todos'
}
watch(
  () => settings.theme,
  (value) => (document.documentElement.dataset.theme = value),
  { immediate: true },
)
onMounted(() => {
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') activePriority.value = null
  })
  document.addEventListener('click', () => {
    activePriority.value = null
  })
  void listen<string>('inkling://navigate', (event) => {
    if (
      event.payload === 'notes' ||
      event.payload === 'clips' ||
      event.payload === 'todos' ||
      event.payload === 'stats' ||
      event.payload === 'settings'
    )
      view.value = event.payload
  }).catch(() => undefined)
  void listen('inkling://notes-changed', () => void refresh()).catch(() => undefined)
  void listen('inkling://clipboard-changed', () => void refresh()).catch(() => undefined)
  void listen('inkling://todos-changed', () => void refresh()).catch(() => undefined)
  void listen<string>('inkling://reminder-fired', () => notify('待办提醒已触发')).catch(() => undefined)
  void refresh().catch(() => notify('当前需要在 Tauri 应用中运行才能访问本地数据库'))
  clipboardTimer = window.setInterval(() => {
    void pollClipboard()
  }, 1500)
})
onUnmounted(() => {
  if (clipboardTimer) window.clearInterval(clipboardTimer)
})
</script>
