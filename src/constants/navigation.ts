import type { View } from '@/typings/domain'

/**
 * 主窗口侧边栏页签（原型 #archiveSide .side-nav 的五项，顺序固定）。
 * 计数键为空的页签不显示徽章；分类色 --tint 由生成层 CSS 按 data-view 提供
 * （笔记蓝 / 粘贴板金 / 待办绿，启动台与灵动岛回退为强调色）。
 */
export interface NavigationItem {
  key: Extract<View, 'notes' | 'clips' | 'todos' | 'launcher' | 'island'>
  icon: string
  label: string
  countKey?: 'notes' | 'clips' | 'todos'
}

export const navigationItems: readonly NavigationItem[] = [
  { key: 'notes', icon: '📝', label: '笔记', countKey: 'notes' },
  { key: 'clips', icon: '📋', label: '粘贴板', countKey: 'clips' },
  { key: 'todos', icon: '✅', label: '待办', countKey: 'todos' },
  { key: 'launcher', icon: '🚀', label: '启动台' },
  { key: 'island', icon: '🏝️', label: '灵动岛' },
]
