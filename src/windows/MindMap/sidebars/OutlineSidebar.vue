<script setup lang="ts">
import { onBeforeUnmount, onMounted, provide, watch } from 'vue'
import { useMindMap } from '../core/useMindMap'
import { useOutlineTree } from '../core/useOutlineTree'
import OutlineTree, { OUTLINE_CONTROLLER } from './OutlineTree.vue'

/**
 * 大纲侧栏（移植参考端 OutlineSidebar.vue）。
 *
 * 大纲树的协调逻辑抽在 core/useOutlineTree 组合式函数里（与全屏大纲编辑对话框共用）；
 * 本组件负责把控制器 provide 给递归行组件、顶部「全屏编辑」按钮。
 *
 * 单抽屉壳下本组件只在打开时挂载（`MmDrawer` 用 `<component :is>` 渲染），
 * `onMounted(subscribe)` 的首次 refresh 就发生在打开那一刻。
 */
const ctx = useMindMap()
const { ui } = ctx
const { treeData, controller, isInTreeArea, refresh, subscribe, unsubscribe } = useOutlineTree(ctx)
provide(OUTLINE_CONTROLLER, controller)

onMounted(subscribe)
onBeforeUnmount(unsubscribe)

// 兜底刷新：侧栏若因故在已打开状态下被重新挂载（如日后加 KeepAlive 复活），仍能对上当前数据。
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
  <div class="mm-outline-toolbar">
    <button type="button" class="btn tiny" title="全屏编辑大纲" @click="openFullscreenEdit">全屏编辑</button>
  </div>
  <div class="mm-outline-tree" @mouseenter="isInTreeArea = true" @mouseleave="isInTreeArea = false">
    <OutlineTree v-for="root in treeData" :key="root.uid" :node="root" :depth="0" />
  </div>
</template>
