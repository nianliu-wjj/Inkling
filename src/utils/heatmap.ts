import type { ActivityDay } from '@/typings/domain'
import { toDateKey } from './datetime'

/**
 * 热力图网格纯函数（原型 renderMiniHeat / renderHeatmap 的口径）。
 *
 * - 一律**周一对齐**（原型 `(getDay() + 6) % 7`）；
 * - 档位阈值分两套：侧栏迷你热力图 <3 / <6 / <10，统计页 <5 / <10 / <18；
 * - 透明度阶 [0, .18, .38, .6, .88] 叠在主题令牌 --hm-base 上，由调用方拼 rgba。
 * 纯函数，不依赖 Vue 与 DOM，可直接用 node:test 覆盖。
 */

export type HeatLevel = 0 | 1 | 2 | 3 | 4

/** 侧栏迷你热力图档位阈值（total < t[0] → 1 级 … ≥ t[2] → 4 级）。 */
export const MINI_THRESHOLDS: readonly [number, number, number] = [3, 6, 10]
/** 统计页热力图档位阈值。 */
export const STATS_THRESHOLDS: readonly [number, number, number] = [5, 10, 18]

const ALPHAS = [0, 0.18, 0.38, 0.6, 0.88] as const

/** 活跃总量 → 档位（0 = 无记录，落回 CSS 默认底色）。 */
export function levelOf(total: number, thresholds: readonly [number, number, number]): HeatLevel {
  if (total <= 0) return 0
  if (total < thresholds[0]) return 1
  if (total < thresholds[1]) return 2
  if (total < thresholds[2]) return 3
  return 4
}

/** 档位 → 叠加在 --hm-base 上的透明度。 */
export function alphaOf(level: HeatLevel): number {
  return ALPHAS[level]
}

export interface MonthCell {
  /** 日期键；空位为 `blank-<序号>`。 */
  key: string
  blank: boolean
  level: HeatLevel
  overdue: boolean
  selected: boolean
  day: ActivityDay | null
}

function totalOf(day: ActivityDay | null): number {
  return day ? day.notes + day.clips + day.todos : 0
}

/**
 * 当月网格：列为周（周一起），行为星期；首尾补空位使总格数为 7 的倍数。
 * `month0` 为 0 起的月份序号（与 Date#getMonth 一致）。
 */
export function buildMonthGrid(
  activity: readonly ActivityDay[],
  year: number,
  month0: number,
  opts: { selected?: string } = {},
): MonthCell[] {
  const byDate = new Map(activity.map((d) => [d.date, d]))
  const lead = (new Date(year, month0, 1).getDay() + 6) % 7
  const daysInMonth = new Date(year, month0 + 1, 0).getDate()
  const cols = Math.ceil((lead + daysInMonth) / 7)
  const cells: MonthCell[] = []
  for (let i = 0; i < cols * 7; i += 1) {
    const dayNum = i - lead + 1
    if (dayNum < 1 || dayNum > daysInMonth) {
      cells.push({ key: `blank-${i}`, blank: true, level: 0, overdue: false, selected: false, day: null })
      continue
    }
    const key = toDateKey(new Date(year, month0, dayNum))
    const day = byDate.get(key) ?? null
    cells.push({
      key,
      blank: false,
      level: levelOf(totalOf(day), MINI_THRESHOLDS),
      overdue: (day?.overdue ?? 0) > 0,
      selected: opts.selected === key,
      day,
    })
  }
  return cells
}

export interface RangeCell {
  key: string
  level: HeatLevel
  overdue: boolean
  day: ActivityDay | null
}

/**
 * 近 N 天网格（默认 182），起点回退到周一，终点为今天；
 * 月份标签：每列（周）首格月份变化、且与上一标签间距 ≥ 3 列时才记录，避免重叠。
 */
export function buildRangeGrid(
  activity: readonly ActivityDay[],
  opts: { days?: number; today?: Date } = {},
): { cells: RangeCell[]; months: { label: string; column: number }[] } {
  const days = opts.days ?? 182
  const today = opts.today ?? new Date()
  const byDate = new Map(activity.map((d) => [d.date, d]))
  const start = new Date(today.getFullYear(), today.getMonth(), today.getDate())
  start.setDate(start.getDate() - (days - 1))
  start.setDate(start.getDate() - ((start.getDay() + 6) % 7))
  const end = new Date(today.getFullYear(), today.getMonth(), today.getDate())

  const cells: RangeCell[] = []
  const months: { label: string; column: number }[] = []
  let prevMonth = -1
  let lastColumn = -99
  const cursor = new Date(start)
  let index = 0
  while (cursor <= end) {
    const key = toDateKey(cursor)
    const day = byDate.get(key) ?? null
    cells.push({ key, level: levelOf(totalOf(day), STATS_THRESHOLDS), overdue: (day?.overdue ?? 0) > 0, day })
    if (index % 7 === 0) {
      const month = cursor.getMonth() + 1
      const column = index / 7
      if (month !== prevMonth && column - lastColumn >= 3) {
        months.push({ label: `${month}月`, column })
        prevMonth = month
        lastColumn = column
      }
    }
    cursor.setDate(cursor.getDate() + 1)
    index += 1
  }
  return { cells, months }
}
