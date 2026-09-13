<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch, type CSSProperties } from 'vue'
import { enter } from '@/motion'
import { logger } from '@/service/logger'
import { anchorBeside, type AnchorResult } from '@/utils/anchor'

/**
 * 删除二次确认浮层（原型 body 级唯一 `#cardConfirm`，spec §4.2）。
 *
 * - `targetId` 非空时 Teleport 到 body 渲染，按 `container.querySelector('[data-id="…"]')` 反查卡片，
 *   用 anchorBeside() 摆到卡片右侧（右侧不够翻左侧 `.flip`；两侧都不够按 `fallback`（默认 'below'：卡片下方箭头朝上，
 *   下方放不下翻到上方 `.above` 箭头朝下——主窗口与面板一致，spec D27 / 4B D29）；
 * - 首次显示横向 10px 滑入（--dur-fast）；浮层自身尺寸变化（ResizeObserver）与列表重渲染
 *   （容器 MutationObserver，childList + subtree）合帧重定位；卡片消失 → emit cancel；
 * - 任意滚动（capture）与窗口 resize → emit cancel（原型 app.js:209–210）；
 * - Esc → emit cancel：document 捕获阶段监听并 stopPropagation，主窗口各页没有 Esc 链也能关；面板里则先于
 *   PanelApp 的窗口冒泡 Esc 处理器吃掉这一次按键（与 ModalShell / TodoEditorPanel 同法），Esc 链仍是「确认 → 收面板」两步。
 *
 * 一个列表页 / TodoTree 各渲染一个实例；同一窗口内同一时刻只会有一个 targetId 非空（v-if 互斥的视图）。
 */
const props = withDefaults(
  defineProps<{
    /** 文案，四种：确认删除该笔记？ / 该条目？ / 该待办事项？ / 该子任务？ */
    text: string
    /** 待确认卡片的 data-id；null = 隐藏。 */
    targetId: string | null
    /** 列表容器：反查卡片与观察重渲染。 */
    container: HTMLElement | null
    /** 左右都放不下时的兜底：'center' 原型 / 'below' D27（主窗口与面板都用它，D29）。 */
    fallback?: 'center' | 'below'
  }>(),
  { fallback: 'below' },
)

const emit = defineEmits<{ (e: 'confirm'): void; (e: 'cancel'): void }>()

const popRef = ref<HTMLElement | null>(null)
const position = ref<AnchorResult>({ left: 0, top: 0, placement: 'right', caretY: 28 })

const style = computed<CSSProperties>(() => ({
  left: `${position.value.left}px`,
  top: `${position.value.top}px`,
  '--caret-y': `${position.value.caretY}px`,
}))

let resizeObserver: ResizeObserver | null = null
let mutationObserver: MutationObserver | null = null
let frame = 0

function findCard(): HTMLElement | null {
  if (!props.container || !props.targetId) return null
  return props.container.querySelector<HTMLElement>(`[data-id="${CSS.escape(props.targetId)}"]`)
}

/** 反查卡片并定位；找不到卡片（已被删除 / 列表已切换）则取消确认态。 */
function reposition(first: boolean): void {
  const pop = popRef.value
  if (!pop || !props.targetId) return
  const card = findCard()
  if (!card) {
    logger.debug('card-confirm', `卡片 ${props.targetId} 已不在列表，清除确认态`)
    emit('cancel')
    return
  }
  const rect = card.getBoundingClientRect()
  position.value = anchorBeside(
    { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
    {
      size: { width: pop.offsetWidth, height: pop.offsetHeight },
      viewport: { width: window.innerWidth, height: window.innerHeight },
      align: 'center',
      fallback: props.fallback,
    },
  )
  if (first) {
    logger.debug('card-confirm', `显示 id=${props.targetId} placement=${position.value.placement}`)
    // 原型：opacity 0、x ∓10 → 0，.15s
    void enter(pop, { axis: 'x', distance: position.value.placement === 'left' ? 10 : -10, duration: 'fast' })
  }
}

/** Observer 回调合帧：一轮重渲染可能触发多次变更，只定位一次。 */
function scheduleReposition(): void {
  if (frame) return
  frame = requestAnimationFrame(() => {
    frame = 0
    reposition(false)
  })
}

function onScroll(): void {
  emit('cancel')
}

function onResize(): void {
  emit('cancel')
}

/** Esc 关闭确认：capture 阶段拦截并阻断冒泡，不让窗口级 Esc 处理器（PanelApp 收面板）在同一次按键里再触发。 */
function onKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Escape') return
  event.stopPropagation()
  logger.debug('card-confirm', 'Esc 取消确认')
  emit('cancel')
}

function attach(): void {
  detach()
  if (popRef.value) {
    resizeObserver = new ResizeObserver(scheduleReposition)
    resizeObserver.observe(popRef.value)
  }
  if (props.container) {
    mutationObserver = new MutationObserver(scheduleReposition)
    mutationObserver.observe(props.container, { childList: true, subtree: true })
  }
  window.addEventListener('scroll', onScroll, true)
  window.addEventListener('resize', onResize)
  document.addEventListener('keydown', onKeydown, true)
}

function detach(): void {
  resizeObserver?.disconnect()
  resizeObserver = null
  mutationObserver?.disconnect()
  mutationObserver = null
  window.removeEventListener('scroll', onScroll, true)
  window.removeEventListener('resize', onResize)
  document.removeEventListener('keydown', onKeydown, true)
  if (frame) {
    cancelAnimationFrame(frame)
    frame = 0
  }
}

watch(
  () => [props.targetId, props.container] as const,
  async ([id]) => {
    if (!id) {
      detach()
      return
    }
    // 等 Teleport 内容挂载出真实尺寸再定位，否则 offsetWidth 为 0。
    await nextTick()
    // ask → 立刻 cancel 时 targetId 已变，不能再为一个已不存在的气泡挂监听。
    if (props.targetId !== id) return
    reposition(true)
    attach()
  },
  { immediate: true, flush: 'post' },
)

onBeforeUnmount(detach)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.targetId"
      id="cardConfirm"
      ref="popRef"
      role="alertdialog"
      aria-live="polite"
      :class="{
        flip: position.placement === 'left',
        below: position.placement === 'below',
        above: position.placement === 'above',
      }"
      :style="style"
    >
      <div class="cc-caret" />
      <span class="card-confirm-text">{{ props.text }}</span>
      <button type="button" class="btn tiny danger" @click.stop="emit('confirm')">删除</button>
      <button type="button" class="btn tiny ghost" @click.stop="emit('cancel')">取消</button>
    </div>
  </Teleport>
</template>
