<script setup lang="ts">
import { computed, ref } from 'vue'
import { vStaggerList } from '@/motion'
import TodoCard from '@/components/card/TodoCard.vue'
import CardConfirm from '@/components/base/CardConfirm.vue'
import PriorityMenu from '@/components/todo/PriorityMenu.vue'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import type { Priority, RemarkStyle, Todo } from '@/typings/domain'
import { dateKeyOf, formatDateKey } from '@/utils/datetime'
import { buildTodoTree, partitionOverdue, type TodoNode } from '@/utils/todo'

/**
 * 待办列表（原型 renderTodoList）：逾期置顶分区 + 两层树结构。
 *
 * - 逾期事项归入顶部「⚠️ 逾期事项」分区并标注项数；父待办存在逾期子任务时整棵树一并归入；
 * - 子任务完成后排在子级列表末尾；树连接线与折叠由 CSS 负责（.todo-children 的 ::before/::after）；
 * - 搜索态：`query` 下发给卡片做 <mark> 高亮；`forceExpand` 里的父级强制展开（只命中子任务时）；
 *   `hitIds` 里的子任务加 search-hit 虚线框；
 * - `archive`：归档页列表加 todo-arch-list，解除面板列表的 380px 限高。
 */
const props = withDefaults(
  defineProps<{
    /** 扁平待办列表，组件内部自行组树与分区。 */
    todos: readonly Todo[]
    remarkStyle?: RemarkStyle
    /** 搜索关键词非空时展示所属日期徽章。 */
    showDateChip?: boolean
    /** 搜索关键词，用于正文高亮。 */
    query?: string
    /** 搜索时需强制展开的父级 id（只命中子任务的情况）。 */
    forceExpand?: ReadonlySet<string>
    /** 搜索命中的子任务 id。 */
    hitIds?: ReadonlySet<string>
    /** 归档页形态：不限高。 */
    archive?: boolean
    /** 删除确认浮层左右都放不下时的兜底：面板 'below'（D27）、主窗口 'center'。 */
    confirmFallback?: 'center' | 'below'
  }>(),
  {
    remarkStyle: 'mixed',
    showDateChip: false,
    query: '',
    forceExpand: undefined,
    hitIds: undefined,
    archive: false,
    confirmFallback: 'below',
  },
)

const emit = defineEmits<{
  (e: 'toggle-done', todo: Todo): void
  (e: 'edit', todo: Todo): void
  (e: 'edit-remark', todo: Todo): void
  (e: 'edit-due', todo: Todo): void
  (e: 'edit-remind', todo: Todo): void
  (e: 'edit-repeat', todo: Todo, anchor: HTMLElement): void
  (e: 'add-sub', parent: Todo): void
  (e: 'delete', todo: Todo): void
  (e: 'priority', todo: Todo, priority: Priority): void
  (e: 'open-tags', todo: Todo): void
}>()

const confirm = useConfirmDelete('todo-tree')
/** 列表根元素：CardConfirm 反查卡片、Task 3 的编辑浮层锚点都从这里查。 */
const listEl = ref<HTMLElement | null>(null)

/** 待确认项的文案：子任务与父待办措辞不同（原型 pendingDeleteText）。 */
const confirmText = computed(() => {
  const id = confirm.pendingId.value
  const target = id ? props.todos.find((todo) => todo.id === id) : undefined
  return target?.parent_id ? '确认删除该子任务？' : '确认删除该待办事项？'
})

/** 按待办 id 取卡片元素（编辑浮层锚点，spec D28）。 */
function cardOf(id: string): HTMLElement | null {
  return listEl.value?.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`) ?? null
}

/** 折叠状态按待办 id 记录，默认展开。 */
const collapsed = ref<Set<string>>(new Set())

const priorityMenu = ref<InstanceType<typeof PriorityMenu> | null>(null)
/** 当前正在改优先级的待办，菜单选中后据此派发。 */
let priorityTarget: Todo | null = null

const tree = computed<TodoNode[]>(() => buildTodoTree(props.todos))
const partitioned = computed(() => partitionOverdue(tree.value))
/** 渲染顺序：逾期分区在前，常规在后；分区标题插在首项之前。 */
const ordered = computed<TodoNode[]>(() => [...partitioned.value.overdue, ...partitioned.value.normal])

/** 搜索命中子任务时父级强制展开，忽略用户的折叠记录。 */
function isCollapsed(id: string): boolean {
  return collapsed.value.has(id) && !props.forceExpand?.has(id)
}

function toggleCollapse(id: string): void {
  const next = new Set(collapsed.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  collapsed.value = next
}

/** 搜索态的所属日期徽章文案（原型：日期 + 今天后缀）。 */
function dateChipOf(todo: Todo): string {
  return props.showDateChip ? formatDateKey(dateKeyOf(todo.due_at), { todaySuffix: true }) : ''
}

async function openPriority(todo: Todo, anchor: HTMLElement): Promise<void> {
  priorityTarget = todo
  await priorityMenu.value?.open(anchor, todo.priority as Priority)
}

function onPrioritySelected(priority: Priority): void {
  if (!priorityTarget) return
  // 仅当值真的变化时才派发，避免无谓的写库与重排。
  if (priorityTarget.priority !== priority) emit('priority', priorityTarget, priority)
  priorityTarget = null
}

function askDelete(todo: Todo): void {
  confirm.ask(todo.id)
}

/** CardConfirm 确认：pendingId 就是要删的 id，从当前列表找回对象派发。 */
function confirmDelete(): void {
  const id = confirm.confirm()
  const target = id ? props.todos.find((todo) => todo.id === id) : undefined
  if (target) emit('delete', target)
}

/**
 * 收起本组件的浮层（面板 Esc 链 / 切页副作用用）：删除确认 + 优先级菜单；关了任一项返回 true。
 * 原型 switchMode 会一并隐藏 #repeatMenu / #prioMenu。
 */
function dismissOverlays(): boolean {
  const menuClosed = priorityMenu.value?.dismiss() ?? false
  if (menuClosed) priorityTarget = null
  if (!confirm.pendingId.value) return menuClosed
  confirm.cancel()
  return true
}

defineExpose({ dismissOverlays, cardOf })
</script>

<template>
  <ul ref="listEl" v-stagger-list class="todo-list" :class="{ 'todo-arch-list': props.archive }">
    <template v-for="(node, index) in ordered" :key="node.todo.id">
      <!-- ⚠️ 逾期分区标题：置于首个逾期项之前 -->
      <li v-if="index === 0 && partitioned.overdue.length" class="todo-section">
        ⚠️ 逾期事项 · 按完成时间与优先级置顶（{{ partitioned.overdue.length }} 项）
      </li>

      <TodoCard
        :todo="node.todo"
        :child-count="node.children.length"
        :has-children="node.children.length > 0"
        :collapsed="isCollapsed(node.todo.id)"
        :remark-style="props.remarkStyle"
        :confirming="confirm.isPending(node.todo.id)"
        :date-chip="dateChipOf(node.todo)"
        :query="props.query"
        @toggle-done="emit('toggle-done', node.todo)"
        @toggle-collapse="toggleCollapse(node.todo.id)"
        @open-priority="openPriority(node.todo, $event)"
        @edit="emit('edit', node.todo)"
        @edit-remark="emit('edit-remark', node.todo)"
        @edit-due="emit('edit-due', node.todo)"
        @edit-remind="emit('edit-remind', node.todo)"
        @edit-repeat="emit('edit-repeat', node.todo, $event)"
        @add-sub="emit('add-sub', node.todo)"
        @open-tags="emit('open-tags', node.todo)"
        @ask-delete="askDelete(node.todo)"
      >
        <template #children>
          <ul v-if="node.children.length && !isCollapsed(node.todo.id)" class="todo-children">
            <TodoCard
              v-for="child in node.children"
              :key="child.id"
              :todo="child"
              :depth="1"
              :remark-style="props.remarkStyle"
              :confirming="confirm.isPending(child.id)"
              :query="props.query"
              :search-hit="props.hitIds?.has(child.id) ?? false"
              @toggle-done="emit('toggle-done', child)"
              @open-priority="openPriority(child, $event)"
              @edit="emit('edit', child)"
              @edit-remark="emit('edit-remark', child)"
              @edit-due="emit('edit-due', child)"
              @edit-remind="emit('edit-remind', child)"
              @edit-repeat="emit('edit-repeat', child, $event)"
              @open-tags="emit('open-tags', child)"
              @ask-delete="askDelete(child)"
            />
          </ul>
        </template>
      </TodoCard>
    </template>

    <PriorityMenu ref="priorityMenu" @select="onPrioritySelected" />
    <CardConfirm
      :text="confirmText"
      :target-id="confirm.pendingId.value"
      :container="listEl"
      :fallback="props.confirmFallback"
      @confirm="confirmDelete"
      @cancel="confirm.cancel()"
    />
  </ul>
</template>
