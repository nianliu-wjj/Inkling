import { cubicBezier, type EasingParam } from 'animejs'
import { logger } from '@/service/logger'

/**
 * 运行时动效参数：来自 CSS 动效令牌（tokens.css / extensions.css），JS 动效与 CSS 过渡共用一套节奏。
 * reduced = 系统开启「减少动态效果」，所有时长归零，预设据此直接落到终态、不创建动画。
 */
export interface MotionTokens {
  /** --dur-fast，毫秒。 */
  fast: number
  /** --dur-base，毫秒。 */
  base: number
  /** --dur-slow，毫秒。 */
  slow: number
  /** --stagger，列表逐项延迟，毫秒。 */
  stagger: number
  /** --ease-out。 */
  easeOut: EasingParam
  /** --ease-spring。 */
  easeSpring: EasingParam
  reduced: boolean
}

/** 令牌读取失败时的兜底值（与原型 :root 逐字一致）。 */
const FALLBACK = {
  fast: 120,
  base: 180,
  slow: 280,
  stagger: 30,
  easeOut: cubicBezier(0.22, 0.61, 0.36, 1),
  easeSpring: cubicBezier(0.34, 1.56, 0.64, 1),
} as const

/** 把 "180ms" / ".2s" 解析为毫秒；无法解析返回 fallback。 */
export function parseDuration(raw: string, fallback: number): number {
  const match = raw.trim().match(/^([\d.]+)(ms|s)$/)
  if (!match) return fallback
  const value = Number(match[1])
  if (!Number.isFinite(value)) return fallback
  return match[2] === 's' ? value * 1000 : value
}

/** 把 CSS `cubic-bezier(a,b,c,d)` 转成 animejs 缓动函数；其他写法返回 fallback。 */
export function parseEase(raw: string, fallback: EasingParam): EasingParam {
  const match = raw.trim().match(/^cubic-bezier\(([^)]+)\)$/)
  if (!match) return fallback
  const parts = match[1].split(',').map((p) => Number(p.trim()))
  if (parts.length !== 4 || parts.some((p) => !Number.isFinite(p))) return fallback
  const [x1, y1, x2, y2] = parts as [number, number, number, number]
  return cubicBezier(x1, y1, x2, y2)
}

/** 读取当前文档的动效令牌。每次调用都重新读，主题切换后无需重置。 */
export function readMotionTokens(): MotionTokens {
  const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  const style = getComputedStyle(document.documentElement)
  const read = (name: string): string => style.getPropertyValue(name)
  const tokens: MotionTokens = {
    fast: parseDuration(read('--dur-fast'), FALLBACK.fast),
    base: parseDuration(read('--dur-base'), FALLBACK.base),
    slow: parseDuration(read('--dur-slow'), FALLBACK.slow),
    stagger: parseDuration(read('--stagger'), FALLBACK.stagger),
    easeOut: parseEase(read('--ease-out'), FALLBACK.easeOut),
    easeSpring: parseEase(read('--ease-spring'), FALLBACK.easeSpring),
    reduced,
  }
  if (reduced) {
    logger.debug('motion', '系统要求减少动态效果，JS 动效整体关闭')
    return { ...tokens, fast: 0, base: 0, slow: 0, stagger: 0 }
  }
  return tokens
}
