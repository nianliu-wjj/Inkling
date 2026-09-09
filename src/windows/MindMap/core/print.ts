import { logger } from '@/service/logger'

/**
 * 通过隐藏 iframe 打印一段 HTML（移植参考端 printOutline 思路）。
 * 用于大纲全屏编辑的「打印」：把大纲树转成层级 HTML 后交给浏览器打印。
 * 打印完成或失败后移除 iframe，避免残留。
 */
export function printHtml(bodyHtml: string, title = '打印'): void {
  try {
    const iframe = document.createElement('iframe')
    iframe.style.position = 'fixed'
    iframe.style.right = '0'
    iframe.style.bottom = '0'
    iframe.style.width = '0'
    iframe.style.height = '0'
    iframe.style.border = '0'
    document.body.appendChild(iframe)

    const doc = iframe.contentWindow?.document
    if (!doc) {
      document.body.removeChild(iframe)
      logger.error('mindmap', '打印失败：无法获取 iframe 文档')
      return
    }
    doc.open()
    doc.write(
      `<!doctype html><html><head><meta charset="utf-8"><title>${title}</title>` +
        `<style>body{font-family:-apple-system,'PingFang SC','Microsoft YaHei',sans-serif;padding:24px;color:#1a1a1a;}` +
        `ul{list-style:disc;padding-left:22px;margin:4px 0;}li{margin:3px 0;line-height:1.6;}</style>` +
        `</head><body>${bodyHtml}</body></html>`,
    )
    doc.close()

    const win = iframe.contentWindow
    const cleanup = (): void => {
      // 延迟移除，确保打印对话框已接管内容。
      setTimeout(() => {
        if (iframe.parentNode) iframe.parentNode.removeChild(iframe)
      }, 500)
    }
    if (win) {
      win.focus()
      win.onafterprint = cleanup
      win.print()
      // 部分环境不触发 onafterprint，兜底清理。
      setTimeout(cleanup, 60000)
    } else {
      cleanup()
    }
    logger.info('mindmap', '打印大纲')
  } catch (error) {
    logger.error('mindmap', '打印大纲失败', error)
  }
}
