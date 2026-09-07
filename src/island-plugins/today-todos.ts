import { computed, type Ref } from 'vue'
import { useTodos } from '@/composables/useData'
import type { Priority, Todo } from '@/typings/domain'
import { dateKeyOf, todayKey } from '@/utils/datetime'
import { isOverdue } from '@/utils/todo'
import type { IslandItem } from './index'
import TodoDetail from './TodoDetail.vue'

/** 优先级 → 色点颜色（与优先级菜单同一套令牌）。 */
const PRIORITY_DOT: Record<Priority, string> = {
  high: 'var(--red)',
  medium: 'var(--gold)',
  low: 'var(--green)',
}

/** 完成时间的 HH:mm；解析失败返回空串。 */
export function formatClock(iso: string): string {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return ''
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`
}

/**
 * 当日待办口径（与面板待办页、托盘提示一致）：顶级待办（不含子任务）、未完成、
 * 属于今天或此前逾期；按完成时间升序。
 */
export function pickTodayTodos(todos: readonly Todo[], today = todayKey()): Todo[] {
  return todos
    .filter((todo) => !todo.parent_id && todo.status !== 'done')
    .filter((todo) => dateKeyOf(todo.due_at) === today || isOverdue(todo))
    .sort((a, b) => a.due_at.localeCompare(b.due_at))
}

/** 「当日待办」插件的条目来源：跟随 todos-changed 自动刷新。 */
export function useTodayTodos(): Ref<IslandItem[]> {
  const { todos } = useTodos()
  return computed<IslandItem[]>(() =>
    pickTodayTodos(todos.value).map((todo) => ({
      id: todo.id,
      title: todo.content,
      meta: formatClock(todo.due_at),
      dot: PRIORITY_DOT[todo.priority] ?? 'var(--text-dim)',
      detail: TodoDetail,
      payload: todo,
    })),
  )
}
