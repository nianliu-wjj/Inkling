/**
 * 日期时间工具。
 *
 * 时间约定（需求 2.2「日期与逾期展示口径」）：
 * - 后端一律存 RFC3339 UTC 字符串；
 * - 前端展示与「归属日期」计算一律按**用户本地时区**解释；
 * - 严禁用 `iso.slice(0, 10)` 截断 UTC 字符串当本地日期——跨午夜会整体错一天。
 */

/** 一天的毫秒数，用于「今天/明天/昨天」判定。 */
const MS_PER_DAY = 86_400_000

/** 把 RFC3339 字符串解析为 Date；无效值返回 null 而非抛错。 */
export function parseTime(value: string | null | undefined): Date | null {
  if (!value) return null
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? null : date
}

/**
 * 取本地时区下的日期键（YYYY-MM-DD）。
 * 这是待办「归属日」、热力图格子、日期详情页的统一口径。
 */
export function toDateKey(value: Date): string {
  const year = value.getFullYear()
  const month = String(value.getMonth() + 1).padStart(2, '0')
  const day = String(value.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

/** RFC3339 → 本地日期键，供按日分组使用。 */
export function dateKeyOf(value: string): string {
  const date = parseTime(value)
  return date ? toDateKey(date) : ''
}

/** 今天的本地日期键。 */
export function todayKey(): string {
  return toDateKey(new Date())
}

/** 日期键偏移若干天，用于 ‹ / › 日期切换。 */
export function shiftDateKey(key: string, deltaDays: number): string {
  const [year, month, day] = key.split('-').map(Number)
  // 用本地构造函数而非 Date.parse，避免 'YYYY-MM-DD' 被当成 UTC 解析。
  const date = new Date(year, month - 1, day)
  date.setDate(date.getDate() + deltaDays)
  return toDateKey(date)
}

/** `<input type="datetime-local">` 需要的本地时间串（YYYY-MM-DDTHH:mm）。 */
export function toLocalInput(value: Date): string {
  const offset = value.getTimezoneOffset()
  return new Date(value.getTime() - offset * 60_000).toISOString().slice(0, 16)
}

/** 拆成 date / time 两个原生选择器需要的值。 */
export function toDateAndTimeInputs(value: string | null): { date: string; time: string } {
  const parsed = parseTime(value)
  if (!parsed) return { date: '', time: '' }
  const local = toLocalInput(parsed)
  return { date: local.slice(0, 10), time: local.slice(11, 16) }
}

/** 由日期 + 时刻两个原生选择器的值合成 RFC3339 UTC 字符串。 */
export function fromDateAndTimeInputs(date: string, time: string): string | null {
  if (!date || !time) return null
  const [year, month, day] = date.split('-').map(Number)
  const [hour, minute] = time.split(':').map(Number)
  const local = new Date(year, month - 1, day, hour, minute, 0, 0)
  return Number.isNaN(local.getTime()) ? null : local.toISOString()
}

/** 相对今天的天数差（本地时区，按自然日计算）。 */
function dayOffsetFromToday(date: Date): number {
  const startOfTarget = new Date(date.getFullYear(), date.getMonth(), date.getDate())
  const now = new Date()
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate())
  return Math.round((startOfTarget.getTime() - startOfToday.getTime()) / MS_PER_DAY)
}

/** 两位补零。 */
function pad2(value: number): string {
  return String(value).padStart(2, '0')
}

/** HH:mm。 */
export function formatClock(value: string): string {
  const date = parseTime(value)
  return date ? `${pad2(date.getHours())}:${pad2(date.getMinutes())}` : ''
}

/**
 * 完成时间徽章文案（需求 2.2）：
 * 当天显示「今天 HH:mm」，其余显示「M/D HH:mm」；昨天/明天额外用自然语言，
 * 便于一眼分辨逾期与临期。
 */
export function formatDueLabel(value: string): string {
  const date = parseTime(value)
  if (!date) return ''

  const clock = `${pad2(date.getHours())}:${pad2(date.getMinutes())}`
  switch (dayOffsetFromToday(date)) {
    case 0:
      return `今天 ${clock}`
    case 1:
      return `明天 ${clock}`
    case -1:
      return `昨天 ${clock}`
    default:
      return `${date.getMonth() + 1}/${date.getDate()} ${clock}`
  }
}

/**
 * 提醒徽章文案（需求 2.2）：与完成时间同一天时只显示时刻，否则带日期。
 */
export function formatRemindLabel(remindAt: string, dueAt: string): string {
  const remind = parseTime(remindAt)
  if (!remind) return ''
  const due = parseTime(dueAt)

  const clock = `${pad2(remind.getHours())}:${pad2(remind.getMinutes())}`
  if (due && toDateKey(remind) === toDateKey(due)) return clock
  return `${remind.getMonth() + 1}/${remind.getDate()} ${clock}`
}

/** 卡片元数据行的时间戳：今天只显示时刻，其余带月日。 */
export function formatStamp(value: string): string {
  const date = parseTime(value)
  if (!date) return ''
  const clock = `${pad2(date.getHours())}:${pad2(date.getMinutes())}`
  const offset = dayOffsetFromToday(date)
  if (offset === 0) return `今天 ${clock}`
  if (offset === -1) return `昨天 ${clock}`
  return `${date.getMonth() + 1}/${date.getDate()} ${clock}`
}

/** 日期键的展示文案，用于归档页日期切换条。 */
export function formatDateKeyLabel(key: string): string {
  const [year, month, day] = key.split('-').map(Number)
  const date = new Date(year, month - 1, day)
  const weekday = '日一二三四五六'[date.getDay()]
  const offset = dayOffsetFromToday(date)
  const suffix = offset === 0 ? '（今天）' : offset === -1 ? '（昨天）' : offset === 1 ? '（明天）' : ''
  return `${year}年${month}月${day}日 周${weekday}${suffix}`
}
