import { Plugin } from 'prosemirror-state'
import { Decoration, DecorationSet } from 'prosemirror-view'
import { renderMarkdownInline } from '@/utils/format'

/**
 * Markdown 占位提示插件。占位提示是 widget，不是文档内容，
 * 因此不会污染 Markdown，也不会在用户输入后残留源码标记。
 */
export function placeholderPlugin(text: string): Plugin {
  return new Plugin({
    props: {
      decorations(state) {
        const { doc } = state
        const isEmpty = doc.childCount === 1 && doc.firstChild?.isTextblock && doc.firstChild.content.size === 0
        if (!isEmpty) return null

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
