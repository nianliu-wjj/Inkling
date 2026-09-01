<script setup lang="ts">
import { computed, ref } from 'vue'
import TodoTree from '@/components/card/TodoTree.vue'
import TodoEditorModal from '@/components/todo/TodoEditorModal.vue'
import { useSettings, useTodos } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Priority, Todo, TodoInput } from '@/typings/domain'
import { dateKeyOf, todayKey } from '@/utils/datetime'
import { isOverdue } from '@/utils/todo'

/**
 * 面板 · 待办模式。
 *
 * 需求 2.2：面板待办页具备全部能力（优先级/逾期/子任务/标签/备注/重复提醒），
 * 但**仅可查看当日待办，不提供日期切换**；逾期事项自动置顶。
 *
 * 展示口径：今天的事项 + 此前日期仍未完成的逾期事项；未来日期不提前进入。
 */
const emit = defineEmits<{ (e: 'modal', open: boolean): void }>()

const { todos } = useTodos()
const { settings } = useSettings()
const { toast } = useToast()

const keyword = ref('')
const priorityFilter = ref<'all' | Priority>('all')

/** 编辑弹窗状态。 */
const editor = ref<{
  mode: 'create' | 'edit' | 'remind' | 'child'
  todo: Todo | null
  parent: Todo | null
} | null>(null)

/**
 * 当日可见集合：
 * 1. 归属日 = 今天的事项；
 * 2. 早于今天且仍未完成的逾期事项；
 * 3. 上述事项的子任务（无论归属日，随父级一起展示）。
 */
const visible = computed(() => {
  const today = todayKey()

  const roots = todos.value.filter((todo) => {
    if (todo.parent_id) return false
    const key = dateKeyOf(todo.due_at)
    if (key === today) return true
    return key < today && isOverdue(todo)
  })

  const rootIds = new Set(roots.map((t) => t.id))
  const children = todos.value.filter((t) => t.parent_id && rootIds.has(t.parent_id))

  // 父待办有逾期子任务时，父级整棵树也要进入今天的逾期区。
  const extraRoots = todos.value.filter(
    (todo) =>
      !todo.parent_id && !rootIds.has(todo.id) && todos.value.some((c) => c.parent_id === todo.id && isOverdue(c)),
  )
  const extraIds = new Set(extraRoots.map((t) => t.id))
  const extraChildren = todos.value.filter((t) => t.parent_id && extraIds.has(t.parent_id))

  return [...roots, ...extraRoots, ...children, ...extraChildren]
})

/** 搜索与优先级过滤；命中子任务时保留其父级以维持树结构。 */
const filtered = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  const prio = priorityFilter.value

  if (!key && prio === 'all') return visible.value

  const matched = visible.value.filter((todo) => {
    const hitKeyword = !key || todo.content.toLowerCase().includes(key)
    const hitPriority = prio === 'all' || todo.priority === prio
    return hitKeyword && hitPriority
  })

  // 补回命中项的父级，避免子任务脱离树单独悬空。
  const ids = new Set(matched.map((t) => t.id))
  const parents = visible.value.filter(
    (t) => !t.parent_id && matched.some((m) => m.parent_id === t.id) && !ids.has(t.id),
  )
  return [...parents, ...matched]
})

function openEditor(
  mode: 'create' | 'edit' | 'remind' | 'child',
  todo: Todo | null = null,
  parent: Todo | null = null,
): void {
  editor.value = { mode, todo, parent }
  emit('modal', true)
}

function closeEditor(): void {
  editor.value = null
  emit('modal', false)
}

/** 已完成事项一律拦截修改（需求 2.2）。 */
function guardDone(todo: Todo, action: string): boolean {
  if (todo.status === 'done') {
    toast(`已完成事项不可${action}`)
    return true
  }
  return false
}

async function saveTodo(input: TodoInput): Promise<void> {
  try {
    await api.todos.save(input)
    toast('已保存')
  } catch (error) {
    logger.error('panel-todo', '保存待办失败', error)
    toast(String(error))
  } finally {
    closeEditor()
  }
}

async function toggleDone(todo: Todo): Promise<void> {
  // 取消勾选同样被拦截。
  if (guardDone(todo, '取消完成')) return
  try {
    await api.todos.complete(todo.id, true)
  } catch (error) {
    logger.error('panel-todo', '完成待办失败', error)
    toast(String(error))
  }
}

async function changePriority(todo: Todo, priority: Priority): Promise<void> {
  if (guardDone(todo, '修改优先级')) return
  try {
    await api.todos.priority(todo.id, priority)
    toast('优先级已更新')
  } catch (error) {
    logger.error('panel-todo', '修改优先级失败', error)
    toast(String(error))
  }
}

async function removeTodo(todo: Todo): Promise<void> {
  if (guardDone(todo, '删除')) return
  try {
    await api.todos.remove(todo.id)
    toast('已删除')
  } catch (error) {
    logger.error('panel-todo', '删除待办失败', error)
    toast(String(error))
  }
}
</script>

<template>
  <section class="panel-page">
    <div class="todo-input-row">
      <input v-model="keyword" class="search-input" placeholder="🔍 搜索当日待办…" />
      <select v-model="priorityFilter" class="prio-select" title="按优先级过滤">
        <option value="all">全部</option>
        <option value="high">🔴 高</option>
        <option value="medium">🟡 中</option>
        <option value="low">🟢 低</option>
      </select>
      <button type="button" class="btn tiny" title="新增待办事项（可设置完成时间）" @click="openEditor('create')">
        📅
      </button>
    </div>

    <div class="todo-panel-hint">
      搜索 / 优先级过滤当日待办 · 逾期事项自动置顶 · 点击优先级徽章可调整 · 点击 📅 新增待办
    </div>

    <TodoTree
      :todos="filtered"
      :remark-style="settings.remark_style"
      @toggle-done="toggleDone"
      @edit="openEditor('edit', $event)"
      @edit-due="openEditor('edit', $event)"
      @edit-remind="openEditor('remind', $event)"
      @add-sub="openEditor('child', null, $event)"
      @open-tags="openEditor('edit', $event)"
      @priority="changePriority"
      @delete="removeTodo"
    />

    <TodoEditorModal
      v-if="editor"
      :mode="editor.mode"
      :todo="editor.todo"
      :parent="editor.parent"
      @save="saveTodo"
      @close="closeEditor"
    />
  </section>
</template>
