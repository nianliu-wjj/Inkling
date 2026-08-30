import type { View } from '@/typings/domain'

export interface NavigationItem {
  key: View
  icon: string
  label: string
}

export const navigationItems: NavigationItem[] = [
  { key: 'notes', icon: '📝', label: '笔记' },
  { key: 'clips', icon: '📋', label: '剪贴板' },
  { key: 'todos', icon: '✅', label: '待办' },
  { key: 'stats', icon: '📊', label: '统计' },
]
