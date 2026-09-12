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
import { dateKeyOf, todayKey } from '@/utils/datetime'
import { isOverdue } from '@/utils/todo'

/**
 * 面板 · 待办模式。
 *
 * 需求 2.2：面板待办页具备全部能力（优先级/逾期/子任务/标签/备注/重复提醒），
 * 但**仅可查看当日待办，不提供日期切换**；逾期事项自动置顶。
 *
 * 展示口径：今天的事项 + 此前日期仍未完成的逾期事项；未来日期不提前进入。
 *
 * 新增 / 编辑走**独立编辑窗口**（editor，spec D23）：面板只有 480px 宽，锚定浮层会被窗口边界裁切，
 * 因此由后端打开铺满工作区的透明窗，浮层在窗内锚定到卡片右侧（锚点经 useEditorWindow 换算为物理像素）；
 * 保存后由 todosChanged 事件驱动刷新。
 */
/** 通知面板：编辑窗口已打开，面板此期间不得因失焦而收起。 */
const emit = defineEmits<{ (e: 'externalEditor'): void }>()

const { todos } = useTodos()
const { settings } = useSettings()
const { toast } = useToast()
const { openTodoEditor } = useEditorWindow('panel-todo')

const keyword = ref('')
const priorityFilter = ref<'all' | Priority>('all')
/** 待办树实例：删除确认态在它内部，Esc 链 / 切页副作用经它取消。 */
const tree = ref<InstanceType<typeof TodoTree> | null>(null)
const repeatMenu = ref<InstanceType<typeof RepeatMenu> | null>(null)
/** 正在改重复规则的待办；菜单选中后据此写库。 */
let repeatTarget: Todo | null = null

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

/**
 * 打开独立编辑窗口。只传 ID 不传整个对象：编辑窗口按 ID 从自己那份最新列表中取，
 * 避免面板持有的旧快照覆盖掉别处刚改过的字段。锚点是卡片（spec D28）或顶部按钮。
 */
function openEditor(
  mode: TodoEditorMode,
  todo: Todo | null = null,
  parent: Todo | null = null,
  anchorEl: HTMLElement | null = null,
): void {
  // 先上报再 invoke：编辑窗口一拿到焦点面板就会 blur，晚于 blur 上报会来不及阻止收起。
  emit('externalEditor')
  void openTodoEditor({ mode, todoId: todo?.id ?? null, parentId: parent?.id ?? null, anchorEl })
}

/** 卡片级锚点：从待办树反查 [data-id]。 */
function cardOf(todo: Todo): HTMLElement | null {
  return tree.value?.cardOf(todo.id) ?? null
}

/** 已完成事项一律拦截修改（需求 2.2）。 */
function guardDone(todo: Todo, action: string): boolean {
  if (todo.status === 'done') {
    toast(`已完成事项不可${action}`)
    return true
  }
  return false
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
  logger.info('panel-todo', `设置重复提醒 id=${todo.id} rule=${rule ?? '(none)'}`)
  try {
    await api.todos.reminder(todo.id, fresh.remind_offset_minutes, fresh.remind_desktop, fresh.remind_email, rule)
    toast(`已设为${rule === 'daily' ? '每天重复' : rule === 'weekly' ? '每周重复' : '不重复'}`)
  } catch (error) {
    logger.error('panel-todo', '设置重复提醒失败', error)
    toast(String(error))
  }
}

/** 关闭本页浮层：删除确认 / 优先级菜单在 TodoTree 内部，转调它；重复菜单挂在本页。原型 switchMode 会一并隐藏 #repeatMenu / #prioMenu。 */
function dismissOverlays(): boolean {
  const treeClosed = tree.value?.dismissOverlays() ?? false
  const repeatClosed = repeatMenu.value?.dismiss() ?? false
  if (repeatClosed) repeatTarget = null
  return treeClosed || repeatClosed
}

defineExpose({ dismissOverlays })
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
      <button
        type="button"
        class="btn tiny"
        title="新增待办事项（可设置完成时间）"
        @click="openEditor('create', null, null, $event.currentTarget as HTMLElement)"
      >
        📅
      </button>
    </div>
    <!-- 原型 .todo-panel-hint：操作提示行 -->
    <div class="todo-panel-hint">
      搜索 / 优先级过滤当日待办 · 逾期事项自动置顶 · 点击优先级徽章可调整 · 点击 📅 新增待办
    </div>

    <TodoTree
      ref="tree"
      :todos="filtered"
      :remark-style="settings.remark_style"
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
  </section>
</template>
