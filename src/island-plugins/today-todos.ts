import { computed, type Ref } from 'vue'
import { useSettings, useTodos } from '@/composables/useData'
import type { Priority } from '@/typings/domain'
import { todayKey } from '@/utils/datetime'
import { pickIslandTodos } from '@/utils/island'
import type { IslandItem } from './index'
import TodoDetail from './TodoDetail.vue'

/**
 * 「当日待办」灵动岛插件的条目来源（spec 4B §3 #3）。
 *
 * 口径由 `utils/island.ts::pickIslandTodos` 统一（含子任务、播放范围 today / all、逾期优先），
 * 与主窗口灵动岛页预览同源；跟随 todos-changed 与 settings-changed 自动刷新。
 */

/** 优先级 → 色点颜色（悬停详情组件 TodoDetail 用；胶囊本体改用原型 .di-dot.{prio} 类）。 */
const PRIORITY_DOT: Record<Priority, string> = {
  high: 'var(--red)',
  medium: 'var(--gold)',
  low: 'var(--green)',
}

export function useTodayTodos(): Ref<IslandItem[]> {
  const { todos } = useTodos()
  const { settings } = useSettings()
  return computed<IslandItem[]>(() => {
    const today = todayKey()
    return pickIslandTodos(todos.value, settings.value.island_scope, today).map((row) => ({
      id: row.id,
      title: row.text,
      meta: row.time,
      dot: PRIORITY_DOT[row.priority] ?? 'var(--text-dim)',
      // 原型：非当天才显示 MM-DD 徽章（t.date.slice(5)）。
      date: row.date !== today ? row.date.slice(5) : undefined,
      overdue: row.overdue,
      priority: row.priority,
      detail: TodoDetail,
      payload: row.todo,
    }))
  })
}
