import { createUid, nodeRichTextToTextWithWrap, textToNodeRichTextWithWrap } from 'simple-mind-map/src/utils/index.js'
import { nextTick, reactive, ref, type Ref } from 'vue'
import { logger } from '@/service/logger'
import { requireMindMap, type MindMapContext } from './useMindMap'
import type { DropPosition, OutlineController, OutlineNode } from '../sidebars/OutlineTree.vue'

/**
 * 大纲树协调器（移植参考端 Outline.vue 逻辑），抽成组合式函数供大纲侧栏与全屏大纲编辑对话框共用。
 *
 * 封装：从 mindMap 数据构建大纲树、编辑/插入/拖拽/删除逻辑、与库数据变更联动的守卫，
 * 以及 data_change / node_tree_render_end 订阅与 window keydown（删除）监听。
 * 消费方在 setup 里调用一次拿到 { treeData, controller, isInTreeArea, refresh, subscribe, unsubscribe }，
 * onMounted 调 subscribe、onBeforeUnmount 调 unsubscribe，并把 controller provide 给递归行组件 OutlineTree。
 *
 * 说明：侧栏与对话框各持一个独立实例，用户交互只作用于各自 DOM 的 contenteditable，
 * 对库的命令互不冲突；两实例的 data_change 各自刷新自己的树，展示同一份数据。
 */
export interface OutlineTreeApi {
  treeData: Ref<OutlineNode[]>
  controller: OutlineController
  isInTreeArea: Ref<boolean>
  refresh: () => void
  subscribe: () => void
  unsubscribe: () => void
}

/** HTML 转义（参考端库 htmlEscape 未在类型声明内，此处本地实现）。 */
function htmlEscape(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

export function useOutlineTree(ctx: MindMapContext): OutlineTreeApi {
  const { bus, ui } = ctx

  const treeData = ref<OutlineNode[]>([])
  const expandedMap = reactive<Record<string, boolean>>({})
  const currentUid = ref('')
  const renderKey = ref(0)
  const isInTreeArea = ref(false)

  // 与库数据变更联动的守卫（参考端 notHandleDataChange / isAfterCreateNewNode）。
  let notHandleDataChange = false
  let isAfterCreateNewNode = false
  let isHandleNodeTreeRenderEnd = false
  let pendingInsert: 'insertNode' | 'insertChildNode' | 'moveUp' | '' = ''
  let beInsertNodeUid = ''

  function buildNode(raw: Record<string, unknown>, isRoot: boolean): OutlineNode {
    const data = (raw.data ?? {}) as Record<string, unknown>
    const richText = !!data.richText
    const rawText = richText ? nodeRichTextToTextWithWrap(String(data.text ?? '')) : String(data.text ?? '')
    const label = htmlEscape(rawText).replace(/\n/g, '<br>')
    const children = ((raw.children as Record<string, unknown>[]) ?? []).map((child) => buildNode(child, false))
    return { uid: String(data.uid ?? ''), label, textCache: label, richText, isRoot, children }
  }

  function refresh(): void {
    const data = requireMindMap(ctx).getData(false) as Record<string, unknown>
    const root = buildNode(data, true)
    const seed = (node: OutlineNode): void => {
      if (expandedMap[node.uid] === undefined) expandedMap[node.uid] = true
      node.children.forEach(seed)
    }
    seed(root)
    treeData.value = [root]
    renderKey.value += 1
  }

  function handleDataChange(): void {
    if (notHandleDataChange) {
      notHandleDataChange = false
      isAfterCreateNewNode = false
      return
    }
    if (isAfterCreateNewNode) {
      isAfterCreateNewNode = false
      return
    }
    refresh()
  }

  function handleNodeTreeRenderEnd(): void {
    if (pendingInsert) {
      runInsert(pendingInsert)
      pendingInsert = ''
      return
    }
    if (isHandleNodeTreeRenderEnd) {
      isHandleNodeTreeRenderEnd = false
      refresh()
      void nextTick(() => afterCreateNewNode())
    }
  }

  function runInsert(type: 'insertNode' | 'insertChildNode' | 'moveUp'): void {
    const mindMap = requireMindMap(ctx)
    if (type === 'moveUp') {
      mindMap.execCommand('MOVE_UP_ONE_LEVEL')
      return
    }
    notHandleDataChange = true
    isHandleNodeTreeRenderEnd = true
    beInsertNodeUid = createUid()
    const command = type === 'insertNode' ? 'INSERT_NODE' : 'INSERT_CHILD_NODE'
    mindMap.execCommand(command, false, [], { uid: beInsertNodeUid })
  }

  function afterCreateNewNode(): void {
    const id = beInsertNodeUid
    beInsertNodeUid = ''
    if (!id) return
    isAfterCreateNewNode = true
    currentUid.value = id
    clickNode(id)
    const el = document.querySelector<HTMLElement>(`.mm-outline-edit[data-uid="${id}"]`)
    if (!el) return
    const selection = window.getSelection()
    const range = document.createRange()
    range.selectNodeContents(el)
    selection?.removeAllRanges()
    selection?.addRange(range)
    el.focus()
  }

  function readonly(): boolean {
    return ui.isReadonly
  }
  function isExpanded(uid: string): boolean {
    return expandedMap[uid] !== false
  }
  function toggleExpand(uid: string): void {
    expandedMap[uid] = !isExpanded(uid)
  }

  function clickNode(uid: string): void {
    const mindMap = requireMindMap(ctx)
    currentUid.value = uid
    const target = mindMap.renderer.findNodeByUid(uid)
    if (target && target.nodeData?.data?.isActive) return
    notHandleDataChange = true
    mindMap.execCommand('GO_TARGET_NODE', uid, () => {
      notHandleDataChange = false
    })
  }

  function blurNode(node: OutlineNode, el: HTMLElement): void {
    if (node.textCache === el.innerHTML) {
      if (pendingInsert) {
        runInsert(pendingInsert)
        pendingInsert = ''
      }
      return
    }
    const mindMap = requireMindMap(ctx)
    const target = mindMap.renderer.findNodeByUid(node.uid)
    if (!target) return
    notHandleDataChange = true
    if (node.richText) {
      target.setText(textToNodeRichTextWithWrap(el.innerHTML), true)
    } else {
      target.setText(el.innerText)
    }
    logger.info('mindmap', '大纲编辑更新节点文本')
  }

  function keydownNode(event: KeyboardEvent, el: HTMLElement): void {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      pendingInsert = 'insertNode'
      el.blur()
    } else if (event.key === 'Tab') {
      event.preventDefault()
      pendingInsert = event.shiftKey ? 'moveUp' : 'insertChildNode'
      el.blur()
    }
  }

  function pasteNode(event: ClipboardEvent): void {
    event.preventDefault()
    const text = event.clipboardData?.getData('text/plain') ?? ''
    const selection = window.getSelection()
    if (!selection || selection.rangeCount === 0) return
    const range = selection.getRangeAt(0)
    range.deleteContents()
    range.insertNode(document.createTextNode(text))
    range.collapse(false)
  }

  const dragState = reactive<{ draggingUid: string; overUid: string; position: DropPosition | '' }>({
    draggingUid: '',
    overUid: '',
    position: '',
  })

  function dragStart(uid: string): void {
    dragState.draggingUid = uid
    ui.isDragOutlineTreeNode = true
  }
  function dragOverRow(uid: string, event: DragEvent): void {
    if (!dragState.draggingUid || uid === dragState.draggingUid) {
      dragState.overUid = ''
      dragState.position = ''
      return
    }
    const row = event.currentTarget as HTMLElement
    const rect = row.getBoundingClientRect()
    const offset = event.clientY - rect.top
    dragState.overUid = uid
    if (offset < rect.height * 0.25) dragState.position = 'before'
    else if (offset > rect.height * 0.75) dragState.position = 'after'
    else dragState.position = 'inner'
  }
  function dropRow(uid: string): void {
    const { draggingUid, position } = dragState
    if (!draggingUid || !position || uid === draggingUid) return
    const mindMap = requireMindMap(ctx)
    const node = mindMap.renderer.findNodeByUid(draggingUid)
    const targetNode = mindMap.renderer.findNodeByUid(uid)
    if (!node || !targetNode) return
    notHandleDataChange = true
    const command = position === 'before' ? 'INSERT_BEFORE' : position === 'after' ? 'INSERT_AFTER' : 'MOVE_NODE_TO'
    mindMap.execCommand(command, node, targetNode)
    logger.info('mindmap', `大纲拖拽 ${position} 目标 ${uid}`)
    dragEnd()
  }
  function dragEnd(): void {
    dragState.draggingUid = ''
    dragState.overUid = ''
    dragState.position = ''
    ui.isDragOutlineTreeNode = false
  }

  function onWindowKeydown(event: KeyboardEvent): void {
    if (!isInTreeArea.value) return
    if ((event.key === 'Delete' || event.key === 'Backspace') && currentUid.value) {
      const mindMap = requireMindMap(ctx)
      const node = mindMap.renderer.findNodeByUid(currentUid.value)
      if (node && !node.isRoot) {
        event.stopPropagation()
        mindMap.renderer.textEdit?.hideEditTextBox?.()
        notHandleDataChange = true
        mindMap.execCommand('REMOVE_NODE', [node])
      }
    }
  }

  const controller: OutlineController = {
    readonly,
    currentUid,
    renderKey,
    isExpanded,
    toggleExpand,
    clickNode,
    blurNode,
    keydownNode,
    pasteNode,
    dragState,
    dragStart,
    dragOverRow,
    dropRow,
    dragEnd,
  }

  const offs: Array<() => void> = []
  function subscribe(): void {
    refresh()
    window.addEventListener('keydown', onWindowKeydown)
    offs.push(bus.on('data_change', handleDataChange), bus.on('node_tree_render_end', handleNodeTreeRenderEnd))
  }
  function unsubscribe(): void {
    window.removeEventListener('keydown', onWindowKeydown)
    offs.forEach((off) => off())
    offs.length = 0
  }

  return { treeData, controller, isInTreeArea, refresh, subscribe, unsubscribe }
}
