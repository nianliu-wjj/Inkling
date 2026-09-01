export type View = 'notes' | 'clips' | 'todos' | 'stats' | 'settings'
export type CaptureMode = 'note' | 'clipboard' | 'todo'
export type Priority = 'high' | 'medium' | 'low'

/**
 * 备注展示样式。
 *
 * 取值以后端为准（domain/models.rs::Settings 注释「mixed / icon / text」），
 * 而非原型 HTML 里的 auto/line —— 前者是持久化契约。
 */
export type RemarkStyle = 'mixed' | 'icon' | 'text'

/** 面板失焦收起策略（domain/models.rs::Settings 注释「immediate / 3s / never」）。 */
export type CollapsePolicy = 'immediate' | '3s' | 'never'
export type PanelPosition = 'top' | 'bottom' | 'left' | 'right'

export interface Note {
  id: string
  content: string
  tags: string[]
  is_draft: boolean
  pinned: boolean
  archived_at: string | null
  created_at: string
  updated_at: string
}

export interface ClipboardEntry {
  id: string
  content_type: string
  content: string
  preview: string
  file_path?: string | null
  pinned: boolean
  copied_at: string
  modified_at: string
}

export interface Todo {
  id: string
  content: string
  due_at: string
  completed_at: string | null
  status: 'open' | 'done'
  remind_at: string | null
  repeat_rule: 'daily' | 'weekly' | string | null
  remind_off: boolean
  priority: Priority
  remark: string
  parent_id: string | null
  tags: string[]
  created_at: string
  updated_at: string
}

export interface NoteInput {
  id?: string
  content: string
  tags: string[]
  draft: boolean
}

/**
 * 待办保存载荷。
 *
 * 注意：字段必须是 camelCase —— 后端 `ipc.rs::TodoPayload` 带
 * `#[serde(rename_all = "camelCase")]`，用 snake_case 会在反序列化阶段直接失败。
 * 这与下方 `Todo` 等**响应**类型不同：响应模型定义在 `domain/models.rs`，
 * 未加 rename_all，因此保持 snake_case。
 */
export interface TodoInput {
  id?: string
  content: string
  dueAt: string
  remindAt?: string | null
  repeatRule?: string | null
  priority: Priority
  remark: string
  tags: string[]
  parentId?: string | null
  allowPast?: boolean
}

export interface Settings {
  collapse_policy: CollapsePolicy
  clipboard_retention_days: number
  start_on_boot: boolean
  shortcut: string
  remark_style: RemarkStyle
  theme: string
  /** 归档主窗口是否启用毛玻璃；关闭时退化为不透明实色。 */
  main_acrylic: boolean
  /** 面板从屏幕哪一侧的中间位置唤出。 */
  panel_position: PanelPosition
}

export interface ActivityDay {
  date: string
  notes: number
  clips: number
  todos: number
  completed: number
  overdue: number
}

export interface MonthTrend {
  month: string
  notes: number
  clips: number
  todos: number
  completed: number
}

export interface StatsSummary {
  notes: number
  clips: number
  todos: number
  completed: number
  overdue: number
}

export interface DayDetailItem {
  kind: 'note' | 'clip' | 'todo' | string
  time: string
  note?: Note
  clip?: ClipboardEntry
  todo?: Todo
}
