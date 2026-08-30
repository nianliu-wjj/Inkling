import type { CaptureMode } from '@/typings/domain'

export interface CaptureModeOption {
  key: CaptureMode
  icon: string
  label: string
}

export const captureModes: CaptureModeOption[] = [
  { key: 'note', icon: '🔴', label: '笔记' },
  { key: 'clipboard', icon: '🟡', label: '剪贴板' },
  { key: 'todo', icon: '🟢', label: '待办' },
]
