import type { IslandScope, Priority, Todo } from '@/typings/domain'
import { dateKeyOf, formatClock, todayKey } from './datetime'
import { isOverdue } from './todo'

/**
 * 灵动岛播放口径纯函数（原型 app.js `renderIsland` 的收集与排序段，spec 4B §3 #3）。
 *
 * - 未完成的顶级待办与子任务都入选；子任务文案 `父内容 / 子内容`（父不存在时只用子内容），
 *   优先级 / 归属日 / 逾期都取子任务自身；
 * - scope = 'today' 取归属日等于 today 的，以及此前已逾期的未完成项；'all' 取全部未完成；
 * - 排序：逾期在前，其余按完成时间升序（原型按 dueTime 字串比较，这里按 due_at 全时刻比较，
 *   'all' 范围下跨日也能排对）。
 *
 * 不碰 DOM、不读时钟（today / now 可注入），灵动岛胶囊与主窗口灵动岛页预览共用，保证两处同源。
 */

export interface IslandTodoRow {
  id: string
  /** 胶囊文案：顶级 = content；子任务 = `父内容 / 子内容`。 */
  text: string
  /** 归属日（YYYY-MM-DD，本地时区）。 */
  date: string
  /** HH:mm；解析失败为空串。 */
  time: string
  overdue: boolean
  priority: Priority
  todo: Todo
}

export function pickIslandTodos(
  todos: readonly Todo[],
  scope: IslandScope,
  today: string = todayKey(),
  now: Date = new Date(),
): IslandTodoRow[] {
  const byId = new Map(todos.map((todo) => [todo.id, todo] as const))
  const rows: IslandTodoRow[] = []
  for (const todo of todos) {
    if (todo.status === 'done') continue
    const date = dateKeyOf(todo.due_at)
    // today 口径与面板待办页 / 托盘一致：当天 + 此前逾期未完成（原型 inScope 只看当天，但本项目自阶段二起
    // 把逾期置顶作为产品语义，灵动岛不能让跨日逾期项凭空消失）；all 取全部未完成。
    const overdue = isOverdue(todo, now)
    if (scope === 'today' && date !== today && !overdue) continue
    const parent = todo.parent_id ? byId.get(todo.parent_id) : undefined
    rows.push({
      id: todo.id,
      text: parent ? `${parent.content} / ${todo.content}` : todo.content,
      date,
      time: formatClock(todo.due_at),
      overdue,
      priority: todo.priority,
      todo,
    })
  }
  rows.sort((a, b) => Number(b.overdue) - Number(a.overdue) || a.todo.due_at.localeCompare(b.todo.due_at))
  return rows
}
