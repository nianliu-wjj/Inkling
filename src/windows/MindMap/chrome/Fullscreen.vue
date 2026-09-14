<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { logger } from '@/service/logger'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 全屏控件（移植参考端 Fullscreen.vue）。
 * 「全屏查看」= 进入全屏并切只读模式；「全屏编辑」= 仅进入全屏。
 * 退出全屏（Esc / fullscreenchange）时恢复编辑模式并 resize 画布。
 * 全屏对整个窗口文档生效（Tauri WebView2 里即整窗铺满）。
 *
 * 阶段五 Task 5 起由 `MmBottombar.vue` 挂在底栏，类名换成生成层的 `.mm-ctrl-btn`；
 * 两枚按钮不再包一层自有壳（原型底栏的按钮就是 `.mm-ctrl-right` 的直接子项，靠容器 gap 排布）。
 * 原型底栏只有一枚全屏按钮，我们区分「查看（只读）/ 编辑」两枚——能力差异，保留。
 */
const ctx = useMindMap()

async function enterFullscreen(): Promise<void> {
  try {
    if (!document.fullscreenElement) await document.documentElement.requestFullscreen()
  } catch (error) {
    logger.error('mindmap', '进入全屏失败', error)
  }
}

/** 全屏查看：全屏 + 只读。 */
async function fullscreenShow(): Promise<void> {
  await enterFullscreen()
  requireMindMap(ctx).setMode('readonly')
  logger.info('mindmap', '进入全屏查看（只读）')
}

/** 全屏编辑：仅全屏。 */
async function fullscreenEdit(): Promise<void> {
  await enterFullscreen()
  logger.info('mindmap', '进入全屏编辑')
}

/** 退出全屏时恢复编辑模式并重算画布尺寸。 */
function onFullscreenChange(): void {
  if (document.fullscreenElement) return
  const mindMap = ctx.mindMap.value
  if (!mindMap) return
  mindMap.setMode('edit')
  setTimeout(() => mindMap.resize(), 300)
  logger.info('mindmap', '退出全屏，恢复编辑模式')
}

onMounted(() => document.addEventListener('fullscreenchange', onFullscreenChange))
onBeforeUnmount(() => document.removeEventListener('fullscreenchange', onFullscreenChange))
</script>

<template>
  <!-- 两枚按钮是底栏 .mm-ctrl-right 的直接子项（多根节点），顺序与原型一致：查看在前、编辑在后。 -->
  <button type="button" class="mm-ctrl-btn iconfont iconquanping" title="全屏查看（只读）" @click="fullscreenShow" />
  <button type="button" class="mm-ctrl-btn iconfont iconquanping1" title="全屏编辑" @click="fullscreenEdit" />
</template>
