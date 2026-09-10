import { animate, stagger, type JSAnimation } from 'animejs'
import { logger } from '@/service/logger'
import { readMotionTokens } from './tokens'

/**
 * 动效预设：调用方不接触 animejs API，只拿 Promise<boolean>（true = 播完，false = 被打断）。
 *
 * 硬约束（本项目实测踩坑，见 spec §8.3）：
 * 1. 带 backdrop-filter 的 .glass 只允许在 enter/exit 这类瞬时动效里加 transform，静止态必须无 transform。
 * 2. 入场一律 JS 驱动，不用 CSS animation + fill-mode: both——WebView2 在窗口隐藏后挂起动画，
 *    再显示时可能停在透明态。
 * 3. 只动 transform 与 opacity。
 * 4. 同一锚点同一时刻只有一条动画：新预设先取消旧动画并清内联样式。
 */

export type Axis = 'x' | 'y'

export interface SlideOptions {
  axis: Axis
  /** 位移距离（px），正负即方向：入场从 distance 滑到 0，收起从 0 滑到 distance。 */
  distance: number
  /** 入场起始缩放（如 0.97）；不传则不缩放。 */
  scale?: number
}

type AnimateParams = NonNullable<Parameters<typeof animate>[1]>

interface Running {
  animation: JSAnimation
  targets: HTMLElement[]
  settle: (completed: boolean) => void
}

/** 键为「动画的锚点元素」（单元素动效即元素本身，错落动效为容器）。 */
const running = new WeakMap<Element, Running>()

function clearInline(el: HTMLElement): void {
  el.style.transform = ''
  el.style.opacity = ''
  el.classList.remove('is-animating')
}

/** 取消锚点上进行中的动画并复位其目标元素；被取消的 Promise 以 false 结束。 */
function cancelRunning(anchor: Element): void {
  const prev = running.get(anchor)
  if (!prev) return
  running.delete(anchor)
  prev.animation.cancel()
  prev.targets.forEach(clearInline)
  prev.settle(false)
}

/**
 * 通用执行：keepEnd = true 时播完后保留终态内联样式（收起后元素应停在透明态，直到下次入场重置）。
 */
function run(anchor: Element, targets: HTMLElement[], params: AnimateParams, keepEnd = false): Promise<boolean> {
  cancelRunning(anchor)
  targets.forEach((el) => el.classList.add('is-animating'))
  return new Promise<boolean>((resolve) => {
    let record: Running | undefined
    const animation = animate(targets, {
      ...params,
      onComplete: () => {
        if (record && running.get(anchor) === record) running.delete(anchor)
        targets.forEach((el) => (keepEnd ? el.classList.remove('is-animating') : clearInline(el)))
        resolve(true)
      },
    })
    record = { animation, targets, settle: resolve }
    running.set(anchor, record)
  })
}

/** 入场：位移 + 淡入（+ 可选缩放），--dur-slow，outBack(1.6)——与原型 showPanel 一致。 */
export function enter(el: HTMLElement, opts: SlideOptions): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) {
    cancelRunning(el)
    clearInline(el)
    return Promise.resolve(true)
  }
  const prop = opts.axis === 'x' ? 'translateX' : 'translateY'
  logger.debug('motion', `enter ${prop} ${opts.distance}px → 0`)
  return run(el, [el], {
    [prop]: [opts.distance, 0],
    opacity: [0, 1],
    ...(opts.scale === undefined ? {} : { scale: [opts.scale, 1] }),
    duration: tokens.slow,
    ease: 'outBack(1.6)',
  })
}

/** 收起：位移 + 淡出，--dur-base，inQuad——与原型 hidePanel 一致；播完保留透明终态。 */
export function exit(el: HTMLElement, opts: SlideOptions): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) {
    cancelRunning(el)
    el.style.opacity = '0'
    return Promise.resolve(true)
  }
  const prop = opts.axis === 'x' ? 'translateX' : 'translateY'
  logger.debug('motion', `exit ${prop} 0 → ${opts.distance}px`)
  return run(
    el,
    [el],
    {
      [prop]: [0, opts.distance],
      opacity: [1, 0],
      duration: tokens.base,
      ease: 'inQuad',
    },
    true,
  )
}

/**
 * 列表错落入场：targets 逐项延迟 --stagger，位移 8px + 淡入，--dur-base，--ease-out。
 * 先同步把目标置为透明，避免动画首帧前闪现终态。锚点为容器：同一容器新一轮会取消上一轮。
 */
export function staggerIn(
  container: HTMLElement,
  targets: HTMLElement[],
  opts: { distance?: number } = {},
): Promise<boolean> {
  const tokens = readMotionTokens()
  if (!targets.length) return Promise.resolve(true)
  if (tokens.reduced) {
    cancelRunning(container)
    targets.forEach(clearInline)
    return Promise.resolve(true)
  }
  cancelRunning(container)
  targets.forEach((el) => {
    el.style.opacity = '0'
  })
  return run(container, targets, {
    translateY: [opts.distance ?? 8, 0],
    opacity: [0, 1],
    duration: tokens.base,
    delay: stagger(tokens.stagger),
    ease: tokens.easeOut,
  })
}

/** 按压/勾选回弹：scale .95 → 1，--dur-fast，--ease-spring。 */
export function pop(el: HTMLElement): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) return Promise.resolve(true)
  return run(el, [el], { scale: [0.95, 1], duration: tokens.fast, ease: tokens.easeSpring })
}

/** 同容器内容替换：旧元素淡出（--dur-fast），新元素淡入（--dur-base）。 */
export async function crossfade(outEl: HTMLElement, inEl: HTMLElement): Promise<boolean> {
  const tokens = readMotionTokens()
  if (tokens.reduced) return true
  const [a, b] = await Promise.all([
    run(outEl, [outEl], { opacity: [1, 0], duration: tokens.fast, ease: tokens.easeOut }, true),
    run(inEl, [inEl], { opacity: [0, 1], duration: tokens.base, ease: tokens.easeOut }),
  ])
  return a && b
}
