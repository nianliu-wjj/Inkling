import { ref, type Ref } from 'vue'
import { useToast } from './useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/** 支持的导出格式（后端 services/export.rs 已实现）。 */
export type ExportFormat = 'markdown' | 'json'

/** 导出引用：`note:id` / `clip:id` / `todo:id`。 */
export type ExportKind = 'note' | 'clip' | 'todo'

/**
 * 导出。
 *
 * 需求 2.6 / P2：支持单条或批量导出。原型没有设计导出界面，
 * 按本次确认的方案落在两处：卡片悬浮功能区（单条）与归档页头部（批量）。
 *
 * 输出目录留空时由后端落到应用数据目录，避免弹系统对话框打断流程。
 */
export function useExport(): {
  exporting: Ref<boolean>
  exportOne: (kind: ExportKind, id: string, format?: ExportFormat) => Promise<void>
  exportMany: (kind: ExportKind, ids: readonly string[], format?: ExportFormat) => Promise<void>
} {
  const { toast } = useToast()
  const exporting = ref(false)

  async function run(refs: string[], format: ExportFormat): Promise<void> {
    if (!refs.length) {
      toast('没有可导出的内容')
      return
    }
    if (exporting.value) return

    exporting.value = true
    logger.info('export', `导出 ${refs.length} 条，格式 ${format}`)
    try {
      const path = await api.exportItems(refs, format, null)
      toast(`已导出 ${refs.length} 条到 ${path}`)
    } catch (error) {
      logger.error('export', '导出失败', error)
      toast(`导出失败：${String(error)}`)
    } finally {
      exporting.value = false
    }
  }

  return {
    exporting,
    exportOne: (kind, id, format = 'markdown') => run([`${kind}:${id}`], format),
    exportMany: (kind, ids, format = 'markdown') =>
      run(
        ids.map((id) => `${kind}:${id}`),
        format,
      ),
  }
}
