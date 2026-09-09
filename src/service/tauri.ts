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

/** 启动器命中项（对应 Rust services::launcher::model::Hit）。 */
export interface LauncherHit {
  id: number
  kind: 'app' | 'uwp' | 'file' | 'folder' | 'command'
  name: string
  path: string
  score: number
  matched_keyword: string
}

/** 启动器索引状态。 */
export interface LauncherStatus {
  count: number
  generation: number
  created_at: number
  rebuilding: boolean
}

export const api = {
  windows: {
    panelShow: () => invoke<void>('panel_show'),
    panelHide: () => invoke<void>('panel_hide'),
    panelResize: (height: number) => invoke<void>('panel_resize', { height }),
    /** 打开独立编辑窗口（全屏遮罩 + 居中对话框），payload 为序列化后的打开参数。 */
    editorOpen: (payload: string) => invoke<void>('editor_open', { payload }),
    editorClose: () => invoke<void>('editor_close'),
    /** 编辑窗口挂载后拉取本次打开参数。 */
    editorPayload: () => invoke<string | null>('editor_payload'),
    /** 编辑窗口内容渲染完成，请求显示（窗口以隐藏方式创建，避免空白遮罩闪一下）。 */
    editorReady: () => invoke<void>('editor_ready'),
    /** 打开思维导图窗口；noteId 省略表示新建。 */
    mindmapOpen: (noteId?: string) => invoke<void>('mindmap_open', { noteId: noteId ?? null }),
    /** 思维导图窗口挂载时按自己的 label 拉取打开参数。 */
    mindmapPayload: (label: string) => invoke<string | null>('mindmap_payload', { label }),
    mindmapClose: (label: string) => invoke<void>('mindmap_close', { label }),
    showMain: (view: string) => invoke<void>('show_main', { view }),
    hideMain: () => invoke<void>('hide_main'),
    minimizeMain: () => invoke<void>('minimize_main'),
    /** 切换最大化，返回切换后的状态。 */
    toggleMaximizeMain: () => invoke<boolean>('toggle_maximize_main'),
    mainIsMaximized: () => invoke<boolean>('main_is_maximized'),
    quit: () => invoke<void>('quit_app'),
    /** 切换归档主窗口毛玻璃（运行时可调，无需重建窗口）。 */
    setMainAcrylic: (enabled: boolean) => invoke<void>('set_main_acrylic', { enabled }),
    pinCreate: (kind: 'note' | 'todo' | 'clip', id: string) => invoke<void>('pin_create', { kind, id }),
    pinClose: (label: string) => invoke<void>('pin_close', { label }),
    pinSetEditing: (label: string, expanded: boolean) => invoke<void>('pin_set_editing', { label, expanded }),
    reminderClose: (todoId: string) => invoke<void>('reminder_close', { todoId }),
    /** 面板显示后取走「本次应切到的插件页」（灵动岛点击写入），无则 null。 */
    panelTakePage: () => invoke<string | null>('panel_take_page'),
  },
  /** 灵动岛。 */
  island: {
    /** 悬停展开 / 收起（只改窗口高度）。 */
    expand: (expanded: boolean) => invoke<void>('island_expand', { expanded }),
  },

  /** 启动器搜索。 */
  launcher: {
    /** 搜索，返回 Top-K 命中。 */
    search: (query: string) => invoke<LauncherHit[]>('launcher_search', { query }),
    /** 启动候选。mode：open / admin / reveal；query 用于记录查询亲和度。 */
    launch: (path: string, kind: LauncherHit['kind'], mode: 'open' | 'admin' | 'reveal', query: string) =>
      invoke<void>('launcher_launch', { path, kind, mode, query }),
    /** 立即重建索引（后台）。 */
    rebuild: () => invoke<void>('launcher_rebuild'),
    /** 索引状态。 */
    status: () => invoke<LauncherStatus>('launcher_status'),
    /** 隐藏搜索窗口。 */
    hide: () => invoke<void>('launcher_hide'),
    /** 改绑启动器全局快捷键。 */
    rebindShortcut: (combo: string) => invoke<string>('rebind_launcher_shortcut', { combo }),
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
    reminder: (id: string, offsetMinutes: number | null, desktop: boolean, email: boolean, repeatRule: string | null) =>
      invoke<Todo>('todo_reminder', { id, offsetMinutes, desktop, email, repeatRule }),
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

  /** 文件落盘（思维导图导出）。 */
  files: {
    /** 把 base64 内容写到绝对路径，返回写入的路径。 */
    writeBase64: (path: string, base64: string) => invoke<string>('write_file_base64', { path, base64 }),
  },

  /** 邮件提醒。 */
  mail: {
    /** 发送测试邮件验证 SMTP 配置，失败时错误信息原样抛出给界面。 */
    test: () => invoke<void>('mail_test'),
  },

  /** 系统集成：走 tauri-plugin-opener，避免在 WebView 内直接导航。 */
  system: {
    /** 用默认浏览器打开链接（剪贴板 link 类型条目使用）。 */
    openUrl: (url: string) => openUrl(url),
    /** 用系统文件管理器打开目录（设置页「打开数据目录」使用）。 */
    openPath: (path: string) => openPath(path),
    dataDir: () => invoke<string>('data_dir'),
  },
}
