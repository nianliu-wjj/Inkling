import { defaultKeymap, indentWithTab } from '@codemirror/commands'
import { markdown } from '@codemirror/lang-markdown'
import { EditorState } from '@codemirror/state'
import { EditorView, keymap } from '@codemirror/view'
import { exitCode } from 'prosemirror-commands'
import type { Node as PMNode } from 'prosemirror-model'
import { Selection } from 'prosemirror-state'
import type { EditorView as PMEditorView, NodeView } from 'prosemirror-view'
import { logger } from '@/service/logger'

/**
 * 代码块 NodeView：在 ProseMirror 文档内嵌一个 CodeMirror 6 编辑器。
 *
 * 分工（本次设计确认的方案）：正文用 ProseMirror 做所见即所得，
 * 唯独代码块交给 CodeMirror，以获得缩进、括号匹配、多光标等源码编辑能力。
 *
 * 三个关键契约：
 * 1. `stopEvent` 返回 true —— 让 CodeMirror 独占键鼠事件，否则两个编辑器抢输入；
 * 2. `update` 同步外部变更（如撤销）到 CodeMirror，且要避免回环；
 * 3. 光标在 CodeMirror 首行按 ↑ / 末行按 ↓ 时，把焦点交还 ProseMirror。
 */
export class CodeBlockView implements NodeView {
  public readonly dom: HTMLElement
  private readonly cm: EditorView
  private node: PMNode
  private readonly view: PMEditorView
  private readonly getPos: () => number | undefined
  /** 正在把 CodeMirror 的变更写回 ProseMirror，期间忽略 update 回调，防止回环。 */
  private updating = false

  constructor(node: PMNode, view: PMEditorView, getPos: () => number | undefined) {
    this.node = node
    this.view = view
    this.getPos = getPos

    this.cm = new EditorView({
      state: EditorState.create({
        doc: node.textContent,
        extensions: [
          markdown(),
          keymap.of([...this.buildEscapeKeymap(), indentWithTab, ...defaultKeymap]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) this.forwardToProseMirror()
          }),
          // 不用 CodeMirror 自带主题：颜色统一由 CSS 变量提供，
          // 才能跟随 Inkling 的 30 套主题一起切换。
          EditorView.theme({ '&': { backgroundColor: 'transparent' } }),
        ],
      }),
    })

    this.dom = document.createElement('div')
    this.dom.className = 'pm-codeblock'
    this.dom.appendChild(this.cm.dom)

    logger.debug('codeblock', '代码块 NodeView 已挂载')
  }

  /** 光标越界时把焦点还给 ProseMirror，保证方向键能走出代码块。 */
  private buildEscapeKeymap() {
    return [
      {
        key: 'ArrowUp',
        run: () => this.maybeEscape('line', -1),
      },
      {
        key: 'ArrowLeft',
        run: () => this.maybeEscape('char', -1),
      },
      {
        key: 'ArrowDown',
        run: () => this.maybeEscape('line', 1),
      },
      {
        key: 'ArrowRight',
        run: () => this.maybeEscape('char', 1),
      },
      {
        // ⌃/⌘+Enter 直接跳出代码块，等价于 ProseMirror 的 exitCode。
        key: 'Mod-Enter',
        run: () => {
          if (!exitCode(this.view.state, this.view.dispatch)) return false
          this.view.focus()
          return true
        },
      },
    ]
  }

  /**
   * 判断光标是否已在边界；是则把选区移到代码块外的相邻位置。
   * unit=line 处理上下键，unit=char 处理左右键。
   */
  private maybeEscape(unit: 'line' | 'char', dir: -1 | 1): boolean {
    const { state } = this.cm
    const { main } = state.selection
    if (!main.empty) return false

    if (unit === 'line') {
      const line = state.doc.lineAt(main.head)
      const atBoundary = dir < 0 ? line.number === 1 : line.number === state.doc.lines
      if (!atBoundary) return false
    } else {
      const atBoundary = dir < 0 ? main.head === 0 : main.head === state.doc.length
      if (!atBoundary) return false
    }

    const pos = this.getPos()
    if (pos === undefined) return false

    // 目标位置：向上取代码块前一格，向下取代码块后一格。
    const target = dir < 0 ? pos - 1 : pos + this.node.nodeSize
    const selection = Selection.near(this.view.state.doc.resolve(target), dir)
    this.view.dispatch(this.view.state.tr.setSelection(selection).scrollIntoView())
    this.view.focus()
    return true
  }

  /** 把 CodeMirror 的文本变更写回 ProseMirror 文档。 */
  private forwardToProseMirror(): void {
    const pos = this.getPos()
    if (pos === undefined) return

    const text = this.cm.state.doc.toString()
    if (text === this.node.textContent) return

    this.updating = true
    try {
      const start = pos + 1
      const end = pos + this.node.nodeSize - 1
      const content = text ? this.view.state.schema.text(text) : null
      const tr = content ? this.view.state.tr.replaceWith(start, end, content) : this.view.state.tr.delete(start, end)
      this.view.dispatch(tr)
    } finally {
      this.updating = false
    }
  }

  /** ProseMirror 侧的变更（撤销/协同）同步进 CodeMirror。 */
  update(node: PMNode): boolean {
    if (node.type !== this.node.type) return false
    this.node = node
    if (this.updating) return true

    const next = node.textContent
    const current = this.cm.state.doc.toString()
    if (next === current) return true

    this.cm.dispatch({
      changes: { from: 0, to: current.length, insert: next },
    })
    return true
  }

  /** 选区落进代码块时把焦点交给 CodeMirror。 */
  setSelection(anchor: number, head: number): void {
    this.cm.focus()
    this.updating = true
    this.cm.dispatch({ selection: { anchor, head } })
    this.updating = false
  }

  /** 代码块内的事件一律由 CodeMirror 处理，ProseMirror 不插手。 */
  stopEvent(): boolean {
    return true
  }

  /** 代码块内容由 CodeMirror 渲染，禁止 ProseMirror 直接改 DOM。 */
  ignoreMutation(): boolean {
    return true
  }

  destroy(): void {
    logger.debug('codeblock', '代码块 NodeView 已销毁')
    this.cm.destroy()
  }
}

/** 供 EditorProps.nodeViews 注册使用。 */
export function codeBlockNodeView(node: PMNode, view: PMEditorView, getPos: () => number | undefined): NodeView {
  return new CodeBlockView(node, view, getPos)
}
