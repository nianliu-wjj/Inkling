/**
 * 导图名规范化（原型文件名岛 `#mmFilenameInput` 的 `maxlength="32"` 口径）。
 *
 * 原型只在 HTML 上限了长度（`docs/index.html:335`），没有 JS 侧处理；空名回退
 * 「未命名导图」是原型 `openMindmapEditor` 里对根节点文本为空时的兜底
 * （`docs/app.js:1697-1698`）。放在这里是为了让「提交什么名字」这件事可单测。
 */

/** 名称长度上限（与原型 `maxlength="32"` 一致）。 */
export const MAP_NAME_LIMIT = 32

/** 空名兜底文案（原型同款）。 */
export const MAP_NAME_FALLBACK = '未命名导图'

/**
 * 规范化导图名：去首尾空白 → 内部连续空白折叠为单个空格 → 截断到 32 字符 → 空则回退。
 * 截断按 UTF-16 码元计（与浏览器 `maxlength` 的行为一致，中文与 emoji 都按原型来）。
 */
export function normalizeMapName(raw: string): string {
  const collapsed = raw.trim().replace(/\s+/g, ' ')
  const clipped = collapsed.slice(0, MAP_NAME_LIMIT)
  return clipped || MAP_NAME_FALLBACK
}
