import {
  defaultMarkdownParser,
  defaultMarkdownSerializer,
  MarkdownParser,
  MarkdownSerializer,
} from 'prosemirror-markdown'
import type { Node as PMNode } from 'prosemirror-model'
import { noteSchema } from './schema'

/**
 * ProseMirror ↔ Markdown 互转。
 *
 * 笔记正文在 SQLite 里以 Markdown 原文存储（不是 JSON 文档），
 * 这样归档卡片可以直接用 markdown-it 渲染，导出也无需转换。
 * 因此编辑器每次保存都要序列化回 Markdown。
 */

/** 序列化器：在默认规则上补 strikethrough。 */
const serializer = new MarkdownSerializer(
  {
    ...defaultMarkdownSerializer.nodes,
  },
  {
    ...defaultMarkdownSerializer.marks,
    strikethrough: {
      open: '~~',
      close: '~~',
      mixable: true,
      expelEnclosingWhitespace: true,
    },
  },
)

/**
 * 解析器：复用默认 tokens，并把 markdown-it 的 `s` token 映射到 strikethrough。
 * 默认解析器未开启 strikethrough 插件，此处显式启用。
 */
const parser = new MarkdownParser(noteSchema, defaultMarkdownParser.tokenizer.enable('strikethrough'), {
  ...defaultMarkdownParser.tokens,
  s: { mark: 'strikethrough' },
})

/** Markdown 原文 → ProseMirror 文档。 */
export function parseMarkdown(source: string): PMNode {
  return parser.parse(source) ?? noteSchema.topNodeType.createAndFill()!
}

/** ProseMirror 文档 → Markdown 原文。 */
export function serializeMarkdown(doc: PMNode): string {
  return serializer.serialize(doc)
}
