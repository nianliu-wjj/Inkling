<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useMindMap } from '../core/useMindMap'
import SidebarShell from './SidebarShell.vue'

/**
 * 备注侧栏（移植参考端 NodeNoteSidebar.vue）：常驻编辑形态。
 * node_active 回显当前节点备注；失焦时保存 `node.setNote`。
 */
const { bus, ui } = useMindMap()

const node = ref<MindMapNode | null>(null)
const note = ref('')

function sync(): void {
  node.value = ui.activeNodes[0] ?? null
  note.value = node.value ? (node.value.getData('note') as string) || '' : ''
}

function save(): void {
  if (!node.value) return
  if ((node.value.getData('note') || '') !== note.value) node.value.setNote(note.value)
}

const offs: Array<() => void> = []
onMounted(() => {
  sync()
  offs.push(bus.on('node_active', sync))
})
onBeforeUnmount(() => offs.forEach((off) => off()))
// 侧栏打开时也同步一次（可能在未选中节点时就打开了）。
watch(
  () => ui.activeSidebar,
  (name) => {
    if (name === 'nodeNoteSidebar') sync()
  },
)
</script>

<template>
  <SidebarShell name="nodeNoteSidebar" title="备注">
    <p v-if="!node" class="mm-dialog-hint">请先选择一个节点</p>
    <textarea
      v-else
      v-model="note"
      class="mm-dialog-textarea mm-note-area"
      placeholder="给节点添加备注…（失焦自动保存）"
      @blur="save"
    />
  </SidebarShell>
</template>
