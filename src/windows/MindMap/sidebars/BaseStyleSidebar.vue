<script setup lang="ts">
import { NTabPane, NTabs } from 'naive-ui'
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { logger } from '@/service/logger'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import {
  backgroundPositionList,
  backgroundRepeatList,
  backgroundSizeList,
  borderDasharrayList,
  fontFamilyList,
  fontSizeList,
  lineWidthList,
  lineStyleList,
  rootLineKeepSameInCurveList,
  rainbowLinesOptions,
  supportLineRadiusLayouts,
  supportLineStyleLayoutsMap,
  supportNodeUseLineStyleLayouts,
  supportRootLineKeepSameInCurveLayouts,
} from '../constants'
import ColorField from '../widgets/ColorField.vue'
import FieldRow from '../widgets/FieldRow.vue'
import NumberField from '../widgets/NumberField.vue'
import SelectField from '../widgets/SelectField.vue'
import SwitchField from '../widgets/SwitchField.vue'
import SidebarShell from './SidebarShell.vue'
import { backgroundFileToDataUrl } from './baseStyleOptions'

/**
 * 基础样式侧栏（移植参考端 BaseStyle.vue，全侧栏里最大的一块）。
 *
 * 绝大多数项走 `mindMap.setThemeConfig({ ...已有自定义配置, [prop]: value })`（库端 setThemeConfig 整体替换
 * opt.themeConfig，须累积后再写，与参考端传整个 config 对象等价）；
 * 彩虹线条走 `mindMap.rainbowLines.updateRainLinesConfig`；外框内边距走 `mindMap.updateConfig`。
 * 受结构限制的项（连线风格 / 圆角 / 括号形态 / 节点边框风格）用当前布局过滤，`layout_change` 时刷新；
 * 主题切换（切主题时侧栏必然关闭，重新打开时 activeSidebar watcher 全量回显）后重新回显全部值。
 *
 * 未移植项：内置背景图列表（参考端 bgList 来自其服务端，本机无该服务，按计划不做）。
 */

type StyleValue = string | number | boolean

/** 参考端 style 对象的初始形态（也用于 initStyle 的键清单）。 */
const STYLE_DEFAULTS = {
  backgroundColor: '',
  lineColor: '',
  lineWidth: 0,
  lineStyle: '',
  showLineMarker: false,
  rootLineKeepSameInCurve: false,
  rootLineStartPositionKeepSameInCurve: false,
  lineRadius: 0,
  generalizationLineWidth: 0,
  generalizationLineColor: '',
  associativeLineColor: '',
  associativeLineWidth: 0,
  associativeLineActiveWidth: 0,
  associativeLineDasharray: '',
  associativeLineActiveColor: '',
  associativeLineTextFontSize: 0,
  associativeLineTextColor: '',
  associativeLineTextFontFamily: '',
  paddingX: 0,
  paddingY: 0,
  imgMaxWidth: 0,
  imgMaxHeight: 0,
  iconSize: 0,
  backgroundImage: '',
  backgroundRepeat: 'no-repeat',
  backgroundPosition: '',
  backgroundSize: '',
  marginX: 0,
  marginY: 0,
  nodeUseLineStyle: false,
}

/** 主题配置回显副本；键名与参考端 style 对象一一对应。 */
const style = reactive<Record<keyof typeof STYLE_DEFAULTS, StyleValue>>({ ...STYLE_DEFAULTS })

const ctx = useMindMap()
const { bus, mapConfig, ui } = ctx

const STYLE_KEYS = Object.keys(STYLE_DEFAULTS) as (keyof typeof STYLE_DEFAULTS)[]

/** 节点外边距按层级取值：second = 二级节点，node = 三级及以下。 */
const marginActiveTab = ref<'second' | 'node'>('second')
/** 当前布局（layout_change 时刷新），用于过滤受结构限制的项。 */
const currentLayout = ref('')
/** 彩虹线条当前配色（null = 未使用）。 */
const curRainbowLineColorList = ref<readonly string[] | null>(null)
/** 外框内边距（库实例配置，非主题配置）。 */
const outerFramePadding = reactive({ outerFramePaddingX: 0, outerFramePaddingY: 0 })

const lineRadiusList: readonly number[] = [0, 2, 5, 7, 10, 12, 15]

/** 连线风格（受结构限制过滤后的可见项）。 */
const lineStyleListShow = computed(() =>
  lineStyleList.filter((item) => {
    const list = supportLineStyleLayoutsMap[item.value]
    return !list || list.includes(currentLayout.value)
  }),
)
/** 直线模式下可设圆角的结构。 */
const showLineRadius = computed(
  () => style.lineStyle === 'straight' && supportLineRadiusLayouts.includes(currentLayout.value),
)
/** 括号形态（根节点曲线样式）可见的结构。 */
const showRootLineKeepSameInCurve = computed(() => supportRootLineKeepSameInCurveLayouts.includes(currentLayout.value))
/** 节点边框风格可见的结构。 */
const showNodeUseLineStyle = computed(() => supportNodeUseLineStyleLayouts.includes(currentLayout.value))
/** 根节点连线起始位置选项。 */
const rootLineStartPosList: { name: string; value: boolean }[] = [
  { name: '中心', value: false },
  { name: '边缘', value: true },
]

const lineWidthOptions = lineWidthList.map((w) => ({ name: String(w), value: w }))
const fontSizeOptions = fontSizeList.map((s) => ({ name: String(s), value: s }))

// —— 回显（参考端 initStyle / initMarginStyle / initRainbowLines / initOuterFramePadding）——

/** 全量回显主题配置；backgroundImage 为 'none' 时置空（参考端约定）。 */
function initStyle(): void {
  const mindMap = requireMindMap(ctx)
  STYLE_KEYS.forEach((key) => {
    const value = mindMap.getThemeConfig(key) as StyleValue
    style[key] = key === 'backgroundImage' && value === 'none' ? '' : value
  })
  initMarginStyle()
}

/** 外边距按当前层级页签回显。 */
function initMarginStyle(): void {
  const config = requireMindMap(ctx).getThemeConfig() as Record<string, Record<string, number>>
  ;(['marginX', 'marginY'] as const).forEach((key) => {
    style[key] = config?.[marginActiveTab.value]?.[key] ?? 0
  })
}

/** 回显彩虹线条：开启时取插件当前配色，未开启置 null。 */
function initRainbowLines(): void {
  const mindMap = requireMindMap(ctx)
  const config = (mindMap.getConfig('rainbowLinesConfig') ?? {}) as { open?: boolean }
  curRainbowLineColorList.value = config.open ? (mindMap.rainbowLines?.getColorsList?.() ?? null) : null
}

/** 回显外框内边距（库实例配置）。参考端 Y 轴误读了 X 的值，此处修正为 Y 自己的。 */
function initOuterFramePadding(): void {
  const mindMap = requireMindMap(ctx)
  outerFramePadding.outerFramePaddingX = (mindMap.getConfig('outerFramePaddingX') as number) ?? 0
  outerFramePadding.outerFramePaddingY = (mindMap.getConfig('outerFramePaddingY') as number) ?? 0
}

/** 打开侧栏 / 主题变化 / 布局变化时统一刷新入口。 */
function refreshAll(): void {
  currentLayout.value = requireMindMap(ctx).getLayout()
  initStyle()
  initRainbowLines()
  initOuterFramePadding()
}

// —— 写入 ——

/** 主题配置写入（参考端 update(key, value)；backgroundImage 'none' 归一化为空串回显）。 */
function update(key: keyof typeof STYLE_DEFAULTS, value: StyleValue): void {
  if (key === 'backgroundImage' && value === 'none') style[key] = ''
  else style[key] = value
  // setThemeConfig 会整体替换 opt.themeConfig，须带上已累积的自定义配置再合入新键，
  // 等价参考端「data.theme.config[key] = value 后传整个 config」。
  const mindMap = requireMindMap(ctx)
  mindMap.setThemeConfig({ ...mindMap.getCustomThemeConfig(), [key]: value })
}

/** 外边距写入：写入当前层级页签（second / node）下的 marginX / marginY（参考端 updateMargin）。 */
function updateMargin(key: 'marginX' | 'marginY', value: number): void {
  style[key] = value
  const mindMap = requireMindMap(ctx)
  const custom = mindMap.getCustomThemeConfig() as Record<string, Record<string, number>>
  const tab = { ...(custom?.[marginActiveTab.value] ?? {}), [key]: value }
  mindMap.setThemeConfig({ ...custom, [marginActiveTab.value]: tab })
}

/** 外框内边距写入（参考端 updateOuterFramePadding；参考端 storeConfig 由 mapConfig 落盘替代）。 */
function updateOuterFramePadding(prop: 'outerFramePaddingX' | 'outerFramePaddingY', value: number): void {
  const mindMap = requireMindMap(ctx)
  outerFramePadding[prop] = value
  mapConfig[prop] = value
  mindMap.updateConfig({ [prop]: value })
  mindMap.render()
}

/** 彩虹线条写入（参考端 updateRainbowLinesConfig；storeConfig 由 mapConfig 落盘替代）。 */
function updateRainbowLinesConfig(item: { list?: string[] }): void {
  const newConfig = item.list ? { open: true, colorsList: item.list } : { open: false }
  curRainbowLineColorList.value = item.list ?? null
  mapConfig.rainbowLinesConfig = newConfig
  requireMindMap(ctx).rainbowLines?.updateRainLinesConfig?.(newConfig)
  logger.info('mindmap', `更新彩虹线条 open=${newConfig.open}`)
}

// —— 背景图上传（隐藏 file input + FileReader；参考端 ImgUpload 组件）——

const bgFileInput = ref<HTMLInputElement | null>(null)

function pickBackgroundImage(): void {
  bgFileInput.value?.click()
}

async function onBackgroundFile(event: Event): Promise<void> {
  const file = (event.target as HTMLInputElement).files?.[0]
  ;(event.target as HTMLInputElement).value = ''
  if (!file) return
  try {
    const dataUrl = await backgroundFileToDataUrl(file)
    update('backgroundImage', dataUrl)
    logger.info('mindmap', `背景图上传成功 name=${file.name} size=${file.size}`)
  } catch (error) {
    logger.error('mindmap', '背景图上传失败', error)
  }
}

/** 删除背景图：写回 'none'（库与参考端的空值约定），回显清空。 */
function removeBackgroundImage(): void {
  update('backgroundImage', 'none')
}

/** 背景图 dataURL（'' 表示无图）；供模板预览与删除按钮判定。 */
const bgImage = computed(() => (style.backgroundImage === 'none' ? '' : String(style.backgroundImage)))

/** 颜色值回显（空值兜底白色）；主题配置里颜色恒为字符串。 */
function colorOf(key: keyof typeof STYLE_DEFAULTS): string {
  return String(style[key]) || '#ffffff'
}

// —— 结构限制联动（参考端 watch lineStyleListShow）——

/** 当前风格在受限列表里不存在时回退到第一项（参考端 watch lineStyleListShow 逻辑）。 */
watch(lineStyleListShow, (list) => {
  if (list.length === 0) return
  if (!list.some((item) => item.value === style.lineStyle)) {
    update('lineStyle', list[0].value)
  }
})

// —— 打开侧栏与库事件刷新 ——

watch(
  () => ui.activeSidebar,
  (name) => {
    if (name === 'baseStyle') refreshAll()
  },
)

const offs: Array<() => void> = []
onMounted(() => {
  refreshAll()
  // 注意：本库版本（0.14.0-fix.3）实际不发 theme_change（只有 view_theme_change，且未转发到 bus），
  // 该订阅保持与计划一致的事件名；主题切换场景由 activeSidebar watcher 兜底（见上）。
  offs.push(
    bus.on('theme_change', () => initStyle()),
    bus.on('layout_change', () => (currentLayout.value = requireMindMap(ctx).getLayout())),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <SidebarShell name="baseStyle" title="基础样式">
    <!-- 背景：颜色 / 图片（重复 / 位置 / 大小 / 本地上传） -->
    <div class="mm-bs-group">背景</div>
    <NTabs type="segment" size="small" default-value="color">
      <NTabPane name="color" tab="颜色">
        <ColorField
          label="背景颜色"
          :model-value="colorOf('backgroundColor')"
          @update:model-value="(v) => update('backgroundColor', v)"
        />
      </NTabPane>
      <NTabPane name="image" tab="图片">
        <div class="mm-bs-bg-actions">
          <button type="button" class="btn" @click="pickBackgroundImage">选择图片…</button>
          <button v-if="bgImage" type="button" class="btn" @click="removeBackgroundImage">删除背景图</button>
          <input ref="bgFileInput" type="file" accept="image/*" hidden @change="onBackgroundFile" />
        </div>
        <div v-if="bgImage" class="mm-bs-bg-preview"><img :src="bgImage" alt="背景预览" /></div>
        <SelectField
          label="图片重复"
          :model-value="String(style.backgroundRepeat)"
          :options="backgroundRepeatList"
          @update:model-value="(v) => update('backgroundRepeat', v)"
        />
        <SelectField
          label="图片位置"
          :model-value="String(style.backgroundPosition)"
          :options="backgroundPositionList"
          @update:model-value="(v) => update('backgroundPosition', v)"
        />
        <SelectField
          label="图片大小"
          :model-value="String(style.backgroundSize)"
          :options="backgroundSizeList"
          @update:model-value="(v) => update('backgroundSize', v)"
        />
      </NTabPane>
    </NTabs>

    <!-- 连线：颜色 / 粗细 / 风格 / 括号形态 / 圆角 / 根节点起始位置 / 箭头 -->
    <div class="mm-bs-group">连线</div>
    <ColorField label="颜色" :model-value="colorOf('lineColor')" @update:model-value="(v) => update('lineColor', v)" />
    <SelectField
      label="粗细"
      :model-value="style.lineWidth"
      :options="lineWidthOptions"
      @update:model-value="(v) => update('lineWidth', v)"
    />
    <SelectField
      v-if="lineStyleListShow.length > 1"
      label="风格"
      :model-value="String(style.lineStyle)"
      :options="lineStyleListShow"
      @update:model-value="(v) => update('lineStyle', v)"
    />
    <SelectField
      v-if="style.lineStyle === 'curve' && showRootLineKeepSameInCurve"
      label="根节点"
      :model-value="style.rootLineKeepSameInCurve"
      :options="rootLineKeepSameInCurveList"
      @update:model-value="(v) => update('rootLineKeepSameInCurve', v)"
    />
    <SelectField
      v-if="showLineRadius"
      label="圆角大小"
      :model-value="style.lineRadius"
      :options="lineRadiusList.map((r) => ({ name: String(r), value: r }))"
      @update:model-value="(v) => update('lineRadius', v)"
    />
    <SelectField
      v-if="style.lineStyle === 'curve' && showRootLineKeepSameInCurve"
      label="根节点连线起始位置"
      :model-value="style.rootLineStartPositionKeepSameInCurve"
      :options="rootLineStartPosList"
      @update:model-value="(v) => update('rootLineStartPositionKeepSameInCurve', v)"
    />
    <SwitchField
      label="是否显示箭头"
      :model-value="!!style.showLineMarker"
      @update:model-value="(v) => update('showLineMarker', v)"
    />

    <!-- 彩虹线条：点当前条弹出配色选择 -->
    <div class="mm-bs-group">彩虹线条</div>
    <details class="mm-bs-rainbow">
      <summary class="mm-bs-rainbow-cur">
        <span v-if="curRainbowLineColorList" class="mm-bs-colors-bar">
          <span
            v-for="color in curRainbowLineColorList"
            :key="color"
            class="mm-bs-color-item"
            :style="{ backgroundColor: color }"
          />
        </span>
        <span v-else>不使用彩虹线条</span>
      </summary>
      <div class="mm-bs-rainbow-options">
        <div
          v-for="item in rainbowLinesOptions"
          :key="item.value"
          class="mm-bs-rainbow-option"
          @click="updateRainbowLinesConfig(item)"
        >
          <span v-if="item.list" class="mm-bs-colors-bar">
            <span
              v-for="color in item.list"
              :key="color"
              class="mm-bs-color-item"
              :style="{ backgroundColor: color }"
            />
          </span>
          <span v-else>不使用彩虹线条</span>
        </div>
      </div>
    </details>

    <!-- 概要的连线 -->
    <div class="mm-bs-group">概要的连线</div>
    <ColorField
      label="颜色"
      :model-value="colorOf('generalizationLineColor')"
      @update:model-value="(v) => update('generalizationLineColor', v)"
    />
    <SelectField
      label="粗细"
      :model-value="style.generalizationLineWidth"
      :options="lineWidthOptions"
      @update:model-value="(v) => update('generalizationLineWidth', v)"
    />

    <!-- 关联线 -->
    <div class="mm-bs-group">关联线</div>
    <ColorField
      label="颜色"
      :model-value="colorOf('associativeLineColor')"
      @update:model-value="(v) => update('associativeLineColor', v)"
    />
    <SelectField
      label="粗细"
      :model-value="style.associativeLineWidth"
      :options="lineWidthOptions"
      @update:model-value="(v) => update('associativeLineWidth', v)"
    />
    <ColorField
      label="激活颜色"
      :model-value="colorOf('associativeLineActiveColor')"
      @update:model-value="(v) => update('associativeLineActiveColor', v)"
    />
    <SelectField
      label="激活粗细"
      :model-value="style.associativeLineActiveWidth"
      :options="lineWidthOptions"
      @update:model-value="(v) => update('associativeLineActiveWidth', v)"
    />
    <FieldRow label="虚线样式">
      <div class="mm-bs-dasharray">
        <svg
          v-for="item in borderDasharrayList"
          :key="item.value"
          class="mm-bs-dasharray-item"
          :class="{ active: style.associativeLineDasharray === item.value }"
          width="64"
          height="22"
          :title="item.name"
          @click="update('associativeLineDasharray', item.value)"
        >
          <line
            x1="6"
            y1="11"
            x2="58"
            y2="11"
            stroke-width="2"
            :stroke-dasharray="item.value === 'none' ? undefined : item.value"
          />
        </svg>
      </div>
    </FieldRow>

    <!-- 关联线文字 -->
    <div class="mm-bs-group">关联线文字</div>
    <SelectField
      label="字体"
      :model-value="String(style.associativeLineTextFontFamily)"
      :options="fontFamilyList"
      @update:model-value="(v) => update('associativeLineTextFontFamily', v)"
    />
    <SelectField
      label="字号"
      :model-value="style.associativeLineTextFontSize"
      :options="fontSizeOptions"
      @update:model-value="(v) => update('associativeLineTextFontSize', v)"
    />
    <ColorField
      label="颜色"
      :model-value="colorOf('associativeLineTextColor')"
      @update:model-value="(v) => update('associativeLineTextColor', v)"
    />

    <!-- 节点边框风格（受结构限制） -->
    <template v-if="showNodeUseLineStyle">
      <div class="mm-bs-group">节点边框风格</div>
      <SwitchField
        label="是否使用只有底边框的风格"
        :model-value="!!style.nodeUseLineStyle"
        @update:model-value="(v) => update('nodeUseLineStyle', v)"
      />
    </template>

    <!-- 节点内边距 -->
    <div class="mm-bs-group">节点内边距</div>
    <NumberField
      label="水平"
      :model-value="Number(style.paddingX) || 0"
      :min="0"
      :max="200"
      @update:model-value="(v) => update('paddingX', v)"
    />
    <NumberField
      label="垂直"
      :model-value="Number(style.paddingY) || 0"
      :min="0"
      :max="200"
      @update:model-value="(v) => update('paddingY', v)"
    />

    <!-- 图片（节点内图片最大尺寸） -->
    <div class="mm-bs-group">图片</div>
    <NumberField
      label="显示的最大宽度"
      :model-value="Number(style.imgMaxWidth) || 0"
      :min="10"
      :max="500"
      @update:model-value="(v) => update('imgMaxWidth', v)"
    />
    <NumberField
      label="显示的最大高度"
      :model-value="Number(style.imgMaxHeight) || 0"
      :min="10"
      :max="500"
      @update:model-value="(v) => update('imgMaxHeight', v)"
    />

    <!-- 图标 -->
    <div class="mm-bs-group">图标</div>
    <NumberField
      label="大小"
      :model-value="Number(style.iconSize) || 0"
      :min="12"
      :max="50"
      @update:model-value="(v) => update('iconSize', v)"
    />

    <!-- 二级节点外边距（按层级页签） -->
    <div class="mm-bs-group">节点外边距</div>
    <NTabs
      type="segment"
      size="small"
      :value="marginActiveTab"
      @update:value="(v) => ((marginActiveTab = v as 'second' | 'node'), initMarginStyle())"
    >
      <NTabPane name="second" tab="二级节点" />
      <NTabPane name="node" tab="三级及以下节点" />
    </NTabs>
    <NumberField
      label="水平"
      :model-value="Number(style.marginX) || 0"
      :min="0"
      :max="200"
      @update:model-value="(v) => updateMargin('marginX', v)"
    />
    <NumberField
      label="垂直"
      :model-value="Number(style.marginY) || 0"
      :min="0"
      :max="200"
      @update:model-value="(v) => updateMargin('marginY', v)"
    />

    <!-- 外框内边距 -->
    <div class="mm-bs-group">外框内边距</div>
    <NumberField
      label="水平"
      :model-value="outerFramePadding.outerFramePaddingX"
      :min="0"
      :max="200"
      @update:model-value="(v) => updateOuterFramePadding('outerFramePaddingX', v)"
    />
    <NumberField
      label="垂直"
      :model-value="outerFramePadding.outerFramePaddingY"
      :min="0"
      :max="200"
      @update:model-value="(v) => updateOuterFramePadding('outerFramePaddingY', v)"
    />
  </SidebarShell>
</template>
