import { invoke } from '@tauri-apps/api/core'
import { openPath, openUrl } from '@tauri-apps/plugin-opener'
import type {
  ActivityDay,
  ClipboardEntry,
  DayDetailItem,
  MonthTrend,
  Note,
  NoteInput,
  Settings,
  StatsSummary,
  Todo,
  TodoInput,
} from '@/typings/domain'

export const api = {
  windows: {
    panelShow: () => invoke<void>('panel_show'),
    panelHide: () => invoke<void>('panel_hide'),
    panelResize: (height: number) => invoke<void>('panel_resize', { height }),
    showMain: (view: string) => invoke<void>('show_main', { view }),
    hideMain: () => invoke<void>('hide_main'),
    quit: () => invoke<void>('quit_app'),
    /** 切换归档主窗口毛玻璃（运行时可调，无需重建窗口）。 */
    setMainAcrylic: (enabled: boolean) => invoke<void>('set_main_acrylic', { enabled }),
    pinCreate: (kind: 'note' | 'todo' | 'clip', id: string) => invoke<void>('pin_create', { kind, id }),
    pinClose: (label: string) => invoke<void>('pin_close', { label }),
    pinSetEditing: (label: string, expanded: boolean) => invoke<void>('pin_set_editing', { label, expanded }),
    reminderClose: (todoId: string) => invoke<void>('reminder_close', { todoId }),
  },
  shortcut: {
    rebind: (combo: string) => invoke<string>('rebind_shortcut', { combo }),
  },
  notes: {
    list: () => invoke<Note[]>('notes_list'),
    draft: () => invoke<Note | null>('note_draft'),
    save: (input: NoteInput) => invoke<Note>('note_save', { input }),
    remove: (id: string) => invoke<void>('note_delete', { id }),
    pin: (id: string, pinned: boolean) => invoke<Note>('note_set_pinned', { id, pinned }),
  },
  clipboard: {
    list: () => invoke<ClipboardEntry[]>('clipboard_list'),
    capture: () => invoke<ClipboardEntry | null>('clipboard_capture'),
    /** 仅写回系统剪贴板，不触发粘贴动作。 */
    write: (id: string) => invoke<void>('clipboard_write', { id }),
    /**
     * 粘贴到光标处：写入剪贴板 → 收起面板交还焦点 → 模拟 Ctrl/Cmd+V。
     * 面板会在此过程中隐藏，焦点回到用户原本所在的应用。
     */
    paste: (id: string) => invoke<void>('clipboard_paste', { id }),
    update: (id: string, content: string) => invoke<ClipboardEntry>('clipboard_update', { id, content }),
    pin: (id: string, pinned: boolean) => invoke<void>('clipboard_pin', { id, pinned }),
    remove: (id: string) => invoke<void>('clipboard_delete', { id }),
    cleanup: () => invoke<number>('clipboard_cleanup'),
  },
  todos: {
    list: () => invoke<Todo[]>('todos_list'),
    save: (input: TodoInput) => invoke<Todo>('todo_save', { input }),
    complete: (id: string, completed: boolean) => invoke<Todo[]>('todo_complete', { id, completed }),
    priority: (id: string, priority: string) => invoke<Todo>('todo_priority', { id, priority }),
    due: (id: string, dueAt: string) => invoke<Todo>('todo_due', { id, dueAt }),
    reminder: (id: string, remindAt: string | null, repeatRule: string | null) =>
      invoke<Todo>('todo_reminder', { id, remindAt, repeatRule }),
    remove: (id: string) => invoke<void>('todo_delete', { id }),
    snooze: (id: string, minutes: number) => invoke<Todo>('todo_snooze', { id, minutes }),
    dismissReminder: (id: string) => invoke<void>('todo_dismiss_reminder', { id }),
  },
  settings: {
    get: () => invoke<Settings>('settings_get'),
    save: (settings: Settings) => invoke<void>('settings_save', { settings }),
  },
  stats: {
    heatmap: (days = 182) => invoke<ActivityDay[]>('stats_heatmap', { days }),
    trend: () => invoke<MonthTrend[]>('stats_trend'),
    summary: () => invoke<StatsSummary>('stats_summary'),
    day: (date: string) => invoke<DayDetailItem[]>('stats_day', { date }),
  },
  exportItems: (refs: string[], format: string, outputDir?: string | null) =>
    invoke<string>('export_items', { payload: { refs, format, outputDir: outputDir ?? null } }),
  dataDir: () => invoke<string>('data_dir'),

  /** 系统集成：走 tauri-plugin-opener，避免在 WebView 内直接导航。 */
  system: {
    /** 用默认浏览器打开链接（剪贴板 link 类型条目使用）。 */
    openUrl: (url: string) => openUrl(url),
    /** 用系统文件管理器打开目录（设置页「打开数据目录」使用）。 */
    openPath: (path: string) => openPath(path),
    dataDir: () => invoke<string>('data_dir'),
  },
}
