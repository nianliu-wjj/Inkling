/**
 * 提醒偏移档位。
 *
 * 与后端 `domain::todo::REMIND_OFFSETS` 一一对应，两侧都以此为准；
 * 后端会校验传入值是否在档位内，前端改动此处必须同步改后端常量。
 */
export const REMIND_OPTIONS: readonly { value: number | null; label: string }[] = [
  { value: null, label: '不提醒' },
  { value: 15, label: '前 15 分钟' },
  { value: 30, label: '前 30 分钟' },
  { value: 60, label: '前 1 小时' },
  { value: 180, label: '前 3 小时' },
  { value: 360, label: '前 6 小时' },
  { value: 1440, label: '前 1 天' },
]

/** 新建待办时的默认偏移。 */
export const DEFAULT_REMIND_OFFSET = 15

/** 把偏移分钟数转成显示文案，未命中档位时回退为「不提醒」。 */
export function remindOffsetLabel(minutes: number | null): string {
  return REMIND_OPTIONS.find((option) => option.value === minutes)?.label ?? '不提醒'
}
