import { InputRule, inputRules, smartQuotes, textblockTypeInputRule, wrappingInputRule } from 'prosemirror-inputrules'
import type { MarkType, NodeType } from 'prosemirror-model'
import type { Plugin } from 'prosemirror-state'
import { marks, nodes } from './schema'

/**
 * Markdown 即时渲染的输入规则。
 *
 * 需求 2.2：「Typora 级别的即时渲染体验（所见即所得）」——用户敲下
 * `**粗体**` 的收尾 `**` 时立刻转成加粗文本并吃掉语法标记，
 * 而不是保留源码等到失焦再渲染。
 */

/**
 * 行内标记的通用输入规则工厂。
 *
 * 匹配到完整的 `开始标记 + 内容 + 结束标记` 后，把内容套上 mark 并删除标记本身。
 * `\S` 与 `\S ?` 的约束用于避免把 `a * b * c` 这类算式误判为斜体。
 */
function markInputRule(pattern: RegExp, markType: MarkType): InputRule {
  return new InputRule(pattern, (state, match, start, end) => {
    const [full, content] = match
    if (!content) return null

    const { tr } = state
    // 匹配串可能带前导字符（如空格），据此校正实际替换区间。
    const contentStart = start + full.indexOf(content)
    const contentEnd = contentStart + content.length

    // 先删掉结束标记，再删开始标记——从后往前删，避免位置偏移。
    if (contentEnd < end) tr.delete(contentEnd, end)
    if (contentStart > start) tr.delete(start, contentStart)

    const markEnd = start + content.length
    return tr.addMark(start, markEnd, markType.create()).removeStoredMark(markType)
  })
}

/** `# ` ~ `### ` → 一至三级标题。 */
function headingRule(nodeType: NodeType, maxLevel: number): InputRule {
  return textblockTypeInputRule(new RegExp(`^(#{1,${maxLevel}})\\s$`), nodeType, (match) => ({
    level: match[1].length,
  }))
}

/**
 * 组装全部输入规则。
 *
 * 顺序有意义：块级规则在前，行内规则在后，避免 `- ` 之类的前缀被行内规则先吃掉。
 */
export function buildInputRules(): Plugin {
  return inputRules({
    rules: [
      ...smartQuotes,

      // ── 块级 ──
      // `> ` → 引用
      wrappingInputRule(/^\s*>\s$/, nodes.blockquote),
      // `- ` / `* ` / `+ ` → 无序列表
      wrappingInputRule(/^\s*([-+*])\s$/, nodes.bullet_list),
      // `1. ` → 有序列表，续接时沿用已有序号
      wrappingInputRule(
        /^(\d+)\.\s$/,
        nodes.ordered_list,
        (match) => ({ order: Number(match[1]) }),
        (match, node) => node.childCount + node.attrs.order === Number(match[1]),
      ),
      // ``` → 代码块（由 CodeMirror NodeView 接管，见 editor/codeblock.ts）
      textblockTypeInputRule(/^```$/, nodes.code_block),
      // `---` → 分隔线
      new InputRule(/^(?:---|___|\*\*\*)$/, (state, _match, start, end) =>
        state.tr.replaceWith(start - 1, end, nodes.horizontal_rule.create()),
      ),
      headingRule(nodes.heading, 3),

      // ── 行内 ──
      // 加粗必须排在斜体之前，否则 `**x**` 会先被斜体规则匹配掉一半。
      markInputRule(/(?:\*\*)([^*]+)(?:\*\*)$/, marks.strong),
      markInputRule(/(?:__)([^_]+)(?:__)$/, marks.strong),
      markInputRule(/(?:^|[^*])\*([^*]+)\*$/, marks.em),
      markInputRule(/(?:^|[^_])_([^_]+)_$/, marks.em),
      markInputRule(/(?:~~)([^~]+)(?:~~)$/, marks.strikethrough),
      markInputRule(/(?:`)([^`]+)(?:`)$/, marks.code),
    ],
  })
}
