<script setup lang="ts">
import { NSlider } from 'naive-ui'
import { computed, onBeforeUnmount, onMounted, reactive, watch } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { logger } from '@/service/logger'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import {
  alignList,
  borderDasharrayList,
  borderRadiusList,
  borderWidthList,
  fontFamilyList,
  fontSizeList,
  linearGradientDirList,
  lineHeightList,
  shapeList,
  shapeListMap,
} from '../constants'
import ColorField from '../widgets/ColorField.vue'
import FieldRow from '../widgets/FieldRow.vue'
import SelectField from '../widgets/SelectField.vue'
import SwitchField from '../widgets/SwitchField.vue'
import SidebarShell from './SidebarShell.vue'

/**
 * 节点样式侧栏（移植参考端 Style.vue 当前版本）。
 *
 * 文字 / 边框 / 背景 / 形状 / 线条 / 节点内边距 / 图片 / 标签，
 * 所有写入对 `ui.activeNodes` 逐节点 `node.setStyle(prop, value)`（多选一并生效），
 * 渐变方向按参考端特殊分支写 startDir / endDir；回显取首个激活节点 `getStyle(prop, false)`。
 * 无激活节点时显示「请选择一个节点」。
 *
 * 注：参考端旧版（2023-08 前）曾有「常态 / 选中状态」两页签（白名单写入 data.activeStyle），
 * 但当前参考端与库 0.14.0-fix.3 均已移除该特性（渲染层零消费），页签属死控件——已按用户
 * 裁决只保留单状态。行高（lineHeight）为旧版遗留键（0.14 排版行高固定 1.2），按计划保留。
 */

type StyleValue = string | number | boolean

/** 侧栏可编辑的节点样式键（参考端 style 数据对象，另含旧版行高键）。 */
const STYLE_DEFAULTS = {
  shape: '',
  paddingX: 0,
  paddingY: 0,
  color: '',
  fontFamily: '',
  fontSize: 0,
  lineHeight: 0,
  textAlign: '',
  textDecoration: '',
  fontWeight: '',
  fontStyle: '',
  borderWidth: '',
  borderColor: '',
  fillColor: '',
  borderDasharray: '',
  borderRadius: '',
  lineColor: '',
  lineDasharray: '',
  lineWidth: '',
  lineMarkerDir: '',
  gradientStyle: false,
  startColor: '',
  endColor: '',
  linearGradientDir: '',
  imgPlacement: '',
  tagPlacement: '',
} as const

type StyleKey = keyof typeof STYLE_DEFAULTS
const STYLE_KEYS = Object.keys(STYLE_DEFAULTS) as StyleKey[]

const ctx = useMindMap()
const { bus, ui } = ctx

/** 回显副本；键名与参考端 style 对象一一对应。 */
const style = reactive<Record<StyleKey, StyleValue>>({ ...STYLE_DEFAULTS })

/** 当前形状是否为矩形（矩形才显示圆角设置，参考端 v-show 约定）。 */
const isRectangleShape = computed(() => String(style.shape) === 'rectangle')

const fontSizeOptions = fontSizeList.map((size) => ({ name: String(size), value: size }))
const lineHeightOptions = lineHeightList.map((size) => ({ name: String(size), value: size }))
const widthOptions = borderWidthList.map((w) => ({ name: String(w), value: w }))
const radiusOptions = borderRadiusList.map((r) => ({ name: String(r), value: r }))
const arrowDirOptions: readonly { name: string; value: string }[] = [
  { name: '头部', value: 'start' },
  { name: '尾部', value: 'end' },
]
/** 划线选项（参考端 U 按钮弹层里的单选组）。 */
const textDecorationList: readonly { name: string; value: string }[] = [
  { name: '无', value: 'none' },
  { name: '下划线', value: 'underline' },
  { name: '中划线', value: 'line-through' },
  { name: '上划线', value: 'overline' },
]
/** 图片布局（上 / 下 / 左 / 右）。 */
const imgPlacementList: readonly { name: string; value: string }[] = [
  { name: '上', value: 'top' },
  { name: '下', value: 'bottom' },
  { name: '左', value: 'left' },
  { name: '右', value: 'right' },
]
/** 标签布局（右 / 下）。 */
const tagPlacementList: readonly { name: string; value: string }[] = [
  { name: '右', value: 'right' },
  { name: '下', value: 'bottom' },
]

// —— 回显（参考端 initNodeStyle / initLinearGradientDir）——

/** 回显全部样式值 + 渐变方向（参考端 initNodeStyle 逻辑：getStyle(prop, false)）。 */
function initNodeStyle(): void {
  const node = ui.activeNodes[0]
  if (!node) return
  STYLE_KEYS.forEach((key) => {
    style[key] = (node.getStyle(key, false) ?? '') as StyleValue
  })
  initLinearGradientDir(node)
}

/** 颜色值回显（空值兜底白色，同基础样式侧栏约定；写回时仍以真实值为准）。 */
function colorOf(key: StyleKey): string {
  return String(style[key]) || '#ffffff'
}

/** 渐变方向：对比节点 startDir / endDir 与方向选项的坐标回显（参考端 initLinearGradientDir）。 */
function initLinearGradientDir(node: MindMapNode): void {
  const startDir = node.getStyle('startDir', false) as [number, number] | undefined
  const endDir = node.getStyle('endDir', false) as [number, number] | undefined
  if (!startDir || !endDir) return
  const target = linearGradientDirList.find(
    (item) =>
      item.start[0] === startDir[0] &&
      item.start[1] === startDir[1] &&
      item.end[0] === endDir[0] &&
      item.end[1] === endDir[1],
  )
  if (target) style.linearGradientDir = target.value
}

// —— 写入 ——

/** 修改样式（参考端 update(prop)：对全部激活节点生效）。 */
function update(key: StyleKey, value: StyleValue): void {
  style[key] = value
  if (key === 'linearGradientDir') {
    // 渐变方向不直接存：按选项坐标写 startDir / endDir（参考端 update 特殊分支）。
    const target = linearGradientDirList.find((item) => item.value === String(value))
    if (!target) {
      logger.error('mindmap', '未知的渐变方向取值', value)
      return
    }
    ui.activeNodes.forEach((node) => {
      node.setStyles({ startDir: [...target.start], endDir: [...target.end] })
    })
    return
  }
  ui.activeNodes.forEach((node) => {
    node.setStyle(key, style[key])
  })
}

/** 切换加粗（参考端 toggleFontWeight）。 */
function toggleFontWeight(): void {
  update('fontWeight', style.fontWeight === 'bold' ? 'normal' : 'bold')
}

/** 切换斜体（参考端 toggleFontStyle）。 */
function toggleFontStyle(): void {
  update('fontStyle', style.fontStyle === 'italic' ? 'normal' : 'italic')
}

// —— 内边距滑块：拖动中只改回显，停顿 150ms 后统一写入（对齐参考端 el-slider 释放时才 @change）——
let paddingWriteTimer: ReturnType<typeof setTimeout> | null = null

function onPaddingInput(key: 'paddingX' | 'paddingY', value: number): void {
  style[key] = value
  if (paddingWriteTimer) clearTimeout(paddingWriteTimer)
  paddingWriteTimer = setTimeout(() => {
    paddingWriteTimer = null
    update(key, style[key])
  }, 150)
}

// —— 打开侧栏 / 节点激活事件刷新 ——

/** 节点激活事件（参考端 onNodeActive：回显首个激活节点）。 */
function onNodeActive(): void {
  initNodeStyle()
}

watch(
  () => ui.activeSidebar,
  (name) => {
    if (name === 'nodeStyle') initNodeStyle()
  },
)

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(bus.on('node_active', onNodeActive))
  initNodeStyle()
})
onBeforeUnmount(() => {
  offs.forEach((off) => off())
  if (paddingWriteTimer) clearTimeout(paddingWriteTimer)
})
</script>

<template>
  <SidebarShell name="nodeStyle" title="节点样式">
    <template v-if="ui.activeNodes.length > 0">
      <!-- 文字 -->
      <div class="mm-ns-group">文字</div>
      <SelectField
        label="字体"
        :model-value="String(style.fontFamily)"
        :options="fontFamilyList"
        @update:model-value="(v) => update('fontFamily', v)"
      />
      <SelectField
        label="字号"
        :model-value="style.fontSize"
        :options="fontSizeOptions"
        @update:model-value="(v) => update('fontSize', v)"
      />
      <SelectField
        label="行高"
        :model-value="style.lineHeight"
        :options="lineHeightOptions"
        @update:model-value="(v) => update('lineHeight', v)"
      />
      <SelectField
        label="对齐"
        :model-value="String(style.textAlign)"
        :options="alignList"
        @update:model-value="(v) => update('textAlign', v)"
      />
      <ColorField label="颜色" :model-value="colorOf('color')" @update:model-value="(v) => update('color', v)" />
      <FieldRow label="字形">
        <div class="mm-ns-seg">
          <button
            type="button"
            class="mm-ns-seg-btn"
            :class="{ active: style.fontWeight === 'bold' }"
            title="加粗"
            @click="toggleFontWeight"
          >
            B
          </button>
          <button
            type="button"
            class="mm-ns-seg-btn mm-ns-seg-italic"
            :class="{ active: style.fontStyle === 'italic' }"
            title="斜体"
            @click="toggleFontStyle"
          >
            I
          </button>
        </div>
      </FieldRow>
      <FieldRow label="划线">
        <div class="mm-ns-seg">
          <button
            v-for="item in textDecorationList"
            :key="item.value"
            type="button"
            class="mm-ns-seg-btn"
            :class="{ active: String(style.textDecoration) === item.value }"
            @click="update('textDecoration', item.value)"
          >
            {{ item.name }}
          </button>
        </div>
      </FieldRow>

      <!-- 边框 -->
      <div class="mm-ns-group">边框</div>
      <ColorField
        label="颜色"
        :model-value="colorOf('borderColor')"
        @update:model-value="(v) => update('borderColor', v)"
      />
      <FieldRow label="样式">
        <div class="mm-ns-tiles">
          <button
            v-for="item in borderDasharrayList"
            :key="item.value"
            type="button"
            class="mm-ns-tile"
            :class="{ active: String(style.borderDasharray) === item.value }"
            :title="item.name"
            @click="update('borderDasharray', item.value)"
          >
            <svg width="34" height="14" viewBox="0 0 34 14">
              <line
                x1="2"
                y1="7"
                x2="32"
                y2="7"
                stroke-width="2"
                :stroke-dasharray="item.value === 'none' ? undefined : item.value"
              />
            </svg>
          </button>
        </div>
      </FieldRow>
      <SelectField
        label="宽度"
        :model-value="String(style.borderWidth)"
        :options="widthOptions"
        @update:model-value="(v) => update('borderWidth', v)"
      />
      <SelectField
        v-if="isRectangleShape"
        label="圆角"
        :model-value="String(style.borderRadius)"
        :options="radiusOptions"
        @update:model-value="(v) => update('borderRadius', v)"
      />

      <!-- 背景 -->
      <div class="mm-ns-group">背景</div>
      <ColorField
        label="颜色"
        :model-value="colorOf('fillColor')"
        @update:model-value="(v) => update('fillColor', v)"
      />
      <SwitchField
        label="渐变"
        :model-value="!!style.gradientStyle"
        @update:model-value="(v) => update('gradientStyle', v)"
      />
      <template v-if="style.gradientStyle">
        <ColorField
          label="起始色"
          :model-value="colorOf('startColor')"
          @update:model-value="(v) => update('startColor', v)"
        />
        <ColorField
          label="结束色"
          :model-value="colorOf('endColor')"
          @update:model-value="(v) => update('endColor', v)"
        />
        <SelectField
          label="方向"
          :model-value="String(style.linearGradientDir)"
          :options="linearGradientDirList"
          @update:model-value="(v) => update('linearGradientDir', v)"
        />
      </template>

      <!-- 形状 -->
      <div class="mm-ns-group">形状</div>
      <div class="mm-ns-shape-grid">
        <button
          v-for="item in shapeList"
          :key="item.value"
          type="button"
          class="mm-ns-shape-item"
          :class="{ active: String(style.shape) === item.value }"
          :title="item.name"
          @click="update('shape', item.value)"
        >
          <svg viewBox="0 0 60 24">
            <path :d="shapeListMap[item.value]" fill="none" stroke-width="2" />
          </svg>
        </button>
      </div>

      <!-- 线条 -->
      <div class="mm-ns-group">线条</div>
      <ColorField
        label="颜色"
        :model-value="colorOf('lineColor')"
        @update:model-value="(v) => update('lineColor', v)"
      />
      <FieldRow label="样式">
        <div class="mm-ns-tiles">
          <button
            v-for="item in borderDasharrayList"
            :key="item.value"
            type="button"
            class="mm-ns-tile"
            :class="{ active: String(style.lineDasharray) === item.value }"
            :title="item.name"
            @click="update('lineDasharray', item.value)"
          >
            <svg width="34" height="14" viewBox="0 0 34 14">
              <line
                x1="2"
                y1="7"
                x2="32"
                y2="7"
                stroke-width="2"
                :stroke-dasharray="item.value === 'none' ? undefined : item.value"
              />
            </svg>
          </button>
        </div>
      </FieldRow>
      <SelectField
        label="宽度"
        :model-value="String(style.lineWidth)"
        :options="widthOptions"
        @update:model-value="(v) => update('lineWidth', v)"
      />
      <SelectField
        label="箭头位置"
        :model-value="String(style.lineMarkerDir)"
        :options="arrowDirOptions"
        @update:model-value="(v) => update('lineMarkerDir', v)"
      />

      <!-- 节点内边距 -->
      <div class="mm-ns-group">节点内边距</div>
      <FieldRow label="水平">
        <NSlider
          class="mm-ns-slider"
          :value="Number(style.paddingX) || 0"
          :min="0"
          :max="100"
          :step="1"
          @update:value="(v) => onPaddingInput('paddingX', v)"
        />
      </FieldRow>
      <FieldRow label="垂直">
        <NSlider
          class="mm-ns-slider"
          :value="Number(style.paddingY) || 0"
          :min="0"
          :max="100"
          :step="1"
          @update:value="(v) => onPaddingInput('paddingY', v)"
        />
      </FieldRow>

      <!-- 图片布局 -->
      <div class="mm-ns-group">图片</div>
      <FieldRow label="布局">
        <div class="mm-ns-seg">
          <button
            v-for="item in imgPlacementList"
            :key="item.value"
            type="button"
            class="mm-ns-seg-btn"
            :class="{ active: String(style.imgPlacement) === item.value }"
            @click="update('imgPlacement', item.value)"
          >
            {{ item.name }}
          </button>
        </div>
      </FieldRow>

      <!-- 标签布局 -->
      <div class="mm-ns-group">标签</div>
      <FieldRow label="布局">
        <div class="mm-ns-seg">
          <button
            v-for="item in tagPlacementList"
            :key="item.value"
            type="button"
            class="mm-ns-seg-btn"
            :class="{ active: String(style.tagPlacement) === item.value }"
            @click="update('tagPlacement', item.value)"
          >
            {{ item.name }}
          </button>
        </div>
      </FieldRow>
    </template>

    <!-- 无激活节点（参考端 tipBox） -->
    <div v-else class="mm-ns-tip">
      <i class="mm-ns-tip-icon iconfont icontianjiazijiedian"></i>
      <p class="mm-ns-tip-text">请选择一个节点</p>
    </div>
  </SidebarShell>
</template>
