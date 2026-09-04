<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { platform } from '@/constants/platform'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 主窗口的窗口控件（最小化 / 最大化切换 / 关闭）。
 *
 * 主窗口是无边框窗口（decorations(false)），系统不画标题栏，控件必须自绘。
 * 按平台各自的原生样式渲染：
 * - macOS：左上角红黄绿三个圆点，悬浮时才显出符号；
 * - Windows / 其他：右上角三个方形按钮，悬浮变底色，关闭键变红。
 *
 * 位置差异（mac 在左、Windows 在右）由调用方按 `controlsOnLeft` 决定插入位置，
 * 本组件只负责按钮本身。
 *
 * 「关闭」的语义沿用既有行为——**隐藏到托盘而非退出进程**：
 * 本应用常驻托盘，剪贴板监听与提醒调度都要继续跑（见 main.rs 的 CloseRequested 处理）。
 * 与「最小化」的区别是最小化仍留在任务栏，关闭后只能从托盘唤起。
 */

/** 最大化状态：决定最大化按钮的图标与提示文案。 */
const maximized = ref(false)

/** 窗口尺寸变化的取消监听函数。 */
let stopResize: (() => void) | null = null

async function syncMaximized(): Promise<void> {
  try {
    maximized.value = await api.windows.mainIsMaximized()
    // 最大化时取消窗口圆角：铺满屏幕的窗口留着圆角会在四角露出桌面。
    const root = document.documentElement
    if (maximized.value) root.setAttribute('data-maximized', '')
    else root.removeAttribute('data-maximized')
  } catch (error) {
    logger.error('window-controls', '查询最大化状态失败', error)
  }
}

async function minimize(): Promise<void> {
  try {
    await api.windows.minimizeMain()
  } catch (error) {
    logger.error('window-controls', '最小化失败', error)
  }
}

async function toggleMaximize(): Promise<void> {
  try {
    maximized.value = await api.windows.toggleMaximizeMain()
    const root = document.documentElement
    if (maximized.value) root.setAttribute('data-maximized', '')
    else root.removeAttribute('data-maximized')
  } catch (error) {
    logger.error('window-controls', '切换最大化失败', error)
  }
}

async function close(): Promise<void> {
  try {
    await api.windows.hideMain()
  } catch (error) {
    logger.error('window-controls', '隐藏主窗口失败', error)
  }
}

onMounted(() => {
  void syncMaximized()
  // 双击标题栏、拖到屏幕顶部等系统手势也会改变最大化状态，靠 resize 兜住。
  const onResize = (): void => void syncMaximized()
  window.addEventListener('resize', onResize)
  stopResize = () => window.removeEventListener('resize', onResize)
})

onBeforeUnmount(() => stopResize?.())
</script>

<template>
  <!-- no-drag：标题栏整体是拖拽区，按钮必须排除在外，否则点不动 -->
  <div class="win-controls no-drag" :class="`win-controls-${platform === 'macos' ? 'mac' : 'win'}`">
    <template v-if="platform === 'macos'">
      <!-- macOS 惯例的排列次序：关闭 / 最小化 / 最大化 -->
      <button type="button" class="mac-dot mac-close" title="关闭（隐藏到托盘）" @click="close">
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M3.5 3.5l5 5M8.5 3.5l-5 5" />
        </svg>
      </button>
      <button type="button" class="mac-dot mac-min" title="最小化" @click="minimize">
        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M3 6h6" /></svg>
      </button>
      <button type="button" class="mac-dot mac-max" :title="maximized ? '还原' : '最大化'" @click="toggleMaximize">
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M6 3v6M3 6h6" />
        </svg>
      </button>
    </template>

    <template v-else>
      <!-- Windows 惯例的排列次序：最小化 / 最大化 / 关闭 -->
      <button type="button" class="win-btn" title="最小化" @click="minimize">
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" /></svg>
      </button>
      <button type="button" class="win-btn" :title="maximized ? '向下还原' : '最大化'" @click="toggleMaximize">
        <!-- 还原态用双层方框，与系统一致 -->
        <svg v-if="maximized" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M2.5 0.5h7v7h-7z" />
          <path d="M0.5 2.5v7h7" />
        </svg>
        <svg v-else viewBox="0 0 10 10" aria-hidden="true"><path d="M0.5 0.5h9v9h-9z" /></svg>
      </button>
      <button type="button" class="win-btn win-btn-close" title="关闭（隐藏到托盘）" @click="close">
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" /></svg>
      </button>
    </template>
  </div>
</template>
