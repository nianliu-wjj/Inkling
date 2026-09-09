<script setup lang="ts">
import OuterFrame from 'simple-mind-map/src/plugins/OuterFrame.js'
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { borderDasharrayList, fontFamilyList, fontSizeList, lineWidthList } from '../constants'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 外框样式面板（移植参考端 NodeOuterFrame.vue，本项目改为锚定到外框的浮动面板）。
 *
 * 激活外框（库事件 outer_frame_active(el, parentNode, range)）时取范围内首个节点的
 * outerFrame 样式回显（缺项用 OuterFrame.defaultStyle 兜底）；改动走
 * `outerFrame.updateActiveOuterFrame({[key]: val})`；删除外框 / 删除外框文字分别走
 * `removeActiveOuterFrame()` / `removeActiveOuterFrameText()`。失活 / 删除时隐藏。
 */
const ctx = useMindMap()
const { bus } = ctx

const DEFAULT = (OuterFrame as unknown as { defaultStyle: Record<string, unknown> }).defaultStyle

const show = ref(false)
const position = reactive<{ left: string; top: string }>({ left: '0', top: '0' })
const styleConfig = reactive<Record<string, unknown>>({ ...DEFAULT })
const padding = reactive<{ x: number; y: number }>({ x: 0, y: 0 })

const widthOptions = lineWidthList.map((w) => ({ name: String(w), value: w }))
const sizeOptions = fontSizeList.map((s) => ({ name: String(s), value: s }))
const radiusOptions = [0, 2, 5, 8, 10, 15, 20].map((r) => ({ name: String(r), value: r }))

function open(
  el: { rbox: () => { x: number; y: number; width: number; height: number } },
  parentNode: MindMapNode,
  range: number[],
): void {
  const firstNode = parentNode.children[range[0]]
  const frame = (firstNode?.getData('outerFrame') as Record<string, unknown>) || {}
  Object.keys(DEFAULT).forEach((key) => {
    styleConfig[key] = typeof frame[key] !== 'undefined' ? frame[key] : DEFAULT[key]
  })
  const tfp = (styleConfig.textFillPadding as number[]) || [0, 0, 0, 0]
  padding.x = tfp[0] ?? 0
  padding.y = tfp[1] ?? 0
  const box = el.rbox()
  const boxWidth = 280
  let left = box.x + box.width / 2 - boxWidth / 2
  if (left < 0) left = 0
  if (left + boxWidth > window.innerWidth) left = window.innerWidth - boxWidth
  position.left = left + 'px'
  position.top = box.y + box.height + 8 + 'px'
  show.value = true
}

function hide(): void {
  show.value = false
}

function update(key: string, value: unknown): void {
  styleConfig[key] = value
  requireMindMap(ctx).outerFrame.updateActiveOuterFrame({ [key]: value })
}
function toggleFontWeight(): void {
  update('fontWeight', styleConfig.fontWeight === 'bold' ? 'normal' : 'bold')
}
function toggleFontStyle(): void {
  update('fontStyle', styleConfig.fontStyle === 'italic' ? 'normal' : 'italic')
}
/** 内边距水平/垂直（写入四元组 [pl,pt,pr,pb]，与参考端一致）。 */
function updatePadding(dir: 'x' | 'y', value: number): void {
  if (dir === 'x') {
    padding.x = value
    update('textFillPadding', [value, padding.y, value, padding.y])
  } else {
    padding.y = value
    update('textFillPadding', [padding.x, value, padding.x, value])
  }
}
function deleteFrame(): void {
  requireMindMap(ctx).outerFrame.removeActiveOuterFrame()
}
function deleteText(): void {
  requireMindMap(ctx).outerFrame.removeActiveOuterFrameText()
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(
    bus.on('outer_frame_active', (el, parentNode, range) =>
      open(
        el as { rbox: () => { x: number; y: number; width: number; height: number } },
        parentNode as MindMapNode,
        range as number[],
      ),
    ),
    bus.on('outer_frame_delete', hide),
    bus.on('outer_frame_deactivate', hide),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <Teleport to="body">
    <div v-show="show" class="mm-framepanel mm-panel" :style="position" @click.stop>
      <div class="mm-framepanel-title">外框</div>
      <div class="mm-framepanel-grid">
        <label
          >边框色<input
            type="color"
            :value="(styleConfig.strokeColor as string) || '#0984e3'"
            @input="update('strokeColor', ($event.target as HTMLInputElement).value)"
        /></label>
        <label
          >边框宽
          <select
            :value="styleConfig.strokeWidth"
            @change="update('strokeWidth', Number(($event.target as HTMLSelectElement).value))"
          >
            <option v-for="o in widthOptions" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >虚线
          <select
            :value="styleConfig.strokeDasharray"
            @change="update('strokeDasharray', ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="o in borderDasharrayList" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >圆角
          <select
            :value="styleConfig.radius"
            @change="update('radius', Number(($event.target as HTMLSelectElement).value))"
          >
            <option v-for="o in radiusOptions" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >填充色<input
            type="color"
            :value="(styleConfig.fill as string) || '#ffffff'"
            @input="update('fill', ($event.target as HTMLInputElement).value)"
        /></label>
      </div>

      <div class="mm-framepanel-title">文字</div>
      <div class="mm-framepanel-grid">
        <label
          >字体
          <select
            :value="styleConfig.fontFamily"
            @change="update('fontFamily', ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="o in fontFamilyList" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >字号
          <select
            :value="styleConfig.fontSize"
            @change="update('fontSize', Number(($event.target as HTMLSelectElement).value))"
          >
            <option v-for="o in sizeOptions" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >颜色<input
            type="color"
            :value="(styleConfig.color as string) || '#ffffff'"
            @input="update('color', ($event.target as HTMLInputElement).value)"
        /></label>
        <div class="mm-framepanel-toggles">
          <button type="button" :class="{ active: styleConfig.fontWeight === 'bold' }" @click="toggleFontWeight">
            <b>B</b>
          </button>
          <button type="button" :class="{ active: styleConfig.fontStyle === 'italic' }" @click="toggleFontStyle">
            <i>I</i>
          </button>
        </div>
        <label
          >内边距X<input
            type="number"
            min="0"
            max="100"
            :value="padding.x"
            @change="updatePadding('x', Number(($event.target as HTMLInputElement).value))"
        /></label>
        <label
          >内边距Y<input
            type="number"
            min="0"
            max="100"
            :value="padding.y"
            @change="updatePadding('y', Number(($event.target as HTMLInputElement).value))"
        /></label>
      </div>

      <div class="mm-framepanel-foot">
        <button type="button" class="btn tiny" @click.stop="deleteText">删除文字</button>
        <button type="button" class="btn tiny" @click.stop="deleteFrame">删除外框</button>
      </div>
    </div>
  </Teleport>
</template>
