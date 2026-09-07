import { useDialog } from 'naive-ui'

/**
 * 二次确认封装：把 naive-ui 的 useDialog().warning 包成 Promise<boolean>，
 * 替代参考端 `this.$confirm(...).then/catch`。resolve true 表示用户点了确认。
 * 必须在 NDialogProvider 子树内调用（MindMapApp 已提供）。
 */
export function useConfirm(): (content: string, title?: string) => Promise<boolean> {
  const dialog = useDialog()
  return (content: string, title = '提示') =>
    new Promise<boolean>((resolve) => {
      dialog.warning({
        title,
        content,
        positiveText: '是',
        negativeText: '否',
        onPositiveClick: () => resolve(true),
        onNegativeClick: () => resolve(false),
        onClose: () => resolve(false),
        onMaskClick: () => resolve(false),
      })
    })
}
