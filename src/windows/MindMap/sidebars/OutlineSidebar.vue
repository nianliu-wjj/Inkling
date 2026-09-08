<script setup lang="ts">
import { createUid, nodeRichTextToTextWithWrap, textToNodeRichTextWithWrap } from 'simple-mind-map/src/utils/index.js'
import { nextTick, onBeforeUnmount, onMounted, provide, reactive, ref, watch } from 'vue'
import { logger } from '@/service/logger'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import SidebarShell from './SidebarShell.vue'
import OutlineTree, {
  OUTLINE_CONTROLLER,
  type DropPosition,
  type OutlineController,
  type OutlineNode,
} from './OutlineTree.vue'

/**
 * 大纲侧栏（移植参考端 Outline.vue + OutlineSidebar.vue）。
 *
 * 本组件是大纲树的协调器：从 mindMap 数据构建大纲树、订阅库事件刷新、承载编辑/插入/
 * 拖拽/删除的全部逻辑，并通过 provide 把控制器交给递归行组件 OutlineTree。
 * 顶部「全屏编辑」按钮置 `ui.isOutlineEdit`（全屏大纲编辑器为后续 Task 32）。
 *
 * 参考端用 element-ui 的 el-tree（内建拖拽/当前项/展开），本项目无等价组件，故自实现：
 * el-tree 的 node-drop(before/after/inner) → INSERT_BEFORE / INSERT_AFTER / MOVE_NODE_TO；
 * setCurrentKey / getNode 等由本地 currentUid + querySelector 复刻。
 */
const ctx = useMindMap()
const { bus, ui } = ctx

/** HTML 转义（参考端用库 htmlEscape，未在类型声明内，此处本地实现）。 */
function htmlEscape(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

// —— 树数据与展开状态 ——

const treeData = ref<OutlineNode[]>([])
/** 展开状态按 uid 记忆，刷新时保留（默认展开）。 */
const expandedMap = reactive<Record<string, boolean>>({})
const currentUid = ref('')
const renderKey = ref(0)

/**
 * 构建大纲树（参考端 refresh 的 walk）：richText 节点取纯文本换行版，转义后把 \n 换成 <br>。
 * textCache 存构建时文本，失焦时用它判断是否被改动。
 */
function buildNode(raw: Record<string, unknown>, isRoot: boolean): OutlineNode {
  const data = (raw.data ?? {}) as Record<string, unknown>
  const richText = !!data.richText
  const rawText = richText ? nodeRichTextToTextWithWrap(String(data.text ?? '')) : String(data.text ?? '')
  const label = htmlEscape(rawText).replace(/\n/g, '<br>')
  const children = ((raw.children as Record<string, unknown>[]) ?? []).map((child) => buildNode(child, false))
  return { uid: String(data.uid ?? ''), label, textCache: label, richText, isRoot, children }
}

/** 刷新整棵大纲树；保留已有展开状态，新节点默认展开。 */
function refresh(): void {
  const data = requireMindMap(ctx).getData(false) as Record<string, unknown>
  const root = buildNode(data, true)
  // 初始化展开状态：未记录过的 uid 默认展开。
  const seed = (node: OutlineNode): void => {
    if (expandedMap[node.uid] === undefined) expandedMap[node.uid] = true
    node.children.forEach(seed)
  }
  seed(root)
  treeData.value = [root]
  renderKey.value += 1
  logger.info('mindmap', '刷新大纲树')
}

// —— 与库数据变更的联动（参考端 notHandleDataChange / isAfterCreateNewNode 守卫）——

/** 在大纲内操作节点时置真，用于跳过由此引发的 data_change 刷新，避免打断编辑。 */
let notHandleDataChange = false
let isAfterCreateNewNode = false
/** 新节点插入后需在 node_tree_render_end 里刷新并聚焦。 */
let isHandleNodeTreeRenderEnd = false
/** 待执行的插入类型（编辑区按键先记录，失焦后执行）。 */
let pendingInsert: 'insertNode' | 'insertChildNode' | 'moveUp' | '' = ''
/** 新插入节点的预分配 uid，用于插入后聚焦。 */
let beInsertNodeUid = ''

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

// —— 插入 / 移动（参考端 insertNode / insertChildNode / moveUp）——

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

/** 插入新节点后：高亮、定位、聚焦其编辑框并全选（参考端 afterCreateNewNode）。 */
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

// —— 控制器实现（交给 OutlineTree 调用）——

function readonly(): boolean {
  return ui.isReadonly
}

function isExpanded(uid: string): boolean {
  return expandedMap[uid] !== false
}
function toggleExpand(uid: string): void {
  expandedMap[uid] = !isExpanded(uid)
}

/** 点击节点：定位并激活画布对应节点；已激活则跳过（参考端 onClick）。 */
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

/** 失焦：文本被改动则写回；否则若有待执行插入则立即执行（参考端 onBlur）。 */
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

/** 编辑区按键：Enter 同级 / Tab 子级 / Shift+Tab 升级（记录类型后 blur，插入在渲染完成后执行）。 */
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

/** 拦截粘贴，强制纯文本插入（参考端 handleInputPasteText）。 */
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

// —— 拖拽排序（参考端 onNodeDrop：before/after/inner → INSERT_BEFORE/INSERT_AFTER/MOVE_NODE_TO）——

const dragState = reactive<{ draggingUid: string; overUid: string; position: DropPosition | '' }>({
  draggingUid: '',
  overUid: '',
  position: '',
})

function dragStart(uid: string): void {
  dragState.draggingUid = uid
  ui.isDragOutlineTreeNode = true
}

/** 悬停目标行：按指针在行内的纵向位置判定 before / inner / after（上下 1/4 为同级前后，中段为成为子节点）。 */
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

// —— 删除（参考端 onKeyDown：Delete/Backspace 且鼠标在树区域内）——

const isInTreeArea = ref(false)

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
provide(OUTLINE_CONTROLLER, controller)

// —— 生命周期与事件订阅 ——

const offs: Array<() => void> = []
onMounted(() => {
  refresh()
  window.addEventListener('keydown', onWindowKeydown)
  offs.push(bus.on('data_change', handleDataChange), bus.on('node_tree_render_end', handleNodeTreeRenderEnd))
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onWindowKeydown)
  offs.forEach((off) => off())
})

// 侧栏可能在 mounted 时不可见（SidebarShell v-if），打开时刷新一次。
watch(
  () => ui.activeSidebar,
  (name) => {
    if (name === 'outline') refresh()
  },
)

function openFullscreenEdit(): void {
  ui.isOutlineEdit = true
}
</script>

<template>
  <SidebarShell name="outline" title="大纲">
    <div class="mm-outline-toolbar">
      <button type="button" class="btn tiny" title="全屏编辑大纲" @click="openFullscreenEdit">全屏编辑</button>
    </div>
    <div class="mm-outline-tree" @mouseenter="isInTreeArea = true" @mouseleave="isInTreeArea = false">
      <OutlineTree v-for="root in treeData" :key="root.uid" :node="root" :depth="0" />
    </div>
  </SidebarShell>
</template>
