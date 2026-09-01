<script setup lang="ts">
import { computed, ref } from 'vue'
import TodoCard from '@/components/card/TodoCard.vue'
import PriorityMenu from '@/components/todo/PriorityMenu.vue'
import { useConfirmDelete } from '@/composables/useConfirmDelete'
import type { Priority, RemarkStyle, Todo } from '@/typings/domain'
import { dateKeyOf } from '@/utils/datetime'
import { buildTodoTree, partitionOverdue, type TodoNode } from '@/utils/todo'

/**
 * 待办列表：逾期置顶分区 + 两层树结构。
 *
 * 需求 2.2：
 * - 逾期事项归入顶部「⚠️ 逾期事项」分区并标注项数；
 * - 父待办存在逾期子任务时，整棵树（含已完成子任务）一并归入该分区；
 * - 子任务完成后排在子级列表末尾；
 * - 树连接线与折叠由 CSS 负责（.todo-children 的 ::before/::after）。
 */
const props = withDefaults(
  defineProps<{
    /** 扁平待办列表，组件内部自行组树与分区。 */
    todos: readonly Todo[]
    remarkStyle?: RemarkStyle
    /** 搜索关键词非空时展示所属日期徽章。 */
    showDateChip?: boolean
  }>(),
  { remarkStyle: 'mixed', showDateChip: false },
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

function isCollapsed(id: string): boolean {
  return collapsed.value.has(id)
}

function toggleCollapse(id: string): void {
  const next = new Set(collapsed.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  collapsed.value = next
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
</script>

<template>
  <ul class="todo-list">
    <!-- ⚠️ 逾期分区：标注项数，置于列表顶部 -->
    <template v-if="partitioned.overdue.length">
      <li class="todo-section">⚠️ 逾期事项（{{ partitioned.overdue.length }}）</li>
      <TodoCard
        v-for="node in partitioned.overdue"
        :key="node.todo.id"
        :todo="node.todo"
        :is-parent="true"
        :has-children="node.children.length > 0"
        :collapsed="isCollapsed(node.todo.id)"
        :remark-style="props.remarkStyle"
        :confirming="confirm.isPending(node.todo.id)"
        :date-chip="props.showDateChip ? dateKeyOf(node.todo.due_at) : ''"
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

    <!-- 常规分区 -->
    <TodoCard
      v-for="node in partitioned.normal"
      :key="node.todo.id"
      :todo="node.todo"
      :is-parent="true"
      :has-children="node.children.length > 0"
      :collapsed="isCollapsed(node.todo.id)"
      :remark-style="props.remarkStyle"
      :confirming="confirm.isPending(node.todo.id)"
      :date-chip="props.showDateChip ? dateKeyOf(node.todo.due_at) : ''"
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

    <PriorityMenu ref="priorityMenu" @select="onPrioritySelected" />
  </ul>
</template>
