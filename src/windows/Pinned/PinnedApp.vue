<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import { useSettings } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Note, Todo } from '@/typings/domain'
import { renderMarkdown } from '@/utils/format'

/**
 * 桌面置顶浮窗（需求 2.5，原型 #pinnedWindow；spec 4B D31）。
 *
 * 每个置顶项是一个独立窗口，label 形如 `pinned-{kind}-{id}`，前端据此解析自己该显示哪条内容。
 * - 透明度调节作用于整窗；
 * - 双击正文进入编辑态：窗口由 Rust 放大（pinSetEditing），正文换成 .pinned-editor 文本域并聚焦置末；
 *   Ctrl+Enter / 失焦保存（按 kind 走 notes.save / todos.save / clipboard.update），toast「已同步回数据库 ✔」；Esc 放弃；
 * - 笔记回写先 notes.get 取原字段（标签 / 编辑模式 / 导图数据），否则会被清空（spec §8）；
 *   待办回写保留其余字段，已完成待办后端会拒绝改内容，错误原样 toast。
 */
applyCachedTheme()
applyCachedGlass()

const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
const { toast } = useToast()
watch(() => settings.value.theme, applyTheme, { immediate: true })
// 玻璃质感与主题同源：后端设置变化时一并同步。
watch(() => settings.value.glass_level, applyGlass, { immediate: true })

const label = getCurrentWindow().label
/** 从窗口 label 解析出 kind 与 id：pinned-note-xxxx。 */
const parsed = computed(() => {
  const match = /^pinned-(note|todo|clip)-(.+)$/.exec(label)
  return match ? { kind: match[1] as 'note' | 'todo' | 'clip', id: match[2] } : null
})

const content = ref('')
const opacity = ref(100)
const editing = ref(false)
const draft = ref('')
const editor = ref<HTMLTextAreaElement | null>(null)
/** 目标已被删除时不允许进入编辑。 */
const missing = ref(false)

const html = computed(() => renderMarkdown(content.value))

/** 按 kind 拉取对应内容。 */
async function load(): Promise<void> {
  const target = parsed.value
  if (!target) {
    logger.error('pinned', `无法解析窗口 label: ${label}`)
    return
  }

  try {
    let found: string | undefined
    if (target.kind === 'note') {
      found = (await api.notes.get(target.id))?.content
    } else if (target.kind === 'todo') {
      const todos = await api.todos.list()
      found = todos.find((t) => t.id === target.id)?.content
    } else {
      const clips = await api.clipboard.list()
      found = clips.find((c) => c.id === target.id)?.content
    }
    missing.value = found === undefined
    content.value =
      found ??
      (target.kind === 'note'
        ? '（该笔记已被删除）'
        : target.kind === 'todo'
          ? '（该待办已被删除）'
          : '（该条目已被删除）')
    logger.info('pinned', `已加载置顶内容 ${label}`)
  } catch (error) {
    logger.error('pinned', '加载置顶内容失败', error)
  }
}

/** 透明度实时作用于整个窗口。 */
watch(opacity, (value) => {
  document.documentElement.style.opacity = String(value / 100)
})

async function close(): Promise<void> {
  try {
    await api.windows.pinClose(label)
  } catch (error) {
    logger.error('pinned', '关闭置顶窗失败', error)
  }
}

/** 双击进入编辑态：窗口放大（Rust）、文本域聚焦且光标置末（原型 prompt 的替代）。 */
async function beginEdit(): Promise<void> {
  if (editing.value || missing.value) return
  draft.value = content.value
  editing.value = true
  logger.info('pinned', `进入编辑态 ${label}`)
  try {
    await api.windows.pinSetEditing(label, true)
  } catch (error) {
    logger.error('pinned', '切换编辑态失败', error)
  }
  await nextTick()
  const el = editor.value
  if (el) {
    el.focus()
    el.setSelectionRange(el.value.length, el.value.length)
  }
}

/** 退出编辑态并还原窗口尺寸。 */
async function endEdit(): Promise<void> {
  editing.value = false
  try {
    await api.windows.pinSetEditing(label, false)
  } catch (error) {
    logger.error('pinned', '还原窗口尺寸失败', error)
  }
}

/** 按 kind 回写：笔记先取原字段再保存；待办保留其余字段；剪贴板直接更新。 */
async function writeBack(kind: 'note' | 'todo' | 'clip', id: string, next: string): Promise<void> {
  if (kind === 'note') {
    const note: Note | null = await api.notes.get(id)
    if (!note) throw new Error('该笔记已被删除')
    await api.notes.save({
      id,
      content: next,
      tags: note.tags,
      editorMode: note.editor_mode,
      mindmapData: note.mindmap_data,
      draft: false,
    })
    return
  }
  if (kind === 'todo') {
    const todo: Todo | undefined = (await api.todos.list()).find((t) => t.id === id)
    if (!todo) throw new Error('该待办已被删除')
    await api.todos.save({
      id,
      content: next,
      dueAt: todo.due_at,
      remindOffsetMinutes: todo.remind_offset_minutes,
      remindDesktop: todo.remind_desktop,
      remindEmail: todo.remind_email,
      repeatRule: todo.repeat_rule,
      priority: todo.priority,
      remark: todo.remark,
      tags: todo.tags,
      parentId: todo.parent_id,
      allowPast: true,
    })
    return
  }
  await api.clipboard.update(id, next)
}

/** 防重入：失焦与 Ctrl+Enter 可能连续触发 save，保存进行中直接忽略。 */
let saving = false

/** Ctrl+Enter / 失焦保存：空内容或未改动只退出编辑态，不写库。 */
async function save(): Promise<void> {
  if (!editing.value || saving) return
  const target = parsed.value
  const next = draft.value
  if (!target || !next.trim() || next === content.value) {
    if (!next.trim()) toast('内容为空，未保存')
    await endEdit()
    return
  }
  saving = true
  logger.info('pinned', `回写 ${target.kind} ${target.id}，长度 ${next.length}`)
  try {
    await writeBack(target.kind, target.id, next)
    content.value = next
    toast('已同步回数据库 ✔')
    await endEdit()
  } catch (error) {
    logger.error('pinned', '回写失败', error)
    toast(String(error))
    // 保存失败保留编辑态让用户改，但失焦触发的保存不应反复弹 toast：重新聚焦。
    editor.value?.focus()
  } finally {
    saving = false
  }
}

/** Esc 放弃：丢弃草稿、还原尺寸。 */
async function cancelEdit(): Promise<void> {
  if (!editing.value) return
  logger.info('pinned', '放弃编辑')
  draft.value = content.value
  await endEdit()
}

onMounted(load)
</script>

<template>
  <div id="pinnedWindow" class="glass">
    <!-- 仅标题行可拖拽：drag 区域会被子元素继承，放在根元素上会让整窗按钮全部点不动 -->
    <div class="pinned-header" data-tauri-drag-region>
      <span><span class="ix">📌</span> 置顶</span>
      <span class="pinned-close no-drag" title="关闭" @click="close">✕</span>
    </div>

    <textarea
      v-if="editing"
      ref="editor"
      v-model="draft"
      class="pinned-editor no-drag"
      spellcheck="false"
      placeholder="编辑内容…（Ctrl+Enter 保存 · Esc 放弃）"
      @keydown.ctrl.enter.prevent="save"
      @keydown.esc.prevent="cancelEdit"
      @blur="save"
    />
    <div v-else class="pinned-body no-drag" title="双击编辑" @dblclick="beginEdit" v-html="html" />

    <div class="pinned-footer no-drag">
      <span>透明度</span>
      <input v-model.number="opacity" type="range" min="30" max="100" />
    </div>

    <ToastHost />
  </div>
</template>
