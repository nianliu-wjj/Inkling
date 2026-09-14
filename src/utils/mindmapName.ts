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
 * 规范化导图名：去首尾空白 → 内部连续空白折叠为单个空格 → 截断到 32 字符 → 再 trim → 空则回退。
 * 截断按 UTF-16 码元计（与浏览器 `maxlength` 的行为一致，中文与 emoji 都按原型来）。
 *
 * 截断之后再补一次 trim：截断点可能正好落在折叠后剩下的那个空格上（如 31 个 `a` + 两个空格 + `b`，
 * 第 32 位是空格），若不补 trim 就会返回以空白结尾的名字，违反本函数「去首尾空白」的口径。
 * 注意不能反过来把 trim 挪到截断之后——那样输入以空白开头时会白吃一个名额。
 */
export function normalizeMapName(raw: string): string {
  const collapsed = raw.trim().replace(/\s+/g, ' ')
  const clipped = collapsed.slice(0, MAP_NAME_LIMIT).trim()
  return clipped || MAP_NAME_FALLBACK
}
