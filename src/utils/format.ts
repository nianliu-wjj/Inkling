import MarkdownIt from 'markdown-it'

/**
 * Markdown 渲染。
 *
 * 用于归档卡片的正文展示（需求 2.2：卡片正文按 Markdown 渲染，不展示源码标记）。
 * 编辑态的所见即所得由 ProseMirror 负责，与此处无关。
 *
 * 安全：`html: false` 禁止原始 HTML 直通——笔记内容来自用户本地输入，
 * 但剪贴板内容可能来自任意来源，一律不信任。
 */
const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
})

/** 渲染完整 Markdown（块级）。 */
export function renderMarkdown(source: string): string {
  return md.render(source)
}

/** 渲染为行内内容，用于卡片摘要，避免外层多一层 <p> 影响两行截断。 */
export function renderMarkdownInline(source: string): string {
  return md.renderInline(source)
}

// 日期时间函数统一收敛到 utils/datetime.ts，此处再导出以兼容既有调用点。
export {
  formatClock,
  formatDateKeyLabel,
  formatDueLabel,
  formatRemindLabel,
  formatStamp,
  fromDateAndTimeInputs,
  toDateAndTimeInputs,
  toDateKey,
  toLocalInput,
} from './datetime'

/** 兼容别名：旧代码使用的名字。 */
export { formatStamp as formatTime, formatDueLabel as formatDue } from './datetime'
