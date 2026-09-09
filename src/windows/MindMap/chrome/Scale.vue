<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 缩放控件（移植参考端 Scale.vue）：缩小 / 百分比输入 / 放大。
 * 走 mindMap.view.narrow() / enlarge() / setScale(ratio, cx, cy)；
 * 订阅库事件 `scale` 回显当前百分比，`draw_click`（点画布空白）时输入框失焦。
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
  <div class="mm-nav-scale">
    <button type="button" class="mm-nav-btn" title="缩小" @click="narrow">−</button>
    <span class="mm-nav-scale-info">
      <input
        ref="inputRef"
        v-model="scaleNum"
        type="text"
        class="mm-nav-scale-input"
        @input="onInput"
        @change="onChange"
        @focus="onFocus"
        @keydown.stop
        @keyup.stop
      />%
    </span>
    <button type="button" class="mm-nav-btn" title="放大" @click="enlarge">＋</button>
  </div>
</template>
