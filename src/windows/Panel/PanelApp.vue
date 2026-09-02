<script setup lang="ts">
import gsap from 'gsap'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import { useSettings } from '@/composables/useData'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { AppEvents, onAppEvent } from '@/service/events'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { CaptureMode } from '@/typings/domain'
import ClipPage from './ClipPage.vue'
import NotePage from './NotePage.vue'
import TodoPage from './TodoPage.vue'

/**
 * 呼出面板：三态合一（笔记 / 粘贴板 / 待办）。
 *
 * 需求 2.1：
 * - 滑入 200ms / 滑出 150ms 的弹性过渡（GSAP）；
 * - 固定宽 480px，高度随内容自适应（120~600px）；
 * - 失焦按设置策略收起：立即 / 延迟 3 秒 / 固定不收起；
 * - **弹窗失焦保护**：任一编辑弹窗打开期间不因失焦收起；全部关闭后若鼠标
 *   已不在面板内，按策略重新计时。
 * - Esc 收起；⌃1/2/3 切换三态。
 */

// 启动瞬间先用缓存主题上色，避免默认深色闪一下再跳变。
applyCachedTheme()

const { settings } = useSettings()
const { applyTheme } = useTheme()

const mode = ref<CaptureMode>('note')
const panel = ref<HTMLElement | null>(null)
/** 打开的弹窗层数：>0 时禁止失焦收起。 */
const modalDepth = ref(0)
/** 鼠标是否在面板内，用于弹窗关闭后判断是否重新计时。 */
const pointerInside = ref(false)

/** 失焦收起的延迟定时器。 */
let collapseTimer: ReturnType<typeof setTimeout> | null = null
/** 高度自适应的观察器。 */
let resizeObserver: ResizeObserver | null = null

/** 面板高度范围，与 Rust 侧 windows.rs 的 PANEL_MIN/MAX_HEIGHT 保持一致。 */
const PANEL_MIN_HEIGHT = 120
const PANEL_MAX_HEIGHT = 600

const MODES: readonly { key: CaptureMode; dot: string; label: string; hotkey: string }[] = [
  { key: 'note', dot: '🔴', label: '笔记', hotkey: '⌃1' },
  { key: 'clipboard', dot: '🟡', label: '粘贴板', hotkey: '⌃2' },
  { key: 'todo', dot: '🟢', label: '待办', hotkey: '⌃3' },
]

/** 后端设置变化时同步主题。 */
watch(
  () => settings.value.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
)

function clearCollapseTimer(): void {
  if (collapseTimer !== null) {
    clearTimeout(collapseTimer)
    collapseTimer = null
  }
}

/** 滑出动画结束后再真正隐藏窗口，避免窗口先消失、动画看不见。 */
function motionAxis(): 'x' | 'y' {
  return settings.value.panel_position === 'left' || settings.value.panel_position === 'right' ? 'x' : 'y'
}

function motionDistance(distance: number): number {
  const position = settings.value.panel_position
  return position === 'bottom' || position === 'right' ? distance : -distance
}

async function hide(): Promise<void> {
  clearCollapseTimer()
  logger.info('panel', '收起面板')

  if (panel.value) {
    await gsap.to(panel.value, {
      [motionAxis()]: motionDistance(12),
      opacity: 0,
      duration: 0.15,
      ease: 'power2.in',
    })
  }
  try {
    await api.windows.panelHide()
  } catch (error) {
    logger.error('panel', '隐藏面板失败', error)
  }
}

/** 滑入：物理弹性（back.out）呼应需求「Spring/Ease-out」。 */
function playEnter(): void {
  if (!panel.value) return
  const axis = motionAxis()
  gsap.fromTo(
    panel.value,
    { [axis]: motionDistance(16), opacity: 0 },
    { [axis]: 0, opacity: 1, duration: 0.2, ease: 'back.out(1.6)' },
  )
}

/**
 * 按设置策略安排收起。
 * 弹窗打开期间直接跳过（弹窗失焦保护）。
 */
function scheduleCollapse(): void {
  if (modalDepth.value > 0) {
    logger.debug('panel', '弹窗打开中，跳过失焦收起')
    return
  }

  const policy = settings.value.collapse_policy
  if (policy === 'never') return

  clearCollapseTimer()
  if (policy === 'immediate') {
    void hide()
    return
  }
  // 默认 3s
  collapseTimer = setTimeout(() => {
    collapseTimer = null
    void hide()
  }, 3000)
}

function onModalToggle(open: boolean): void {
  modalDepth.value = Math.max(0, modalDepth.value + (open ? 1 : -1))
  logger.debug('panel', `弹窗层数 = ${modalDepth.value}`)

  // 弹窗开合都会改变所需窗口高度，立即重新上报。
  void nextTick(reportHeight)

  if (open) {
    // 弹窗打开：取消已在计时的收起。
    clearCollapseTimer()
    return
  }
  // 全部弹窗关闭且鼠标已不在面板内 → 按策略重新计时。
  if (modalDepth.value === 0 && !pointerInside.value) scheduleCollapse()
}

function onKeydown(event: KeyboardEvent): void {
  // 弹窗自己处理 Esc，面板不抢。
  if (event.key === 'Escape' && modalDepth.value === 0) {
    event.preventDefault()
    void hide()
    return
  }

  // ⌃1/2/3 切换三态
  if (event.ctrlKey || event.metaKey) {
    const index = Number(event.key) - 1
    if (index >= 0 && index < MODES.length) {
      event.preventDefault()
      mode.value = MODES[index].key
    }
  }
}

/**
 * 高度自适应：把内容实际高度报给窗口，钳制在 120~600px。
 *
 * 弹窗例外：`.glass` 的 backdrop-filter 让 #panel 成为 fixed 定位子元素的
 * **包含块**，因此编辑弹窗被限制在面板窗口内，而弹窗高度不计入
 * #panel.offsetHeight。若仍按内容高度上报，面板只有 ~200px，
 * 弹窗会被整片截断（只剩标题与首行）。故弹窗打开期间直接用最大高度。
 */
function reportHeight(): void {
  if (!panel.value) return

  const height =
    modalDepth.value > 0
      ? PANEL_MAX_HEIGHT
      : Math.min(PANEL_MAX_HEIGHT, Math.max(PANEL_MIN_HEIGHT, Math.ceil(panel.value.offsetHeight) + 12))

  void api.windows.panelResize(height).catch((error) => {
    logger.error('panel', '调整面板高度失败', error)
  })
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('blur', scheduleCollapse)
  // 重新获得焦点时取消待执行的收起。
  window.addEventListener('focus', clearCollapseTimer)

  if (panel.value) {
    resizeObserver = new ResizeObserver(reportHeight)
    resizeObserver.observe(panel.value)
  }

  playEnter()

  // 后端每次显示面板都会广播，据此重播入场动画并复位到笔记态。
  void onAppEvent(AppEvents.panelShown, () => {
    clearCollapseTimer()
    playEnter()
  })

  logger.info('panel', '面板已挂载')
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('blur', scheduleCollapse)
  window.removeEventListener('focus', clearCollapseTimer)
  resizeObserver?.disconnect()
  clearCollapseTimer()
})

const activeLabel = computed(() => MODES.find((m) => m.key === mode.value)?.label ?? '')
</script>

<template>
  <div
    id="panel"
    ref="panel"
    class="glass"
    :aria-label="`Inkling 呼出面板 · ${activeLabel}`"
    @mouseenter="pointerInside = true"
    @mouseleave="pointerInside = false"
  >
    <!-- 三态圆点导航 -->
    <div class="panel-nav">
      <span
        v-for="item in MODES"
        :key="item.key"
        class="nav-dot"
        :class="{ active: mode === item.key }"
        :title="`${item.label} (${item.hotkey})`"
        @click="mode = item.key"
        >{{ item.dot }}</span
      >
      <span class="panel-hint">Esc 收起</span>
    </div>

    <!-- 三态页面：用 v-show 保留各自状态（如笔记草稿、搜索关键词） -->
    <NotePage v-show="mode === 'note'" @modal="onModalToggle" />
    <ClipPage v-show="mode === 'clipboard'" @modal="onModalToggle" />
    <TodoPage v-show="mode === 'todo'" @modal="onModalToggle" />

    <ToastHost />
  </div>
</template>
