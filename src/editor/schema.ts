import { Schema } from 'prosemirror-model'
import { schema as markdownSchema } from 'prosemirror-markdown'

/**
 * 笔记编辑器的文档结构。
 *
 * 以 prosemirror-markdown 的标准 schema 为基础（doc / paragraph / heading /
 * blockquote / code_block / bullet_list / ordered_list / list_item /
 * horizontal_rule / hard_break / image，marks: em / strong / code / link），
 * 额外补一个 `strikethrough`（~~删除线~~），它不在标准 schema 内但常用。
 *
 * 不自造节点类型：所有节点都必须能被 prosemirror-markdown 的序列化器写回
 * Markdown，否则「归档 → 卡片渲染」会丢内容。
 */
export const noteSchema = new Schema({
  nodes: markdownSchema.spec.nodes,
  marks: markdownSchema.spec.marks.addToEnd('strikethrough', {
    parseDOM: [{ tag: 's' }, { tag: 'del' }, { style: 'text-decoration=line-through' }],
    toDOM: () => ['s', 0],
  }),
})

/** 便捷引用，避免调用方到处写 noteSchema.nodes.xxx。 */
export const nodes = noteSchema.nodes
export const marks = noteSchema.marks
