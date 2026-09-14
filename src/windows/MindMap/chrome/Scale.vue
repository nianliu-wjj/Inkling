<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 缩放控件（移植参考端 Scale.vue）：缩小 / 百分比输入 / 放大。
 * 走 mindMap.view.narrow() / enlarge() / setScale(ratio, cx, cy)；
 * 订阅库事件 `scale` 回显当前百分比，`draw_click`（点画布空白）时输入框失焦。
 *
 * 阶段五 Task 5 起由 `MmBottombar.vue` 挂在底栏分隔线之后，三件元素改为底栏的直接子项：
 * 两枚按钮用生成层 `.mm-ctrl-btn`，百分比用生成层 `.mm-zoom-val`（原型那里是只读文本，
 * 这里是可编辑输入框——保留我们的手输缩放能力，输入框样式见自有层 `.mm-zoom-input`）。
 */
const ctx = useMindMap()
const { bus } = ctx

const scaleNum = ref('100')
let cacheScaleNum = '100'
const inputRef = ref<HTMLInputElement | null>(null)

/** 缩放系数 → 百分数整数字符串。 */
function toPercent(scale: number): string {
  return (scale * 100).toFixed(0)
}

function narrow(): void {
  requireMindMap(ctx).view.narrow()
}
function enlarge(): void {
  requireMindMap(ctx).view.enlarge()
}

function onFocus(): void {
  cacheScaleNum = scaleNum.value
}
/** 只允许数字。 */
function onInput(): void {
  scaleNum.value = scaleNum.value.replace(/[^0-9]+/g, '')
}
/** 手动输入缩放：非法值回退缓存，合法值以画布中心为焦点 setScale。 */
function onChange(): void {
  const value = Number(scaleNum.value)
  if (Number.isNaN(value) || value <= 0) {
    scaleNum.value = cacheScaleNum
    return
  }
  const mindMap = requireMindMap(ctx)
  mindMap.view.setScale(value / 100, mindMap.width / 2, mindMap.height / 2)
}

function onScale(scale: number): void {
  scaleNum.value = toPercent(scale)
}
function onDrawClick(): void {
  inputRef.value?.blur()
}

const offs: Array<() => void> = []
onMounted(() => {
  scaleNum.value = toPercent(requireMindMap(ctx).view.scale)
  offs.push(
    bus.on('scale', (scale) => onScale(scale as number)),
    bus.on('draw_click', onDrawClick),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <!-- 缩小 / 百分比 / 放大 三件平铺在底栏 .mm-ctrl-right 里（多根节点），不留自有壳。 -->
  <button type="button" class="mm-ctrl-btn" title="缩小" @click="narrow">−</button>
  <span class="mm-zoom-val">
    <input
      ref="inputRef"
      v-model="scaleNum"
      type="text"
      class="mm-zoom-input"
      @input="onInput"
      @change="onChange"
      @focus="onFocus"
      @keydown.stop
      @keyup.stop
    />%
  </span>
  <button type="button" class="mm-ctrl-btn" title="放大" @click="enlarge">＋</button>
</template>
