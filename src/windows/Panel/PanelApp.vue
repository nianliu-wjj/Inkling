<script setup lang="ts">
import gsap from 'gsap'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import { useSettings } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
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
applyCachedGlass()

const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()

const mode = ref<CaptureMode>('note')
const panel = ref<HTMLElement | null>(null)
/** 面板内弹窗的层数：>0 时禁止失焦收起。 */
const modalDepth = ref(0)
/**
 * 独立编辑窗口（editor）是否打开。
 *
 * 待办新增 / 编辑用的是铺满屏幕的独立窗口，它一拿到焦点面板就会 blur；
 * 面板必须在此期间保持展开，否则用户填写时面板已经收起，关闭后无处可回。
 */
const externalEditorOpen = ref(false)
/**
 * 鼠标是否在面板窗口内，用于弹窗关闭后判断是否重新计时。
 * 编辑弹窗 Teleport 到 body 后不再是 #panel 的后代，因此按整个文档而非 #panel 判断，
 * 否则弹窗一盖住 #panel 就会触发 mouseleave，弹窗关闭后面板会被误判为「鼠标已离开」而收起。
 */
const pointerInside = ref(false)
/** 当前被高度观察器跟踪的弹窗元素，弹窗开合时重新同步。 */
const observedModals = new Set<HTMLElement>()

/** 失焦收起的延迟定时器。 */
let collapseTimer: ReturnType<typeof setTimeout> | null = null
/** 高度自适应的观察器。 */
let resizeObserver: ResizeObserver | null = null
/**
 * DOM 变化观察器。
 *
 * 各页数据是异步加载的，列表渲染出来时 ResizeObserver 未必回调（面板此刻可能还没显示，
 * WebView 处于挂起状态），高度就会停在「数据到达前」的值，底部内容被窗口边界切掉。
 * 结构变化必然经过 DOM，用它补一次上报。
 */
let mutationObserver: MutationObserver | null = null
/** 上报的合帧句柄：一次 DOM 批量变更只发一次 IPC。 */
let reportFrame: number | null = null

/** 面板高度范围，与 Rust 侧 windows.rs 的 PANEL_MIN/MAX_HEIGHT 保持一致。 */
const PANEL_MIN_HEIGHT = 120
const PANEL_MAX_HEIGHT = 600
/** 内容高度之外预留给窗口的余量（面板阴影与边框）。 */
const PANEL_HEIGHT_PADDING = 12

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

// 玻璃质感与主题同源：后端设置变化时一并同步。
watch(
  () => settings.value.glass_level,
  (level) => applyGlass(level),
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

/**
 * 滑入：物理弹性（back.out）呼应需求「Spring/Ease-out」。
 *
 * 这是全项目唯一保留 GSAP 的地方——弹性曲线用 CSS 表达不了，
 * 而窗口入场只此一处。其余动效一律走 styles/motion.css 的 CSS 令牌，
 * 避免四个独立 Vue app 都为一条曲线背上整个动画库的启动开销。
 */
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
  if (externalEditorOpen.value) {
    logger.debug('panel', '独立编辑窗口打开中，跳过失焦收起')
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

/**
 * 让高度观察器跟踪当前所有弹窗：弹窗在面板窗口里是整页编辑器，其内容增减
 * （添加标签、拖高备注等）同样需要驱动窗口高度变化。
 */
function syncModalObservers(): void {
  if (!resizeObserver) return
  const current = Array.from(document.querySelectorAll<HTMLElement>('.modal-shell'))
  for (const element of observedModals) {
    if (!current.includes(element)) {
      resizeObserver.unobserve(element)
      observedModals.delete(element)
    }
  }
  for (const element of current) {
    if (!observedModals.has(element)) {
      resizeObserver.observe(element)
      observedModals.add(element)
    }
  }
}

function onModalToggle(open: boolean): void {
  modalDepth.value = Math.max(0, modalDepth.value + (open ? 1 : -1))
  logger.debug('panel', `弹窗层数 = ${modalDepth.value}`)

  // 弹窗开合都会改变所需窗口高度：等 DOM 更新后重新跟踪弹窗并立即上报。
  void nextTick(() => {
    syncModalObservers()
    reportHeight()
  })

  if (open) {
    // 弹窗打开：取消已在计时的收起。
    clearCollapseTimer()
    return
  }
  // 全部弹窗关闭且鼠标已不在面板内 → 按策略重新计时。
  if (modalDepth.value === 0 && !pointerInside.value) scheduleCollapse()
}

/** 待办编辑窗口打开：取消已在计时的收起，并进入保护状态。 */
function onExternalEditorOpen(): void {
  externalEditorOpen.value = true
  clearCollapseTimer()
  logger.debug('panel', '独立编辑窗口已打开，暂停失焦收起')
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
 * 弹窗打开期间，弹窗在面板窗口中以整页编辑器的形式铺满窗口并盖住 #panel
 * （见 window-fit.css），此时窗口高度应跟随弹窗而非 #panel；
 * 多层弹窗取最高者。弹窗自身的 max-height 已按 PANEL_MAX_HEIGHT 收口，
 * 超出部分在弹窗内部滚动，因此这里量到的 offsetHeight 不会形成反馈回路。
 */
/** 合帧上报：把同一帧内的多次观察器回调合并成一次 IPC。 */
function scheduleReport(): void {
  if (reportFrame !== null) return
  reportFrame = requestAnimationFrame(() => {
    reportFrame = null
    reportHeight()
  })
}

function reportHeight(): void {
  if (!panel.value) return

  const modals = Array.from(document.querySelectorAll<HTMLElement>('.modal-shell'))
  // 只量元素自身，绝不能把 #app 拿进来：#app 是 height: 100vh 的容器，
  // 它的 scrollHeight 恒 ≥ 窗口高度，据此上报会「窗口越高、量得越高」地无限膨胀。
  const measure = (element: HTMLElement): number => Math.max(element.offsetHeight, element.scrollHeight)
  const contentHeight = modals.length ? Math.max(...modals.map(measure)) : measure(panel.value)
  const height = Math.min(PANEL_MAX_HEIGHT, Math.max(PANEL_MIN_HEIGHT, Math.ceil(contentHeight) + PANEL_HEIGHT_PADDING))

  void api.windows.panelResize(height).catch((error) => {
    logger.error('panel', '调整面板高度失败', error)
  })
}

function onPointerEnter(): void {
  pointerInside.value = true
}

function onPointerLeave(): void {
  pointerInside.value = false
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('blur', scheduleCollapse)
  // 重新获得焦点时取消待执行的收起。
  window.addEventListener('focus', clearCollapseTimer)
  document.documentElement.addEventListener('mouseenter', onPointerEnter)
  document.documentElement.addEventListener('mouseleave', onPointerLeave)

  if (panel.value) {
    resizeObserver = new ResizeObserver(scheduleReport)
    resizeObserver.observe(panel.value)
    mutationObserver = new MutationObserver(scheduleReport)
    mutationObserver.observe(panel.value, { childList: true, subtree: true, characterData: true })
  }

  playEnter()

  // 后端每次显示面板都会广播，据此重播入场动画并复位到笔记态。
  void onAppEvent(AppEvents.panelShown, () => {
    clearCollapseTimer()
    playEnter()
  })

  // 独立编辑窗口关闭 → 解除保护；若此时鼠标已不在面板上，按策略重新计时。
  void onAppEvent(AppEvents.editorClosed, () => {
    externalEditorOpen.value = false
    logger.debug('panel', '独立编辑窗口已关闭，恢复失焦收起')
    if (!pointerInside.value) scheduleCollapse()
  })

  logger.info('panel', '面板已挂载')
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('blur', scheduleCollapse)
  window.removeEventListener('focus', clearCollapseTimer)
  document.documentElement.removeEventListener('mouseenter', onPointerEnter)
  document.documentElement.removeEventListener('mouseleave', onPointerLeave)
  resizeObserver?.disconnect()
  mutationObserver?.disconnect()
  observedModals.clear()
  if (reportFrame !== null) cancelAnimationFrame(reportFrame)
  clearCollapseTimer()
})

const activeLabel = computed(() => MODES.find((m) => m.key === mode.value)?.label ?? '')
</script>

<template>
  <!-- modal-open：弹窗作为整页编辑器盖在面板之上时，把面板本体隐去，避免透过毛玻璃叠影。 -->
  <div
    id="panel"
    ref="panel"
    class="glass"
    :class="{ 'modal-open': modalDepth > 0 }"
    :aria-label="`Inkling 呼出面板 · ${activeLabel}`"
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
    <TodoPage v-show="mode === 'todo'" @external-editor="onExternalEditorOpen" />

    <ToastHost />
  </div>
</template>
