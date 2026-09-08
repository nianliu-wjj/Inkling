/**
 * 基础样式侧栏的纯函数（背景图上传相关）。
 * 从组件拆出以便独立测试；不依赖 Vue 组件实例。
 */

/** 选中的图片文件 → dataURL（参考端 ImgUpload 组件的 FileReader 流程）。 */
export function backgroundFileToDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = () => reject(reader.error ?? new Error('读取背景图文件失败'))
    reader.readAsDataURL(file)
  })
}
