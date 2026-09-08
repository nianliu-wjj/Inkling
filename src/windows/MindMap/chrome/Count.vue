<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 字数与节点数统计（移植参考端 Count.vue）。
 *
 * 每次 `data_change` / 首次渲染后遍历完整节点树：节点数累加，文本拼接后用 DOM 取纯文本长度
 * （节点文本可能是富文本 HTML，直接 length 会把标签算进去）。左下角常显。
 */
const ctx = useMindMap()
const { bus } = ctx

const words = ref(0)
const num = ref(0)
// 复用一个游离 div 把富文本 HTML 转纯文本，避免每次新建。
const measure = document.createElement('div')

interface RawNode {
  data: { text?: unknown }
  children?: RawNode[]
}

function walk(data: RawNode | null, acc: { text: string; count: number }): void {
  if (!data) return
  acc.count += 1
  acc.text += data.data?.text != null ? String(data.data.text) : ''
  data.children?.forEach((child) => walk(child, acc))
}

function recount(): void {
  const data = requireMindMap(ctx).getData() as RawNode
  const acc = { text: '', count: 0 }
  walk(data, acc)
  measure.innerHTML = acc.text
  words.value = (measure.textContent ?? '').length
  num.value = acc.count
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(bus.on('data_change', recount))
  offs.push(bus.on('node_tree_render_end', recount))
  recount()
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <div class="mm-count mm-panel">
    <span>字数 {{ words }}</span>
    <span>节点 {{ num }}</span>
  </div>
</template>
