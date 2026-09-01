import { ref, type Ref } from 'vue'
import { AppEvents, onAppEvent } from '@/service/events'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { ClipboardEntry, Note, Settings, Todo } from '@/typings/domain'

/**
 * 数据层。
 *
 * 四个窗口各自是独立的 Vue app，无法共享内存状态，因此统一走
 * 「IPC 拉取 + 后端事件驱动刷新」：任一窗口写数据后，Rust 侧广播
 * inkling://*-changed，其余窗口收到后重新拉取。
 *
 * 每个窗口内部用模块级 ref 做单例，避免同一窗口内多个组件各拉一份。
 */

/** 偏好设置的默认值，与 domain/models.rs::Settings::default 保持一致。 */
const DEFAULT_SETTINGS: Settings = {
  collapse_policy: '3s',
  clipboard_retention_days: 30,
  start_on_boot: false,
  shortcut: 'Ctrl+Shift+Space',
  remark_style: 'mixed',
  theme: 'dark',
  main_acrylic: true,
}

const notes = ref<Note[]>([])
const clips = ref<ClipboardEntry[]>([])
const todos = ref<Todo[]>([])
const settings = ref<Settings>({ ...DEFAULT_SETTINGS })

/** 各数据源是否已完成首次加载，避免重复注册事件监听。 */
const initialized = { notes: false, clips: false, todos: false, settings: false }

/** 统一的加载包装：记录耗时与异常，失败不抛出以免打断 UI。 */
async function load<T>(scope: string, fetcher: () => Promise<T>, target: Ref<T>): Promise<void> {
  try {
    const data = await fetcher()
    target.value = data
    logger.debug(scope, '数据加载完成')
  } catch (error) {
    logger.error(scope, '数据加载失败', error)
  }
}

export function useNotes(): { notes: Ref<Note[]>; reload: () => Promise<void> } {
  const reload = () => load('notes', api.notes.list, notes)

  if (!initialized.notes) {
    initialized.notes = true
    void reload()
    void onAppEvent(AppEvents.notesChanged, () => void reload())
  }

  return { notes, reload }
}

export function useClips(): { clips: Ref<ClipboardEntry[]>; reload: () => Promise<void> } {
  const reload = () => load('clips', api.clipboard.list, clips)

  if (!initialized.clips) {
    initialized.clips = true
    void reload()
    void onAppEvent(AppEvents.clipboardChanged, () => void reload())
  }

  return { clips, reload }
}

export function useTodos(): { todos: Ref<Todo[]>; reload: () => Promise<void> } {
  const reload = () => load('todos', api.todos.list, todos)

  if (!initialized.todos) {
    initialized.todos = true
    void reload()
    void onAppEvent(AppEvents.todosChanged, () => void reload())
  }

  return { todos, reload }
}

export function useSettings(): {
  settings: Ref<Settings>
  reload: () => Promise<void>
  save: (next: Settings) => Promise<void>
} {
  const reload = () => load('settings', api.settings.get, settings)

  async function save(next: Settings): Promise<void> {
    logger.info('settings', '保存偏好设置', next)
    try {
      await api.settings.save(next)
      settings.value = next
    } catch (error) {
      logger.error('settings', '保存偏好设置失败', error)
      throw error
    }
  }

  if (!initialized.settings) {
    initialized.settings = true
    void reload()
    // 其他窗口改了设置也要跟随（如主题、备注展示样式）。
    void onAppEvent<Settings>(AppEvents.settingsChanged, (payload) => {
      if (payload) settings.value = payload
    })
  }

  return { settings, reload, save }
}
