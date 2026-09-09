import { open } from '@tauri-apps/plugin-dialog'
import type { MindMapNode } from 'simple-mind-map'
import { api } from '@/service/tauri'
import { logger } from '@/service/logger'
import type { Bus } from '../core/bus'
import type { MindMapContext } from '../core/useMindMap'

/**
 * 节点附件选择器（移植参考端附件流程），非组件——由 MindMapApp 在实例就绪后调用一次，返回清理函数。
 *
 * - `selectAttachment(nodes)`：弹系统文件选择框（单选），为每个节点 `SET_NODE_ATTACHMENT(node, path, fileName)`；
 * - `node_attachmentClick(node)`：用系统默认程序打开 `attachmentUrl`；
 * - `node_attachmentContextmenu(node)`：确认后删除附件（`SET_NODE_ATTACHMENT(node, '', '')`）。
 *
 * 取真实文件路径必须用 Tauri 的 open（浏览器 file input 出于安全不给路径），路径存入 attachmentUrl 供后续打开。
 */
export function setupAttachmentPicker(ctx: MindMapContext): () => void {
  const bus: Bus = ctx.bus

  /** 选择文件并写入各节点附件。 */
  async function onSelectAttachment(nodes: MindMapNode[]): Promise<void> {
    const mindMap = ctx.mindMap.value
    if (!mindMap || !nodes || nodes.length === 0) return
    try {
      const selected = await open({ multiple: false, directory: false })
      if (!selected || typeof selected !== 'string') return
      const fileName = selected.split(/[/\\]/).pop() ?? selected
      nodes.forEach((node) => mindMap.execCommand('SET_NODE_ATTACHMENT', node, selected, fileName))
      logger.info('mindmap', `设置节点附件 ${fileName}，生效节点数=${nodes.length}`)
    } catch (error) {
      logger.error('mindmap', '选择附件失败', error)
    }
  }

  /** 点击附件图标：系统默认程序打开。 */
  async function onAttachmentClick(node: MindMapNode): Promise<void> {
    const url = node.getData('attachmentUrl') as string
    if (!url) return
    try {
      await api.system.openPath(url)
      logger.info('mindmap', `打开节点附件 ${url}`)
    } catch (error) {
      logger.error('mindmap', '打开附件失败', error)
    }
  }

  /** 右键附件：确认后删除。 */
  function onAttachmentContextmenu(node: MindMapNode): void {
    if (!node) return
    if (!window.confirm('确定删除该节点的附件吗？')) return
    ctx.mindMap.value?.execCommand('SET_NODE_ATTACHMENT', node, '', '')
    logger.info('mindmap', '删除节点附件')
  }

  const offs: Array<() => void> = [
    bus.on('selectAttachment', (nodes) => void onSelectAttachment(nodes as MindMapNode[])),
    bus.on('node_attachmentClick', (node) => void onAttachmentClick(node as MindMapNode)),
    bus.on('node_attachmentContextmenu', (node) => onAttachmentContextmenu(node as MindMapNode)),
  ]
  return () => offs.forEach((off) => off())
}
