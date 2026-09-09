<script setup lang="ts">
import { onBeforeUnmount, onMounted, provide, watch } from 'vue'
import { useMindMap } from '../core/useMindMap'
import { useOutlineTree } from '../core/useOutlineTree'
import { printHtml } from '../core/print'
import OutlineTree, { OUTLINE_CONTROLLER, type OutlineNode } from '../sidebars/OutlineTree.vue'

/**
 * 大纲全屏编辑对话框（移植参考端 OutlineEdit.vue）。
 *
 * `ui.isOutlineEdit` 为真时铺满窗口，复用 core/useOutlineTree 协调器与递归行组件 OutlineTree。
 * 顶部：返回（关闭全屏）、打印（把大纲树转成层级 HTML 交给 core/print 打印）。
 */
const ctx = useMindMap()
const { ui } = ctx
const { treeData, controller, isInTreeArea, refresh, subscribe, unsubscribe } = useOutlineTree(ctx)
provide(OUTLINE_CONTROLLER, controller)

let subscribed = false
function ensureSubscribed(open: boolean): void {
  if (open && !subscribed) {
    subscribe()
    subscribed = true
    refresh()
  } else if (!open && subscribed) {
    unsubscribe()
    subscribed = false
  }
}

onMounted(() => ensureSubscribed(ui.isOutlineEdit))
onBeforeUnmount(() => ensureSubscribed(false))
// 仅在打开时订阅，关闭时释放，避免与大纲侧栏的协调器同时空跑。
watch(
  () => ui.isOutlineEdit,
  (open) => ensureSubscribed(open),
)

function back(): void {
  ui.isOutlineEdit = false
}

/** 大纲树 → 层级 <ul><li> HTML（label 已是转义文本）。 */
function nodeToHtml(node: OutlineNode): string {
  const childrenHtml = node.children.length ? `<ul>${node.children.map(nodeToHtml).join('')}</ul>` : ''
  return `<li>${node.label || '(空)'}${childrenHtml}</li>`
}
function print(): void {
  const html = `<ul>${treeData.value.map(nodeToHtml).join('')}</ul>`
  printHtml(html, '思维导图大纲')
}
</script>

<template>
  <div v-if="ui.isOutlineEdit" class="mm-outline-edit-dialog">
    <div class="mm-outline-edit-bar">
      <button type="button" class="btn" @click="back">← 返回</button>
      <span class="mm-outline-edit-title">大纲编辑</span>
      <button type="button" class="btn" @click="print">🖨 打印</button>
    </div>
    <div
      class="mm-outline-edit-body mm-outline-tree"
      @mouseenter="isInTreeArea = true"
      @mouseleave="isInTreeArea = false"
    >
      <OutlineTree v-for="root in treeData" :key="root.uid" :node="root" :depth="0" />
    </div>
  </div>
</template>
