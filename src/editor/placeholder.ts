import { Plugin } from 'prosemirror-state'
import { Decoration, DecorationSet } from 'prosemirror-view'

/**
 * 占位提示插件。
 *
 * 原型的 `.editor:empty::before { content: attr(data-placeholder) }` 在
 * ProseMirror 下永远不生效——空文档也会渲染出 `<p><br></p>`，`:empty` 匹配不到。
 * 因此改为：文档为空时给第一个段落挂 `.is-empty` 类并写入 data-placeholder，
 * 由 CSS 的 ::before 读取该属性渲染提示文字。
 */
export function placeholderPlugin(text: string): Plugin {
  return new Plugin({
    props: {
      decorations(state) {
        const { doc } = state
        // 判空：只有一个空段落即视为空文档。
        const isEmpty = doc.childCount === 1 && doc.firstChild?.isTextblock && doc.firstChild.content.size === 0
        if (!isEmpty) return null

        // 文案直接写进 decoration 的 data 属性：CSS 的 attr() 只能读取
        // 元素自身的属性，读不到宿主 .editor 上的 data-placeholder。
        return DecorationSet.create(doc, [
          Decoration.node(0, doc.firstChild!.nodeSize, {
            class: 'is-empty',
            'data-placeholder': text,
          }),
        ])
      },
    },
  })
}
