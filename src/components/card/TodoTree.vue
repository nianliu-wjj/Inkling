<script setup lang="ts">
import { computed, ref } from 'vue'
import { vStaggerList } from '@/motion'
import TodoCard from '@/components/card/TodoCard.vue'
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
  }>(),
  { remarkStyle: 'mixed', showDateChip: false, query: '', forceExpand: undefined, hitIds: undefined, archive: false },
)

const emit = defineEmits<{
  (e: 'toggle-done', todo: Todo): void
  (e: 'edit', todo: Todo): void
  (e: 'edit-due', todo: Todo): void
  (e: 'edit-remind', todo: Todo): void
  (e: 'edit-repeat', todo: Todo, anchor: HTMLElement): void
  (e: 'add-sub', parent: Todo): void
  (e: 'delete', todo: Todo): void
  (e: 'priority', todo: Todo, priority: Priority): void
  (e: 'open-tags', todo: Todo): void
}>()

const confirm = useConfirmDelete('todo-tree')
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

function confirmDelete(todo: Todo): void {
  if (confirm.confirm()) emit('delete', todo)
}

/** 取消待确认的删除（面板 Esc 链 / 切页副作用用）；有待确认项返回 true。 */
function dismissConfirm(): boolean {
  if (!confirm.pendingId.value) return false
  confirm.cancel()
  return true
}

defineExpose({ dismissConfirm })
</script>

<template>
  <ul v-stagger-list class="todo-list" :class="{ 'todo-arch-list': props.archive }">
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
        @edit-due="emit('edit-due', node.todo)"
        @edit-remind="emit('edit-remind', node.todo)"
        @edit-repeat="emit('edit-repeat', node.todo, $event)"
        @add-sub="emit('add-sub', node.todo)"
        @open-tags="emit('open-tags', node.todo)"
        @ask-delete="askDelete(node.todo)"
        @confirm-delete="confirmDelete(node.todo)"
        @cancel-delete="confirm.cancel()"
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
              @edit-due="emit('edit-due', child)"
              @edit-remind="emit('edit-remind', child)"
              @edit-repeat="emit('edit-repeat', child, $event)"
              @open-tags="emit('open-tags', child)"
              @ask-delete="askDelete(child)"
              @confirm-delete="confirmDelete(child)"
              @cancel-delete="confirm.cancel()"
            />
          </ul>
        </template>
      </TodoCard>
    </template>

    <PriorityMenu ref="priorityMenu" @select="onPrioritySelected" />
  </ul>
</template>
