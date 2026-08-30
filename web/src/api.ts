import { invoke } from '@tauri-apps/api/core'
import type { ActivityDay, ClipboardEntry, Note, Settings, Todo, TodoInput, NoteInput } from './types'

export const api = {
  notes: {
    list: () => invoke<Note[]>('list_notes'),
    save: (input: NoteInput) => invoke<Note>('save_note', { input }),
    remove: (id: string) => invoke<void>('delete_note', { id }),
  },
  clipboard: {
    list: () => invoke<ClipboardEntry[]>('list_clipboard'),
    capture: (content: string, contentType = 'text') => invoke<ClipboardEntry>('save_clipboard', { content, contentType }),
    update: (id: string, content: string) => invoke<void>('update_clipboard', { id, content }),
    pin: (id: string, pinned: boolean) => invoke<void>('set_clipboard_pinned', { id, pinned }),
    remove: (id: string) => invoke<void>('delete_clipboard', { id }),
  },
  todos: {
    list: () => invoke<Todo[]>('list_todos'),
    save: (input: TodoInput) => invoke<Todo>('save_todo', { input }),
    complete: (id: string, completed: boolean) => invoke<Todo[]>('complete_todo', { id, completed }),
    child: (parentId: string, content: string, dueAt: string) => invoke<Todo>('create_child_todo', { parentId, content, dueAt }),
    priority: (id: string, priority: string) => invoke<void>('set_todo_priority', { id, priority }),
    remove: (id: string) => invoke<void>('delete_todo', { id }),
  },
  settings: {
    get: () => invoke<Settings>('get_settings'),
    save: (settings: Settings) => invoke<void>('save_settings', { settings }),
  },
  activity: () => invoke<ActivityDay[]>('get_activity'),
}
