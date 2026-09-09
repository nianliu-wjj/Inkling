<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 画布滚动条（移植参考端 Scrollbar.vue）。
 *
 * 仅在 `localConfig.isShowScrollbar` 开启时渲染（此时 Scrollbar 插件已由 MindMapApp 动态挂载）。
 * 竖向 / 横向两条：把轨道尺寸告诉插件（setScrollBarWrapSize），订阅 `scrollbar_change`
 * 更新滑块位置与大小；滑块按下 / 轨道点击转交插件（移动与松开由插件内部绑定 mindMap 事件处理）。
 */
const ctx = useMindMap()
const { bus, localConfig } = ctx

const verticalRef = ref<HTMLElement | null>(null)
const horizontalRef = ref<HTMLElement | null>(null)
const verticalStyle = reactive<Record<string, string>>({ top: '0%', height: '0%' })
const horizontalStyle = reactive<Record<string, string>>({ left: '0%', width: '0%' })

let resizeTimer: ReturnType<typeof setTimeout> | null = null

/** 把两条轨道的宽高告诉插件（插件据此换算滑块比例）。 */
function setWrapSize(): void {
  const mindMap = ctx.mindMap.value
  if (!mindMap?.scrollbar || !horizontalRef.value || !verticalRef.value) return
  const { width } = horizontalRef.value.getBoundingClientRect()
  const { height } = verticalRef.value.getBoundingClientRect()
  mindMap.scrollbar.setScrollBarWrapSize(width, height)
}

function onResize(): void {
  if (resizeTimer) clearTimeout(resizeTimer)
  resizeTimer = setTimeout(setWrapSize, 300)
}

/** 插件回传的滑块位置 / 大小（百分比）。 */
function updateScrollbar(payload: {
  vertical: { top: number; height: number }
  horizontal: { left: number; width: number }
}): void {
  verticalStyle.top = payload.vertical.top + '%'
  verticalStyle.height = payload.vertical.height + '%'
  horizontalStyle.left = payload.horizontal.left + '%'
  horizontalStyle.width = payload.horizontal.width + '%'
}

function onVerticalMousedown(e: MouseEvent): void {
  requireMindMap(ctx).scrollbar.onMousedown(e, 'vertical')
}
function onVerticalClick(e: MouseEvent): void {
  requireMindMap(ctx).scrollbar.onClick(e, 'vertical')
}
function onHorizontalMousedown(e: MouseEvent): void {
  requireMindMap(ctx).scrollbar.onMousedown(e, 'horizontal')
}
function onHorizontalClick(e: MouseEvent): void {
  requireMindMap(ctx).scrollbar.onClick(e, 'horizontal')
}

const offs: Array<() => void> = []
onMounted(() => {
  setWrapSize()
  window.addEventListener('resize', onResize)
  offs.push(
    bus.on('scrollbar_change', (payload) =>
      updateScrollbar(
        payload as {
          vertical: { top: number; height: number }
          horizontal: { left: number; width: number }
        },
      ),
    ),
  )
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', onResize)
  offs.forEach((off) => off())
  if (resizeTimer) clearTimeout(resizeTimer)
})
</script>

<template>
  <div v-if="localConfig.isShowScrollbar" class="mm-scrollbar">
    <!-- 竖向 -->
    <div ref="verticalRef" class="mm-scrollbar-track vertical" @click="onVerticalClick">
      <div class="mm-scrollbar-inner" :style="verticalStyle" @click.stop @mousedown="onVerticalMousedown" />
    </div>
    <!-- 横向 -->
    <div ref="horizontalRef" class="mm-scrollbar-track horizontal" @click="onHorizontalClick">
      <div class="mm-scrollbar-inner" :style="horizontalStyle" @click.stop @mousedown="onHorizontalMousedown" />
    </div>
  </div>
</template>
