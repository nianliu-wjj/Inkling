import { inject, provide, type ShallowRef } from 'vue'
import type MindMap from 'simple-mind-map'
import type { Bus } from './bus'
import type { LocalConfig, MapConfig } from './localConfig'
import type { MindMapUiState } from './store'

/**
 * 导图窗口上下文：MindMapApp 在根部 provide，其余组件用 useMindMap() 取。
 * `mindMap` 是 shallowRef——库实例内部结构巨大且自管理，不能被 Vue 深度代理。
 */
export interface MindMapContext {
  mindMap: ShallowRef<MindMap | null>
  bus: Bus
  ui: MindMapUiState
  /** reactive；改动后由 MindMapApp 统一落 localStorage 并同步到库。 */
  localConfig: LocalConfig
  mapConfig: MapConfig
}

const KEY = Symbol('inkling-mindmap-context')

export function provideMindMapContext(ctx: MindMapContext): void {
  provide(KEY, ctx)
}

export function useMindMap(): MindMapContext {
  const ctx = inject<MindMapContext | null>(KEY, null)
  if (!ctx) throw new Error('useMindMap 必须在 MindMapApp 子树内调用')
  return ctx
}

/**
 * 取当前库实例；组件只在 `v-if="mindMap"` 内渲染，因此正常情况下非空。
 * 为空时抛错而不是静默返回，便于在日志里定位挂载时序问题。
 */
export function requireMindMap(ctx: MindMapContext): MindMap {
  const instance = ctx.mindMap.value
  if (!instance) throw new Error('思维导图实例尚未创建')
  return instance
}
