<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { logger } from '@/service/logger'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 小地图导航（移植参考端 Navigator.vue）。
 *
 * 订阅自定义事件 `toggleMiniMap`（由 NavigatorToolbar 发出）开合；数据/视图变化时
 * 500ms 节流重绘。缩略图与视口框由 `mindMap.miniMap.calculationMiniMap(w,h)` 计算；
 * 拖拽平移与视口框拖动直接转交 miniMap 的鼠标事件方法；库事件
 * `mini_map_view_box_position_change` 实时更新视口框位置（该事件仅本组件关心，直接
 * 用 mindMap.on 订阅，不经全局 bus）。
 */
const ctx = useMindMap()
const { bus } = ctx

const showMiniMap = ref(false)
const width = ref(0)
const boxWidth = ref(0)
const boxHeight = ref(0)
const svgBoxScale = ref(1)
const svgBoxLeft = ref(0)
const svgBoxTop = ref(0)
const mindMapImg = ref('')
const withTransition = ref(true)
const viewBoxStyle = reactive<Record<string, number | string>>({ left: 0, top: 0, right: 0, bottom: 0 })

const navigatorBox = ref<HTMLElement | null>(null)

let redrawTimer: ReturnType<typeof setTimeout> | null = null
let sizeTimer: ReturnType<typeof setTimeout> | null = null

/** 读取容器宽高（缩略图计算需要）。 */
function measure(): void {
  if (!navigatorBox.value) return
  const rect = navigatorBox.value.getBoundingClientRect()
  boxWidth.value = rect.width
  boxHeight.value = rect.height
}

/** 渲染缩略图：拿到图片 URL、视口框样式与缩放定位。 */
function drawMiniMap(): void {
  const mindMap = requireMindMap(ctx)
  const {
    getImgUrl,
    viewBoxStyle: vbs,
    miniMapBoxScale,
    miniMapBoxLeft,
    miniMapBoxTop,
  } = mindMap.miniMap.calculationMiniMap(boxWidth.value, boxHeight.value)
  getImgUrl((img: string) => (mindMapImg.value = img))
  Object.assign(viewBoxStyle, vbs)
  svgBoxScale.value = miniMapBoxScale
  svgBoxLeft.value = miniMapBoxLeft
  svgBoxTop.value = miniMapBoxTop
}

/** 开合小地图（参考端 toggle_mini_map，本项目事件名 toggleMiniMap）。 */
function onToggle(show: boolean): void {
  showMiniMap.value = show
  logger.info('mindmap', `小地图${show ? '开启' : '关闭'}`)
  if (!show) return
  void nextTick(() => {
    if (navigatorBox.value) {
      measure()
      drawMiniMap()
    }
  })
}

/** 数据 / 视图变化：500ms 节流重绘（仅在显示时）。 */
function onDataChange(): void {
  if (!showMiniMap.value) return
  if (redrawTimer) clearTimeout(redrawTimer)
  redrawTimer = setTimeout(() => drawMiniMap(), 500)
}

/** 窗口尺寸变化：300ms 防抖后重算容器宽度并重绘。 */
function onResize(): void {
  if (sizeTimer) clearTimeout(sizeTimer)
  sizeTimer = setTimeout(() => {
    width.value = Math.min(window.innerWidth - 80, 370)
    void nextTick(() => {
      if (!showMiniMap.value) return
      measure()
      drawMiniMap()
    })
  }, 300)
}

// —— 鼠标事件转交 miniMap ——
function onMousedown(e: MouseEvent): void {
  requireMindMap(ctx).miniMap.onMousedown(e)
}
function onMousemove(e: MouseEvent): void {
  requireMindMap(ctx).miniMap.onMousemove(e)
}
function onMouseup(e: MouseEvent): void {
  if (!withTransition.value) withTransition.value = true
  ctx.mindMap.value?.miniMap?.onMouseup(e)
}
function onViewBoxMousedown(e: MouseEvent): void {
  requireMindMap(ctx).miniMap.onViewBoxMousedown(e)
}
function onViewBoxMousemove(e: MouseEvent): void {
  requireMindMap(ctx).miniMap.onViewBoxMousemove(e)
}

/** 视口框位置变化（拖动画布时实时更新，关闭过渡以跟手）。 */
function onViewBoxPositionChange(pos: { left: number; right: number; top: number; bottom: number }): void {
  withTransition.value = false
  viewBoxStyle.left = pos.left
  viewBoxStyle.right = pos.right
  viewBoxStyle.top = pos.top
  viewBoxStyle.bottom = pos.bottom
}

const offs: Array<() => void> = []
onMounted(() => {
  width.value = Math.min(window.innerWidth - 80, 370)
  window.addEventListener('resize', onResize)
  window.addEventListener('mouseup', onMouseup)
  offs.push(
    bus.on('toggleMiniMap', (show) => onToggle(show as boolean)),
    bus.on('data_change', onDataChange),
    bus.on('view_data_change', onDataChange),
    bus.on('node_tree_render_end', onDataChange),
  )
  requireMindMap(ctx).on('mini_map_view_box_position_change', onViewBoxPositionChange)
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', onResize)
  window.removeEventListener('mouseup', onMouseup)
  offs.forEach((off) => off())
  ctx.mindMap.value?.off?.('mini_map_view_box_position_change', onViewBoxPositionChange)
  if (redrawTimer) clearTimeout(redrawTimer)
  if (sizeTimer) clearTimeout(sizeTimer)
})
</script>

<template>
  <div
    v-if="showMiniMap"
    ref="navigatorBox"
    class="mm-minimap mm-panel"
    :style="{ width: width + 'px' }"
    @mousedown="onMousedown"
    @mousemove="onMousemove"
  >
    <div
      class="mm-minimap-svg"
      :style="{ transform: `scale(${svgBoxScale})`, left: svgBoxLeft + 'px', top: svgBoxTop + 'px' }"
    >
      <img :src="mindMapImg" alt="缩略图" @mousedown.prevent />
    </div>
    <div
      class="mm-minimap-viewbox"
      :class="{ 'with-transition': withTransition }"
      :style="viewBoxStyle"
      @mousedown.stop="onViewBoxMousedown"
      @mousemove="onViewBoxMousemove"
    />
  </div>
</template>
