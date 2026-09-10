import type { Directive } from 'vue'
import { logger } from '@/service/logger'
import { staggerIn } from './presets'

/** 每个容器最多错落的项数，其余立即显示（长列表不能让用户等一串入场）。 */
const LIMIT = 12

/** 容器 → 已经出现过的直接子元素。只有首次出现的子元素才入场，删除/重排不重播。 */
const seenByContainer = new WeakMap<HTMLElement, WeakSet<Element>>()

function reveal(container: HTMLElement): void {
  let seen = seenByContainer.get(container)
  if (!seen) {
    seen = new WeakSet()
    seenByContainer.set(container, seen)
  }
  const fresh: HTMLElement[] = []
  for (const child of Array.from(container.children)) {
    if (!(child instanceof HTMLElement) || seen.has(child)) continue
    seen.add(child)
    fresh.push(child)
  }
  if (!fresh.length) return
  logger.debug('motion', `列表新增 ${fresh.length} 项，错落入场前 ${Math.min(fresh.length, LIMIT)} 项`)
  void staggerIn(container, fresh.slice(0, LIMIT))
}

/**
 * v-stagger-list：容器内新出现的直接子元素逐项错落入场。
 *
 * - mounted：首屏全部子元素入场（受 LIMIT 限制）；
 * - updated：宿主组件每次重渲染后检查，只对 Vue 新建的 DOM 节点入场（keyed v-for 复用的节点不动）；
 * - 不用 <TransitionGroup>：它依赖 CSS 类与 fill 态，违反 spec §8.3 硬约束 2。
 */
export const vStaggerList: Directive<HTMLElement> = {
  mounted: reveal,
  updated: reveal,
  beforeUnmount(el) {
    seenByContainer.delete(el)
  },
}
