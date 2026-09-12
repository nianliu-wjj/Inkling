<script setup lang="ts">
import { computed, ref } from 'vue'
import TodoTree from '@/components/card/TodoTree.vue'
import RepeatMenu from '@/components/todo/RepeatMenu.vue'
import { useEditorWindow } from '@/composables/useEditorWindow'
import type { TodoEditorMode } from '@/constants/todoEditor'
import { useSettings, useTodos } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Priority, Todo } from '@/typings/domain'
import { dateKeyOf, shiftDateKey, todayKey } from '@/utils/datetime'
import { isOverdue, searchTodos, type TodoNode } from '@/utils/todo'

/**
 * 归档 · 待办页。
 *
 * 需求 2.2「日期视图」：
 * - 默认展示当天，提供 ‹ / › / 今天 切换任意日期；
 * - 选历史日期时只展示归属该日的顶级待办，且该日未完成事项均标记逾期；
 * - 选今天时展示今天事项 + 此前仍未完成的逾期事项；未来日期不提前进入；
 * - 搜索**跨全部日期**（含子任务文本与已完成项），结果显示所属日期。
 * - 新增 / 编辑走 editor 窗内的锚定浮层（spec D23）：锚点为卡片或顶部「＋」按钮，保存后经 todosChanged 刷新。
 */
const { todos } = useTodos()
const { settings } = useSettings()
const { toast } = useToast()
const { openTodoEditor } = useEditorWindow('todos-view')

const currentDate = ref(todayKey())
const keyword = ref('')
const tree = ref<InstanceType<typeof TodoTree> | null>(null)
const repeatMenu = ref<InstanceType<typeof RepeatMenu> | null>(null)
/** 正在改重复规则的待办；菜单选中后据此写库。 */
let repeatTarget: Todo | null = null

const isToday = computed(() => currentDate.value === todayKey())
/** 搜索态：跨日期查询，日期切换条的日期口径不再生效。 */
const searching = computed(() => keyword.value.trim().length > 0)

/** 按当前日期筛选出的可见集合（非搜索态）。 */
const byDate = computed(() => {
  const target = currentDate.value
  const today = todayKey()

  const roots = todos.value.filter((todo) => {
    if (todo.parent_id) return false
    const key = dateKeyOf(todo.due_at)
    if (key === target) return true
    // 只有「今天」这一视图会额外纳入此前未完成的逾期事项。
    return target === today && key < today && isOverdue(todo)
  })

  const rootIds = new Set(roots.map((t) => t.id))

  // 父待办有逾期子任务时，父级整棵树进入今天的逾期区。
  const extraRoots =
    target === today
      ? todos.value.filter(
          (todo) =>
            !todo.parent_id &&
            !rootIds.has(todo.id) &&
            todos.value.some((c) => c.parent_id === todo.id && isOverdue(c)),
        )
      : []
  extraRoots.forEach((t) => rootIds.add(t.id))

  const children = todos.value.filter((t) => t.parent_id && rootIds.has(t.parent_id))
  return [...roots, ...extraRoots, ...children]
})

/** 搜索态：跨全部日期（原型 renderTodoList 搜索分支），含子任务与已完成项。 */
const search = computed(() => searchTodos(todos.value, keyword.value))

/** 树节点摊平回扁平列表：TodoTree 内部会重新组树。 */
function flatten(nodes: readonly TodoNode[]): Todo[] {
  return nodes.flatMap((node) => [node.todo, ...node.children])
}

const visible = computed(() => (searching.value ? flatten(search.value.nodes) : byDate.value))

/** 打开 editor 窗内的锚定浮层；历史日期视图下新增走补录（presetDate）。 */
function openEditor(
  mode: TodoEditorMode,
  todo: Todo | null = null,
  parent: Todo | null = null,
  anchorEl: HTMLElement | null = null,
): void {
  void openTodoEditor({
    mode,
    todoId: todo?.id ?? null,
    parentId: parent?.id ?? null,
    presetDate: mode === 'create' && !isToday.value ? currentDate.value : '',
    anchorEl,
  })
}

/** 卡片级锚点：从待办树反查 [data-id]。 */
function cardOf(todo: Todo): HTMLElement | null {
  return tree.value?.cardOf(todo.id) ?? null
}

function guardDone(todo: Todo, action: string): boolean {
  if (todo.status === 'done') {
    toast(`已完成事项不可${action}`)
    return true
  }
  return false
}

async function toggleDone(todo: Todo): Promise<void> {
  if (todo.status === 'done') {
    if (!window.confirm('确定将已完成事项恢复为未完成吗？')) return
    try {
      await api.todos.complete(todo.id, false)
    } catch (error) {
      logger.error('todos-view', '恢复待办失败', error)
      toast(String(error))
    }
    return
  }
  try {
    await api.todos.complete(todo.id, true)
  } catch (error) {
    logger.error('todos-view', '完成待办失败', error)
    toast(String(error))
  }
}

async function changePriority(todo: Todo, priority: Priority): Promise<void> {
  if (guardDone(todo, '修改优先级')) return
  try {
    await api.todos.priority(todo.id, priority)
    toast('优先级已更新')
  } catch (error) {
    logger.error('todos-view', '修改优先级失败', error)
    toast(String(error))
  }
}

async function removeTodo(todo: Todo): Promise<void> {
  if (guardDone(todo, '删除')) return
  try {
    await api.todos.remove(todo.id)
    toast('已删除')
  } catch (error) {
    logger.error('todos-view', '删除待办失败', error)
    toast(String(error))
  }
}

/** 🔁 徽章 → 重复菜单（原型 showRepeatMenu）。 */
async function openRepeat(todo: Todo, anchor: HTMLElement): Promise<void> {
  if (guardDone(todo, '修改重复提醒')) return
  repeatTarget = todo
  await repeatMenu.value?.open(anchor, todo.repeat_rule)
}

/** 菜单选中：只改 repeat_rule，其余提醒字段原样回写。 */
async function applyRepeat(rule: string | null): Promise<void> {
  const todo = repeatTarget
  repeatTarget = null
  if (!todo) return
  // 菜单打开期间提醒可能被独立编辑窗改过，回写前取最新值；「是否有变化」也要按最新值判断。
  const fresh = todos.value.find((t) => t.id === todo.id) ?? todo
  if ((fresh.repeat_rule ?? null) === rule) return
  logger.info('todos-view', `设置重复提醒 id=${todo.id} rule=${rule ?? '(none)'}`)
  try {
    await api.todos.reminder(todo.id, fresh.remind_offset_minutes, fresh.remind_desktop, fresh.remind_email, rule)
    toast(`已设为${rule === 'daily' ? '每天重复' : rule === 'weekly' ? '每周重复' : '不重复'}`)
  } catch (error) {
    logger.error('todos-view', '设置重复提醒失败', error)
    toast(String(error))
  }
}
</script>

<template>
  <div class="archive-page">
    <div class="todo-date-bar">
      <button type="button" class="btn tiny" title="前一天" @click="currentDate = shiftDateKey(currentDate, -1)">
        ‹
      </button>
      <span class="todo-date-label">{{ currentDate }}</span>
      <button type="button" class="btn tiny" title="后一天" @click="currentDate = shiftDateKey(currentDate, 1)">
        ›
      </button>
      <button type="button" class="btn tiny" title="回到今天" @click="currentDate = todayKey()">今天</button>
      <span class="todo-date-hint">逾期未完成自动置顶</span>

      <input v-model="keyword" class="search-input todo-search" placeholder="🔍 搜索全部待办（含子任务）…" />
      <button
        type="button"
        class="icon-btn todo-new-btn"
        title="新增待办事项"
        @click="openEditor('create', null, null, $event.currentTarget as HTMLElement)"
      >
        <svg width="13" height="13" viewBox="0 0 12 12" fill="none">
          <path d="M6 1v10M1 6h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
        </svg>
      </button>
    </div>

    <TodoTree
      ref="tree"
      :todos="visible"
      :remark-style="settings.remark_style"
      :show-date-chip="searching"
      :query="keyword"
      :force-expand="search.forceExpand"
      :hit-ids="search.hitIds"
      archive
      confirm-fallback="center"
      @toggle-done="toggleDone"
      @edit="openEditor('edit', $event, null, cardOf($event))"
      @edit-due="openEditor('due', $event, null, cardOf($event))"
      @edit-remind="openEditor('remind', $event, null, cardOf($event))"
      @edit-remark="openEditor('remark', $event, null, cardOf($event))"
      @edit-repeat="openRepeat"
      @add-sub="openEditor('child', null, $event, cardOf($event))"
      @open-tags="openEditor('tags', $event, null, cardOf($event))"
      @priority="changePriority"
      @delete="removeTodo"
    />
    <RepeatMenu ref="repeatMenu" @pick="applyRepeat" />

    <div v-if="!visible.length" class="todo-empty">
      {{ searching ? `未找到匹配「${keyword.trim()}」的待办事项` : '该日暂无待办事项' }}
    </div>
  </div>
</template>
