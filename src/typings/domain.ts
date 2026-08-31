export type View = 'notes' | 'clips' | 'todos' | 'stats' | 'settings'
export type CaptureMode = 'note' | 'clipboard' | 'todo'
export type Priority = 'high' | 'medium' | 'low'

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

export interface TodoInput {
  id?: string
  content: string
  due_at: string
  remind_at?: string | null
  repeat_rule?: string | null
  priority: Priority
  remark: string
  tags: string[]
  parent_id?: string | null
  allow_past?: boolean
}

export interface Settings {
  collapse_policy: string
  clipboard_retention_days: number
  start_on_boot: boolean
  shortcut: string
  remark_style: string
  theme: string
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
