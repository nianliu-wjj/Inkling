<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useMindMap } from '../core/useMindMap'

/**
 * 节点图片预览（移植参考端 NodeImgPreview.vue）。
 *
 * 双击节点图片（库事件 node_img_dblclick(node, e)）时全屏预览该节点图片
 * （node.getData('image')）；点背景 / ✕ / Esc 关闭。参考端用 el-image 的预览，本项目
 * 用轻量遮罩层实现等价的全图预览（点击关闭），避免为编程式触发预览引入额外组件耦合。
 */
const { bus } = useMindMap()

const show = ref(false)
const src = ref('')
const title = ref('')

function open(node: MindMapNode): void {
  const image = node.getData('image') as string
  if (!image) return
  src.value = image
  title.value = (node.getData('imageTitle') as string) || ''
  show.value = true
}
function close(): void {
  show.value = false
  src.value = ''
  title.value = ''
}
function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape' && show.value) close()
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(bus.on('node_img_dblclick', (node) => open(node as MindMapNode)))
  window.addEventListener('keydown', onKeydown)
})
onBeforeUnmount(() => {
  offs.forEach((off) => off())
  window.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="mm-imgpv" @click="close">
      <img class="mm-imgpv-img" :src="src" :alt="title" @click.stop />
      <button type="button" class="mm-imgpv-close" title="关闭预览（Esc）" @click="close">✕</button>
    </div>
  </Teleport>
</template>
