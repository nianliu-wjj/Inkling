import type { Priority, Todo } from '@/typings/domain'
import { parseTime } from './datetime'

/**
 * 待办排序与分区。
 *
 * 与后端 `src-tauri/src/domain/todo.rs` 保持同一套口径：
 * 排序权重、逾期判定必须与 Rust 侧一致，否则前后端展示会打架。
 */

/** 优先级权重：高 → 中 → 低（与 domain/todo.rs::priority_weight 一致）。 */
const PRIORITY_WEIGHT: Record<Priority, number> = { high: 0, medium: 1, low: 2 }

function weightOf(priority: string): number {
  return PRIORITY_WEIGHT[priority as Priority] ?? 3
}

/** 毫秒时间戳；无效时间排到最后。 */
function timeOf(value: string): number {
  return parseTime(value)?.getTime() ?? Number.MAX_SAFE_INTEGER
}

/**
 * 逾期判定（需求 2.2）：未完成且**完成时刻已过**即逾期。
 * 当日事项完成时刻一过就立刻标红，不等到次日。
 */
export function isOverdue(todo: Todo, now: Date = new Date()): boolean {
  return todo.status === 'open' && timeOf(todo.due_at) < now.getTime()
}

/**
 * 稳定排序（需求 2.2「排序规则 v1.2」）：
 * 完成时间升序 → 优先级高在前 → 创建时间升序；**已完成的沉底**。
 *
 * 返回新数组，不修改入参。
 */
export function sortTodos(todos: readonly Todo[]): Todo[] {
  return [...todos].sort((a, b) => {
    // 已完成统一沉底，其内部仍按同样规则排。
    const doneA = a.status === 'done' ? 1 : 0
    const doneB = b.status === 'done' ? 1 : 0
    if (doneA !== doneB) return doneA - doneB

    const dueDiff = timeOf(a.due_at) - timeOf(b.due_at)
    if (dueDiff !== 0) return dueDiff

    const prioDiff = weightOf(a.priority) - weightOf(b.priority)
    if (prioDiff !== 0) return prioDiff

    return timeOf(a.created_at) - timeOf(b.created_at)
  })
}

/** 顶级待办及其子任务组成的一棵树。 */
export interface TodoNode {
  todo: Todo
  children: Todo[]
}

/**
 * 把扁平列表组装成「顶级待办 + 子任务」两层结构。
 * 子任务不可再嵌套（后端 validate_parent 已保证），因此只有两层。
 */
export function buildTodoTree(todos: readonly Todo[]): TodoNode[] {
  const roots = todos.filter((t) => !t.parent_id)
  const childrenOf = new Map<string, Todo[]>()

  for (const todo of todos) {
    if (!todo.parent_id) continue
    const bucket = childrenOf.get(todo.parent_id)
    if (bucket) bucket.push(todo)
    else childrenOf.set(todo.parent_id, [todo])
  }

  return sortTodos(roots).map((todo) => ({
    todo,
    // 子任务同样排序；已完成子任务沉到子列表末尾（需求 2.2）。
    children: sortTodos(childrenOf.get(todo.id) ?? []),
  }))
}

/**
 * 逾期置顶分区（需求 2.2「逾期置顶」）。
 *
 * 关键规则：**父待办存在逾期子任务时，父待办连同其全部子任务
 * （含已完成的）整体归入逾期分区**，而不是只把那个子任务拎出来。
 */
export function partitionOverdue(
  nodes: readonly TodoNode[],
  now: Date = new Date(),
): { overdue: TodoNode[]; normal: TodoNode[] } {
  const overdue: TodoNode[] = []
  const normal: TodoNode[] = []

  for (const node of nodes) {
    const selfOverdue = isOverdue(node.todo, now)
    const hasOverdueChild = node.children.some((child) => isOverdue(child, now))
    if (selfOverdue || hasOverdueChild) overdue.push(node)
    else normal.push(node)
  }

  return { overdue, normal }
}

/**
 * 判断某棵树是否归属于指定日期（本地时区）。
 *
 * 归属日 = 完成时间的日期（需求 2.2「日期视图」）。
 */
export function belongsToDate(todo: Todo, dateKey: string): boolean {
  const due = parseTime(todo.due_at)
  if (!due) return false
  const year = due.getFullYear()
  const month = String(due.getMonth() + 1).padStart(2, '0')
  const day = String(due.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}` === dateKey
}
