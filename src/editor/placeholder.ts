import { Plugin } from 'prosemirror-state'
import { Decoration, DecorationSet } from 'prosemirror-view'
import { renderMarkdownInline } from '@/utils/format'

/**
 * Markdown 占位提示插件。占位提示是 widget，不是文档内容，
 * 因此不会污染 Markdown，也不会在用户输入后残留源码标记。
 *
 * 只在**默认段落**为空时提示：输入 `# ` 会被输入规则转成标题，此刻文档里是
 * 一个空的 heading，同样满足「单个空文本块」。若不区分节点类型，提示文字就会
 * 被渲染进 h1 并继承标题字号，显示成一行巨大的灰字。
 * 而且用户已经在写标题了，本就不该再提示「此刻在想什么」。
 */
export function placeholderPlugin(text: string): Plugin {
  return new Plugin({
    props: {
      decorations(state) {
        const { doc, schema } = state
        const first = doc.firstChild
        const isEmptyParagraph =
          doc.childCount === 1 && first?.type === schema.nodes.paragraph && first.content.size === 0
        if (!isEmptyParagraph) return null

        return DecorationSet.create(doc, [
          Decoration.widget(
            1,
            () => {
              const element = document.createElement('span')
              element.className = 'editor-placeholder'
              element.setAttribute('aria-hidden', 'true')
              element.innerHTML = renderMarkdownInline(text)
              return element
            },
            { ignoreSelection: true },
          ),
        ])
      },
    },
  })
}
