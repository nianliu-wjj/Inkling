<template>
  <div class="app-shell" :data-theme="settings.theme">
    <header class="titlebar">
      <div class="brand"><span class="brand-mark">✒</span><span>Inkling</span><small>念头捕手</small></div>
      <div class="mode-switcher" role="tablist" aria-label="快速捕获模式">
        <button v-for="item in captureModes" :key="item.key" :class="['mode-dot', item.key, { active: captureMode === item.key }]" @click="switchCapture(item.key)" :title="item.label">{{ item.icon }}</button>
      </div>
      <div class="titlebar-actions"><span class="save-indicator">{{ loading ? '同步中…' : message || '本地已保存' }}</span><button class="icon-button" title="收起面板" @click="captureMode = 'note'">⌃</button></div>
    </header>

    <main class="workspace">
      <aside class="sidebar">
        <button v-for="item in navItems" :key="item.key" :class="['nav-item', item.key, { active: view === item.key }]" @click="view = item.key"><span>{{ item.icon }}</span><span>{{ item.label }}</span><b>{{ item.count }}</b></button>
        <div class="sidebar-spacer" />
        <div class="quick-capture"><span>快捷捕获</span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>Space</kbd></div>
        <button class="nav-item settings" :class="{ active: view === 'settings' }" @click="view = 'settings'"><span>⚙</span><span>偏好设置</span></button>
      </aside>

      <section class="content">
        <div v-if="view === 'notes'" class="view-page">
          <section class="capture-card note-capture"><div class="section-title"><div><span class="eyebrow red">RED · NOTE</span><h1>此刻在想什么？</h1></div><button class="link-button" @click="newNote">清空</button></div><textarea v-model="noteDraft" placeholder="写下一个念头，支持 Markdown…" @input="scheduleDraft"></textarea><div class="capture-footer"><input v-model="noteTags" placeholder="标签，以逗号分隔" /><button class="primary-button" :disabled="!noteDraft.trim()" @click="archiveNote">归档念头 ↵</button></div></section>
          <div class="page-heading"><div><span class="eyebrow">ARCHIVE</span><h2>最近归档</h2></div><input v-model="search" class="search" placeholder="搜索笔记…" /></div>
          <div class="card-grid"><article v-for="note in filteredNotes" :key="note.id" class="data-card note-card"><div class="card-actions"><button @click="removeNote(note)" title="删除">×</button></div><div class="card-content markdown" v-html="renderMarkdown(note.content)"></div><div class="card-meta"><time>{{ formatTime(note.updated_at) }}</time><span v-for="tag in note.tags.slice(0, 3)" :key="tag" class="tag">#{{ tag }}</span><span v-if="note.tags.length > 3" class="tag">+{{ note.tags.length - 3 }}</span><button class="mini-action" @click="editNote(note)">编辑</button></div></article><div v-if="!filteredNotes.length" class="empty">还没有归档笔记，先捕获一个念头吧。</div></div>
        </div>

        <div v-else-if="view === 'clips'" class="view-page"><div class="page-heading"><div><span class="eyebrow yellow">YELLOW · CLIPBOARD</span><h2>剪贴板历史</h2></div><div class="heading-actions"><input v-model="search" class="search" placeholder="搜索剪贴板…" /><button class="secondary-button" @click="captureClipboard">捕获当前剪贴板</button></div></div><div class="stack-list"><article v-for="clip in filteredClips" :key="clip.id" class="data-card clip-card" @dblclick="pasteClip(clip)"><div class="clip-type">{{ clip.content_type === 'link' ? '↗' : clip.content_type === 'code' ? '</>' : '▤' }}</div><div class="clip-body"><div class="clip-preview">{{ clip.preview }}</div><time>{{ formatTime(clip.modified_at) }}</time></div><div class="card-actions visible"><button :class="{ pinned: clip.pinned }" @click.stop="toggleClipPin(clip)">{{ clip.pinned ? '★' : '☆' }}</button><button @click.stop="editClip(clip)">编辑</button><button @click.stop="removeClip(clip)">×</button></div></article><div v-if="!filteredClips.length" class="empty">剪贴板暂无记录。</div></div></div>

        <div v-else-if="view === 'todos'" class="view-page"><div class="page-heading"><div><span class="eyebrow green">GREEN · TODO</span><h2>待办事项</h2></div><button class="primary-button" @click="openTodoEditor()">＋ 新建待办</button></div><div class="todo-list"><article v-for="todo in rootTodos" :key="todo.id" :class="['data-card', 'todo-card', { done: todo.status === 'done', overdue: isOverdue(todo) }]" @click="activeTodo = activeTodo === todo.id ? null : todo.id"><div class="check" :class="{ checked: todo.status === 'done' }" @click.stop="toggleTodo(todo)">{{ todo.status === 'done' ? '✓' : '' }}</div><div class="todo-body"><div class="todo-title"><span>{{ todo.content }}</span><span :class="['priority', todo.priority]" @click.stop="changePriority(todo)">{{ priorityLabel(todo.priority) }}</span><div v-if="activePriority === todo.id" class="priority-popover" @click.stop><button v-for="option in priorityOptions" :key="option.value" :class="{ selected: todo.priority === option.value }" @click="selectPriority(todo, option.value)">{{ option.label }}<b v-if="todo.priority === option.value">✓</b></button></div></div><div class="todo-meta"><span>{{ formatDue(todo.due_at) }}</span><span v-if="isOverdue(todo)" class="overdue-text">逾期</span><span v-if="todo.remind_at">⏰ {{ formatTime(todo.remind_at) }}</span><span v-if="todo.tags.length">#{{ todo.tags.join(' #') }}</span></div><div v-if="todo.remark" class="remark">▣ {{ todo.remark }}</div><div v-if="childrenOf(todo.id).length" class="children"><div v-for="child in childrenOf(todo.id)" :key="child.id" class="child-row"><span class="child-check" :class="{ checked: child.status === 'done' }" @click.stop="toggleTodo(child)">{{ child.status === 'done' ? '✓' : '' }}</span><span :class="{ strike: child.status === 'done' }">{{ child.content }}</span><span :class="['priority', child.priority]" @click.stop="changePriority(child)">{{ priorityLabel(child.priority) }}</span><div v-if="activePriority === child.id" class="priority-popover child-popover" @click.stop><button v-for="option in priorityOptions" :key="option.value" :class="{ selected: child.priority === option.value }" @click="selectPriority(child, option.value)">{{ option.label }}<b v-if="child.priority === option.value">✓</b></button></div></div></div></div><div class="card-actions visible"><button v-if="todo.status === 'done'" @click.stop="addChild(todo)">＋ 子任务</button><button v-else @click.stop="openTodoEditor(todo)">编辑</button><button v-if="todo.status !== 'done'" @click.stop="removeTodo(todo)">×</button></div></article><div v-if="!rootTodos.length" class="empty">暂无待办。</div></div></div>

        <div v-else-if="view === 'stats'" class="view-page"><div class="page-heading"><div><span class="eyebrow blue">ACTIVITY</span><h2>使用统计</h2></div></div><div class="stats-summary"><div><b>{{ totals.notes }}</b><span>笔记归档</span></div><div><b>{{ totals.clips }}</b><span>剪贴板捕获</span></div><div><b>{{ totals.todos }}</b><span>待办创建</span></div><div><b>{{ totals.completed }}</b><span>待办完成</span></div></div><section class="data-card heatmap-card"><h3>近 180 天活跃度</h3><div class="heatmap"><div v-for="day in activity" :key="day.date" :class="['heat-cell', heatLevel(day), { overdue: day.overdue > 0 }]" :title="`${day.date}：笔记 ${day.notes}，剪贴板 ${day.clips}，待办 ${day.todos}，完成 ${day.completed}，逾期 ${day.overdue}`"></div></div><div class="legend"><span>少</span><i class="heat-cell level-0"/><i class="heat-cell level-1"/><i class="heat-cell level-2"/><i class="heat-cell level-3"/><span>多</span><em>红框表示存在逾期</em></div></section></div>

        <div v-else class="view-page"><div class="page-heading"><div><span class="eyebrow">SETTINGS</span><h2>偏好设置</h2></div></div><section class="settings-card data-card"><label>主题<select v-model="settings.theme" @change="saveSettings"><option v-for="theme in themes" :key="theme.key" :value="theme.key">{{ theme.label }}</option></select></label><label>失焦收起<select v-model="settings.collapse_policy" @change="saveSettings"><option value="immediate">立即收起</option><option value="3s">延迟 3 秒</option><option value="never">固定不收起</option></select></label><label>剪贴板保留天数<input v-model.number="settings.clipboard_retention_days" type="number" min="1" max="365" @change="saveSettings" /></label><label>备注展示样式<select v-model="settings.remark_style" @change="saveSettings"><option value="mixed">混合模式</option><option value="icon">图标徽章 + 悬浮</option><option value="text">置灰文本行</option></select></label><label>全局快捷键<input v-model="settings.shortcut" @change="saveSettings" /></label><label class="switch-line"><span>开机静默自启动</span><input v-model="settings.start_on_boot" type="checkbox" @change="saveSettings" /></label></section></div>
      </section>
    </main>

    <div v-if="todoEditor" class="modal-backdrop" @click.self="todoEditor = false"><form class="modal" @submit.prevent="saveTodo"><div class="modal-head"><h3>{{ editingTodoRef ? '编辑待办' : '新建待办' }}</h3><button type="button" @click="todoEditor = false">×</button></div><label>内容<input v-model="todoForm.content" autofocus required /></label><div class="form-grid"><label>计划完成时间<input v-model="todoForm.due_at" type="datetime-local" required /></label><label>优先级<select v-model="todoForm.priority"><option value="high">高</option><option value="medium">中</option><option value="low">低</option></select></label></div><label>提醒时间<input v-model="todoForm.remind_at" type="datetime-local" /></label><label>标签<input v-model="todoForm.tagsText" placeholder="以逗号分隔" /></label><label>备注<textarea v-model="todoForm.remark" rows="3" /></label><div class="modal-actions"><button type="button" class="secondary-button" @click="todoEditor = false">取消</button><button class="primary-button">保存</button></div></form></div>
    <div v-if="clipEditor" class="modal-backdrop" @click.self="clipEditor = null"><form class="modal" @submit.prevent="saveClipEdit"><div class="modal-head"><h3>编辑剪贴板条目</h3><button type="button" @click="clipEditor = null">×</button></div><textarea v-model="clipEditor.content" rows="10" autofocus /><div class="modal-actions"><button type="button" class="secondary-button" @click="clipEditor = null">取消</button><button class="primary-button">保存</button></div></form></div>
    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { api } from './api'
import type { ActivityDay, ClipboardEntry, Note, Priority, Settings, Todo, View } from './types'

const captureModes = [{ key: 'note' as const, icon: '🔴', label: '笔记' }, { key: 'clipboard' as const, icon: '🟡', label: '剪贴板' }, { key: 'todo' as const, icon: '🟢', label: '待办' }]
const captureMode = ref<'note' | 'clipboard' | 'todo'>('note')
const view = ref<View>('notes'); const loading = ref(false); const message = ref(''); const toast = ref(''); const search = ref('')
const notes = ref<Note[]>([]); const clips = ref<ClipboardEntry[]>([]); const todos = ref<Todo[]>([]); const activity = ref<ActivityDay[]>([])
const noteDraft = ref(''); const noteTags = ref(''); let draftTimer: number | undefined
const todoEditor = ref(false); const activeTodo = ref<string | null>(null); const activePriority = ref<string | null>(null); const clipEditor = ref<ClipboardEntry | null>(null)
const settings = reactive<Settings>({ collapse_policy: '3s', clipboard_retention_days: 30, start_on_boot: false, shortcut: 'Ctrl+Shift+Space', remark_style: 'mixed', theme: 'dark' })
const todoForm = reactive({ content: '', due_at: toLocalInput(new Date(Date.now() + 3600000)), remind_at: '', priority: 'medium' as Priority, remark: '', tagsText: '', parent_id: null as string | null })
const priorityOptions = [{ value: 'high' as Priority, label: '高优先级' }, { value: 'medium' as Priority, label: '中优先级' }, { value: 'low' as Priority, label: '低优先级' }]
const themes = [{ key: 'dark', label: '深色' }, { key: 'light', label: '浅色' }, { key: 'copper', label: '焦糖拿铁' }, { key: 'aurora', label: '北极光' }]
const navItems = computed(() => [{ key: 'notes' as View, icon: '📝', label: '笔记', count: notes.value.length }, { key: 'clips' as View, icon: '📋', label: '剪贴板', count: clips.value.length }, { key: 'todos' as View, icon: '✅', label: '待办', count: todos.value.filter(x => x.status === 'open').length }, { key: 'stats' as View, icon: '📊', label: '统计', count: 0 }])
const filteredNotes = computed(() => notes.value.filter(x => !search.value || x.content.toLowerCase().includes(search.value.toLowerCase()) || x.tags.join(' ').toLowerCase().includes(search.value.toLowerCase())))
const filteredClips = computed(() => clips.value.filter(x => !search.value || x.content.toLowerCase().includes(search.value.toLowerCase())))
const rootTodos = computed(() => todos.value.filter(x => !x.parent_id))
const totals = computed(() => activity.value.reduce((sum, x) => ({ notes: sum.notes + x.notes, clips: sum.clips + x.clips, todos: sum.todos + x.todos, completed: sum.completed + x.completed }), { notes: 0, clips: 0, todos: 0, completed: 0 }))
const editingTodo = computed(() => todoForm.parent_id ? null : editingTodoRef.value)
const editingTodoRef = ref<Todo | null>(null)

async function run<T>(job: () => Promise<T>, success = '') { loading.value = true; try { const result = await job(); if (success) notify(success); return result } catch (error) { notify(String(error).replace(/^Error: /, '')); throw error } finally { loading.value = false } }
function notify(text: string) { message.value = text; toast.value = text; window.setTimeout(() => { if (toast.value === text) toast.value = '' }, 2200) }
async function refresh() { await run(async () => { [notes.value, clips.value, todos.value, activity.value] = await Promise.all([api.notes.list(), api.clipboard.list(), api.todos.list(), api.activity()]); Object.assign(settings, await api.settings.get()) }) }
function scheduleDraft() { window.clearTimeout(draftTimer); draftTimer = window.setTimeout(() => { if (noteDraft.value.trim()) void run(() => api.notes.save({ id: 'draft-main', content: noteDraft.value, tags: parseTags(noteTags.value), draft: true })) }, 500) }
async function archiveNote() { if (!noteDraft.value.trim()) return; await run(async () => { const note = await api.notes.save({ id: editingNoteId.value, content: noteDraft.value, tags: parseTags(noteTags.value), draft: false }); const index = notes.value.findIndex(x => x.id === note.id); if (index >= 0) notes.value[index] = note; else notes.value.unshift(note); noteDraft.value = ''; noteTags.value = ''; editingNoteId.value = undefined; await api.notes.remove('draft-main').catch(() => undefined) }, '念头已归档') }
function newNote() { noteDraft.value = ''; noteTags.value = ''; editingNoteId.value = undefined }
function editNote(note: Note) { view.value = 'notes'; noteDraft.value = note.content; noteTags.value = note.tags.join(','); editingNoteId.value = note.id }
const editingNoteId = ref<string | undefined>()
async function removeNote(note: Note) { if (!window.confirm('确认删除该笔记？')) return; await run(async () => { await api.notes.remove(note.id); notes.value = notes.value.filter(x => x.id !== note.id) }, '笔记已删除') }
async function captureClipboard() { const content = await navigator.clipboard?.readText().catch(() => '') || ''; if (!content) { notify('当前剪贴板没有可读取的文本'); return } const clip = await run(() => api.clipboard.capture(content), '已捕获当前剪贴板'); clips.value = [clip, ...clips.value.filter(x => x.id !== clip.id)] }
async function pasteClip(clip: ClipboardEntry) { await navigator.clipboard?.writeText(clip.content); await api.clipboard.pin(clip.id, true); clip.pinned = true; notify('已复制并置顶') }
async function toggleClipPin(clip: ClipboardEntry) { await run(() => api.clipboard.pin(clip.id, !clip.pinned)); clip.pinned = !clip.pinned }
function editClip(clip: ClipboardEntry) { clipEditor.value = { ...clip } }
async function saveClipEdit() { if (!clipEditor.value) return; await run(async () => { await api.clipboard.update(clipEditor.value!.id, clipEditor.value!.content); const current = clips.value.find(x => x.id === clipEditor.value!.id); if (current) Object.assign(current, clipEditor.value, { preview: clipEditor.value!.content.slice(0, 240), modified_at: new Date().toISOString() }); clipEditor.value = null }, '剪贴板条目已更新') }
async function removeClip(clip: ClipboardEntry) { if (!window.confirm('确认删除该剪贴板条目？')) return; await api.clipboard.remove(clip.id); clips.value = clips.value.filter(x => x.id !== clip.id) }
function openTodoEditor(todo?: Todo) { editingTodoRef.value = todo || null; Object.assign(todoForm, todo ? { content: todo.content, due_at: toLocalInput(new Date(todo.due_at)), remind_at: todo.remind_at ? toLocalInput(new Date(todo.remind_at)) : '', priority: todo.priority, remark: todo.remark, tagsText: todo.tags.join(','), parent_id: todo.parent_id } : { content: '', due_at: toLocalInput(new Date(Date.now() + 3600000)), remind_at: '', priority: 'medium', remark: '', tagsText: '', parent_id: null }); todoEditor.value = true }
async function saveTodo() { const input = { id: editingTodoRef.value?.id, content: todoForm.content, due_at: new Date(todoForm.due_at).toISOString(), remind_at: todoForm.remind_at ? new Date(todoForm.remind_at).toISOString() : null, priority: todoForm.priority, remark: todoForm.remark, tags: parseTags(todoForm.tagsText), parent_id: todoForm.parent_id }; await run(async () => { const todo = await api.todos.save(input); const index = todos.value.findIndex(x => x.id === todo.id); if (index >= 0) todos.value[index] = todo; else todos.value.push(todo); todoEditor.value = false }, '待办已保存') }
async function toggleTodo(todo: Todo) { if (todo.status === 'done') { notify('已完成待办不可取消完成'); return } const updated = await run(() => api.todos.complete(todo.id, true)); todos.value = updated; notify('待办已完成') }
async function addChild(todo: Todo) { const content = window.prompt('请输入子任务内容'); if (!content?.trim()) return; const child = await run(() => api.todos.child(todo.id, content, new Date(Date.now() + 3600000).toISOString()), '子任务已新增'); todos.value.push(child); const parent = todos.value.find(x => x.id === todo.id); if (parent) { parent.status = 'open'; parent.completed_at = null } }
function changePriority(todo: Todo) { if (todo.status === 'done') { notify('已完成待办不可变更优先级'); return } activePriority.value = activePriority.value === todo.id ? null : todo.id }
async function selectPriority(todo: Todo, priority: Priority) { if (priority === todo.priority) { activePriority.value = null; return } await run(() => api.todos.priority(todo.id, priority), '优先级已更新'); todo.priority = priority; activePriority.value = null }
async function removeTodo(todo: Todo) { if (!window.confirm('确认删除该待办？')) return; await api.todos.remove(todo.id); todos.value = todos.value.filter(x => x.id !== todo.id && x.parent_id !== todo.id) }
async function saveSettings() { await run(() => api.settings.save({ ...settings }), '设置已保存') }
function childrenOf(id: string) { return todos.value.filter(x => x.parent_id === id) }
function parseTags(input: string) { return input.split(/[,，]/).map(x => x.trim()).filter(Boolean) }
function priorityLabel(priority: Priority) { return ({ high: '高', medium: '中', low: '低' } as Record<Priority, string>)[priority] }
function isOverdue(todo: Todo) { return todo.status === 'open' && new Date(todo.due_at).getTime() < Date.now() }
function formatTime(value: string) { return new Intl.DateTimeFormat('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(new Date(value)) }
function formatDue(value: string) { return new Intl.DateTimeFormat('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(new Date(value)) }
function toLocalInput(value: Date) { const offset = value.getTimezoneOffset(); return new Date(value.getTime() - offset * 60000).toISOString().slice(0, 16) }
function renderMarkdown(value: string) { return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>').replace(/`(.+?)`/g, '<code>$1</code>').replace(/\n/g, '<br>') }
function heatLevel(day: ActivityDay) { const total = day.notes + day.clips + day.todos; return `level-${total === 0 ? 0 : total < 3 ? 1 : total < 7 ? 2 : 3}` }
function switchCapture(mode: 'note' | 'clipboard' | 'todo') { captureMode.value = mode; view.value = mode === 'note' ? 'notes' : mode === 'clipboard' ? 'clips' : 'todos' }
watch(() => settings.theme, value => document.documentElement.dataset.theme = value, { immediate: true })
onMounted(() => { document.addEventListener('keydown', event => { if (event.key === 'Escape') activePriority.value = null }); document.addEventListener('click', () => { activePriority.value = null }); void refresh().catch(() => notify('当前需要在 Tauri 应用中运行才能访问本地数据库')) })
</script>



