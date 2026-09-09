<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { borderDasharrayList, fontFamilyList, fontSizeList, lineWidthList } from '../constants'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 关联线样式面板（移植参考端 AssociativeLineStyle.vue，本项目改为锚定到线的浮动面板）。
 *
 * 点击关联线（库事件 associative_line_click(path, clickPath, node, toNode)）时读
 * `associativeLine.getStyleConfig(node, toNode)` 回显；改动写入起点节点 data 的
 * `associativeLineStyle[toNodeUid]` 后调用 `associativeLine.updateActiveLineStyle()`；
 * 删除走 `associativeLine.removeLine()`。字段与参考端一致（无箭头方向项——参考端该版本无此项）。
 * 失活 / 缩放 / 平移 / 点空白时隐藏。
 */
const ctx = useMindMap()
const { bus } = ctx

interface LineStyle {
  associativeLineColor: string
  associativeLineWidth: number
  associativeLineActiveWidth: number
  associativeLineActiveColor: string
  associativeLineDasharray: string
  associativeLineTextFontFamily: string
  associativeLineTextFontSize: number
  associativeLineTextColor: string
}
const DEFAULT: LineStyle = {
  associativeLineColor: '',
  associativeLineWidth: 0,
  associativeLineActiveWidth: 0,
  associativeLineActiveColor: '',
  associativeLineDasharray: '',
  associativeLineTextFontFamily: '',
  associativeLineTextFontSize: 0,
  associativeLineTextColor: '',
}

const show = ref(false)
const position = reactive<{ left: string; top: string }>({ left: '0', top: '0' })
const style = reactive<LineStyle>({ ...DEFAULT })
const fromNode = ref<MindMapNode | null>(null)
const toNode = ref<MindMapNode | null>(null)

const widthOptions = lineWidthList.map((w) => ({ name: String(w), value: w }))
const sizeOptions = fontSizeList.map((s) => ({ name: String(s), value: s }))

function open(
  path: { rbox: () => { x: number; y: number; width: number; height: number } },
  node: MindMapNode,
  target: MindMapNode,
): void {
  fromNode.value = node
  toNode.value = target
  const config = requireMindMap(ctx).associativeLine.getStyleConfig(node, target) as Record<string, unknown>
  ;(Object.keys(DEFAULT) as (keyof LineStyle)[]).forEach((key) => {
    // @ts-expect-error 逐键回填，值类型由库保证
    style[key] = config[key] ?? DEFAULT[key]
  })
  const box = path.rbox()
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
  fromNode.value = null
  toNode.value = null
  Object.assign(style, DEFAULT)
}

/** 写入：合并到起点节点 associativeLineStyle[toNodeUid]，再让插件刷新激活线（参考端 update）。 */
function update<K extends keyof LineStyle>(prop: K, value: LineStyle[K]): void {
  if (!fromNode.value || !toNode.value) return
  style[prop] = value
  const all = (fromNode.value.getData('associativeLineStyle') as Record<string, unknown>) || {}
  const toUid = toNode.value.getData('uid') as string
  const cur = (all[toUid] as Record<string, unknown>) || {}
  fromNode.value.setData({
    associativeLineStyle: { ...all, [toUid]: { ...cur, ...style } },
  })
  requireMindMap(ctx).associativeLine.updateActiveLineStyle()
}

function removeLine(): void {
  requireMindMap(ctx).associativeLine.removeLine()
  hide()
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(
    bus.on('associative_line_click', (path, _clickPath, node, target) =>
      open(
        path as { rbox: () => { x: number; y: number; width: number; height: number } },
        node as MindMapNode,
        target as MindMapNode,
      ),
    ),
    bus.on('associative_line_deactivate', hide),
    bus.on('scale', hide),
    bus.on('translate', hide),
    bus.on('svg_mousedown', hide),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <Teleport to="body">
    <div v-show="show" class="mm-linepanel mm-panel" :style="position" @click.stop>
      <div class="mm-linepanel-title">关联线</div>
      <div class="mm-linepanel-grid">
        <label
          >颜色<input
            type="color"
            :value="style.associativeLineColor || '#000000'"
            @input="update('associativeLineColor', ($event.target as HTMLInputElement).value)"
        /></label>
        <label
          >粗细
          <select
            :value="style.associativeLineWidth"
            @change="update('associativeLineWidth', Number(($event.target as HTMLSelectElement).value))"
          >
            <option v-for="o in widthOptions" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >激活颜色<input
            type="color"
            :value="style.associativeLineActiveColor || '#000000'"
            @input="update('associativeLineActiveColor', ($event.target as HTMLInputElement).value)"
        /></label>
        <label
          >激活粗细
          <select
            :value="style.associativeLineActiveWidth"
            @change="update('associativeLineActiveWidth', Number(($event.target as HTMLSelectElement).value))"
          >
            <option v-for="o in widthOptions" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >虚线
          <select
            :value="style.associativeLineDasharray"
            @change="update('associativeLineDasharray', ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="o in borderDasharrayList" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
      </div>
      <div class="mm-linepanel-title">关联线文字</div>
      <div class="mm-linepanel-grid">
        <label
          >字体
          <select
            :value="style.associativeLineTextFontFamily"
            @change="update('associativeLineTextFontFamily', ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="o in fontFamilyList" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >字号
          <select
            :value="style.associativeLineTextFontSize"
            @change="update('associativeLineTextFontSize', Number(($event.target as HTMLSelectElement).value))"
          >
            <option v-for="o in sizeOptions" :key="o.value" :value="o.value">{{ o.name }}</option>
          </select>
        </label>
        <label
          >颜色<input
            type="color"
            :value="style.associativeLineTextColor || '#000000'"
            @input="update('associativeLineTextColor', ($event.target as HTMLInputElement).value)"
        /></label>
      </div>
      <div class="mm-linepanel-foot">
        <button type="button" class="btn tiny" @click.stop="removeLine">删除关联线</button>
      </div>
    </div>
  </Teleport>
</template>
