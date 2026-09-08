<script lang="ts">
import type { InjectionKey, Ref } from 'vue'

/**
 * 大纲树的一个节点（由 OutlineSidebar 的 refresh 从 mindMap 数据转换而来）。
 * label 已做 HTML 转义并把换行转成 <br>；textCache 是构建时的 label 快照，用于失焦时比对是否被改动。
 */
export interface OutlineNode {
  uid: string
  label: string
  textCache: string
  richText: boolean
  isRoot: boolean
  children: OutlineNode[]
}

/** 拖拽落点相对目标行的位置：上方（同级前插）/ 下方（同级后插）/ 内部（成为子节点）。 */
export type DropPosition = 'before' | 'after' | 'inner'

/** 拖拽中的共享状态（供各行渲染落点提示）。 */
export interface OutlineDragState {
  draggingUid: string
  overUid: string
  position: DropPosition | ''
}

/**
 * 大纲树控制器：协调逻辑集中在 OutlineSidebar，递归行组件 OutlineTree 只调用这些方法。
 * 拆分原因——递归组件的每个实例是「一行」，无法独占「整棵树」的单例协调状态（当前项、
 * 拖拽、插入调度、与库的命令交互），故把协调器放在容器 OutlineSidebar 里 provide 下来。
 */
export interface OutlineController {
  /** 只读模式下 contenteditable 关闭。 */
  readonly: () => boolean
  /** 当前高亮的节点 uid。 */
  currentUid: Ref<string>
  /** 每次 refresh 自增，用作 contenteditable 的 key 强制重建（丢弃编辑中残留，与参考端随机 key 等价）。 */
  renderKey: Ref<number>
  isExpanded: (uid: string) => boolean
  toggleExpand: (uid: string) => void
  /** 点击节点：定位并激活画布对应节点（GO_TARGET_NODE）。 */
  clickNode: (uid: string) => void
  /** 失焦：文本被改动则写回 node.setText（富文本分支）。 */
  blurNode: (node: OutlineNode, el: HTMLElement) => void
  /** 编辑区按键：Enter 同级 / Tab 子级 / Shift+Tab 升级（先记录插入类型再 blur）。 */
  keydownNode: (event: KeyboardEvent, el: HTMLElement) => void
  /** 拦截粘贴，强制纯文本（避免带入外部富文本样式）。 */
  pasteNode: (event: ClipboardEvent) => void
  dragState: OutlineDragState
  dragStart: (uid: string) => void
  dragOverRow: (uid: string, event: DragEvent) => void
  dropRow: (uid: string) => void
  dragEnd: () => void
}

/** provide / inject 键。 */
export const OUTLINE_CONTROLLER: InjectionKey<OutlineController> = Symbol('inkling-outline-controller')
</script>

<script setup lang="ts">
import { inject } from 'vue'

/**
 * 大纲树递归行组件（移植参考端 Outline.vue 的 el-tree 自定义节点为 Vue 3 自实现递归树）。
 * 每个实例渲染一行（展开箭头 + contenteditable 文本），并递归渲染其子节点。
 * 所有交互转交注入的控制器（见 OutlineSidebar）。
 *
 * 注：OUTLINE_CONTROLLER 与 OutlineNode 定义在上方普通 `<script>` 块，与本 `<script setup>`
 * 同属一个模块作用域，可直接引用，无需从自身文件 import。
 */
defineOptions({ name: 'OutlineTree' })

const props = defineProps<{ node: OutlineNode; depth: number }>()

const ctrl = inject(OUTLINE_CONTROLLER)
if (!ctrl) throw new Error('OutlineTree 必须在 OutlineSidebar 的 provide 作用域内使用')

const hasChildren = (): boolean => props.node.children.length > 0

function onEditKeydown(event: KeyboardEvent): void {
  ctrl!.keydownNode(event, event.target as HTMLElement)
}
function onEditBlur(event: FocusEvent): void {
  ctrl!.blurNode(props.node, event.target as HTMLElement)
}
</script>

<template>
  <div class="mm-outline-node">
    <div
      class="mm-outline-row"
      :class="{
        current: ctrl!.currentUid.value === node.uid,
        'drop-before': ctrl!.dragState.overUid === node.uid && ctrl!.dragState.position === 'before',
        'drop-after': ctrl!.dragState.overUid === node.uid && ctrl!.dragState.position === 'after',
        'drop-inner': ctrl!.dragState.overUid === node.uid && ctrl!.dragState.position === 'inner',
      }"
      :style="{ paddingLeft: depth * 16 + 'px' }"
      :draggable="!node.isRoot && !ctrl!.readonly()"
      @click="ctrl!.clickNode(node.uid)"
      @dragstart="ctrl!.dragStart(node.uid)"
      @dragover.prevent="ctrl!.dragOverRow(node.uid, $event)"
      @drop.prevent="ctrl!.dropRow(node.uid)"
      @dragend="ctrl!.dragEnd()"
    >
      <button
        v-if="hasChildren()"
        type="button"
        class="mm-outline-arrow"
        :class="{ expanded: ctrl!.isExpanded(node.uid) }"
        title="展开 / 收起"
        @click.stop="ctrl!.toggleExpand(node.uid)"
      >
        ▶
      </button>
      <span v-else class="mm-outline-dot" />
      <span
        :key="ctrl!.renderKey.value + ':' + node.uid"
        class="mm-outline-edit"
        :class="{ root: node.isRoot }"
        :contenteditable="!ctrl!.readonly()"
        :data-uid="node.uid"
        spellcheck="false"
        @keydown.stop="onEditKeydown"
        @keyup.stop
        @blur="onEditBlur"
        @paste="ctrl!.pasteNode($event)"
        v-html="node.label"
      />
    </div>
    <div v-if="hasChildren() && ctrl!.isExpanded(node.uid)" class="mm-outline-children">
      <OutlineTree v-for="child in node.children" :key="child.uid" :node="child" :depth="depth + 1" />
    </div>
  </div>
</template>
