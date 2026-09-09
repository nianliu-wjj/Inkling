<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useMindMap } from '../core/useMindMap'

/**
 * 节点图片位置浮动栏（移植参考端 NodeImgPlacementToolbar.vue）。
 *
 * 点击节点图片（库事件 `node_img_click(node, imgNode)`）时在图片上方弹出上/下/左/右
 * 四向按钮，切换 `node.setStyle('imgPlacement', dir)`。位置依据图片元素 `rbox()`；
 * Teleport 到 body；缩放 / 平移 / 点空白 / 双击 / 调整图片 / 删除图片 / 切换激活节点时关闭。
 */
const { bus } = useMindMap()

type Placement = 'top' | 'bottom' | 'left' | 'right'
const placementList: Placement[] = ['top', 'bottom', 'left', 'right']
const placementLabel: Record<Placement, string> = { top: '上', bottom: '下', left: '左', right: '右' }

const show = ref(false)
const style = reactive<{ left: string; top: string }>({ left: '0', top: '0' })
const node = ref<MindMapNode | null>(null)
// svg.js 元素，无类型声明，用 any 承接（仅调用 rbox）。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let imgNode: any = null
const imgPlacement = ref('')
const rootRef = ref<HTMLElement | null>(null)

function updatePos(): void {
  if (!imgNode || !rootRef.value) return
  const { width } = rootRef.value.getBoundingClientRect()
  const box = imgNode.rbox()
  style.left = box.x + box.width / 2 - width / 2 + 'px'
  style.top = box.y - 40 - 5 + 'px'
}

function open(target: MindMapNode, img: unknown): void {
  node.value = target
  imgPlacement.value = (target.getStyle('imgPlacement', false) as string) || ''
  imgNode = img
  show.value = true
  void nextTick(updatePos)
}

function close(): void {
  show.value = false
  node.value = null
  imgNode = null
  imgPlacement.value = ''
}

function onNodeActive(active: MindMapNode | null): void {
  if (active === node.value) return
  close()
}

function update(dir: Placement): void {
  if (!node.value) return
  imgPlacement.value = dir
  node.value.setStyle('imgPlacement', dir)
  close()
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(
    bus.on('node_img_click', (target, img) => open(target as MindMapNode, img)),
    bus.on('draw_click', close),
    bus.on('svg_mousedown', close),
    bus.on('node_dblclick', close),
    bus.on('node_active', (active) => onNodeActive((active as MindMapNode) ?? null)),
    bus.on('scale', updatePos),
    bus.on('translate', close),
    bus.on('node_img_adjust_btn_mousedown', close),
    bus.on('delete_node_img_from_delete_btn', close),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <Teleport to="body">
    <div v-show="show" ref="rootRef" class="mm-imgtb mm-panel" :style="style" @click.stop>
      <button
        v-for="item in placementList"
        :key="item"
        type="button"
        class="mm-imgtb-item"
        :class="{ selected: imgPlacement === item }"
        :title="`图片在文字${placementLabel[item]}侧`"
        @click="update(item)"
      >
        {{ placementLabel[item] }}
      </button>
    </div>
  </Teleport>
</template>
