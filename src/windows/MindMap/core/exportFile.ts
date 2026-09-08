import { save } from '@tauri-apps/plugin-dialog'
import { api } from '@/service/tauri'
import { logger } from '@/service/logger'

/** 导出格式的保存框过滤器显示名。 */
const EXT_NAME: Record<string, string> = {
  smm: 'SMM 文件',
  json: 'JSON',
  png: 'PNG 图片',
  svg: 'SVG',
  pdf: 'PDF',
  md: 'Markdown',
  xmind: 'XMind',
  txt: '文本',
}

/**
 * 把库导出的 dataURL 经系统保存框写盘（Rust `write_file_base64`）。
 *
 * 库所有格式都经 readBlob→readAsDataURL 返回 `data:...;base64,...`，因此统一剥前缀取 base64。
 * 用户取消保存框返回 null。返回实际写入路径。
 */
export async function saveDataUrl(dataUrl: string, fileName: string, ext: string): Promise<string | null> {
  const base64 = dataUrl.replace(/^data:[^,]*,/, '')
  const path = await save({
    defaultPath: `${fileName}.${ext}`,
    filters: [{ name: EXT_NAME[ext] ?? ext, extensions: [ext] }],
  })
  if (!path) {
    logger.info('mindmap', '用户取消导出')
    return null
  }
  return api.files.writeBase64(path, base64)
}
