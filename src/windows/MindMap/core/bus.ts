/**
 * 导图窗口的事件总线。
 *
 * 参考端用 Vue 2 的 `$bus`（全局 EventEmitter）把库事件转发给几十个组件，
 * 这里用 40 行自实现替代，不引入 mitt。事件名不做枚举约束：
 * 一半是库原生事件名（node_active / data_change …），另一半是窗口内自定义事件（showExport …）。
 */
export type BusHandler = (...args: unknown[]) => void

export interface Bus {
  /** 订阅，返回取消订阅函数。 */
  on(event: string, handler: BusHandler): () => void
  off(event: string, handler: BusHandler): void
  once(event: string, handler: BusHandler): void
  emit(event: string, ...args: unknown[]): void
}

export function createBus(): Bus {
  const handlers = new Map<string, Set<BusHandler>>()

  const off = (event: string, handler: BusHandler): void => {
    handlers.get(event)?.delete(handler)
  }

  const on = (event: string, handler: BusHandler): (() => void) => {
    if (!handlers.has(event)) handlers.set(event, new Set())
    handlers.get(event)!.add(handler)
    return () => off(event, handler)
  }

  const once = (event: string, handler: BusHandler): void => {
    const wrapped: BusHandler = (...args) => {
      off(event, wrapped)
      handler(...args)
    }
    on(event, wrapped)
  }

  const emit = (event: string, ...args: unknown[]): void => {
    // 拷贝一份再遍历：handler 内部可能 off 自己。
    Array.from(handlers.get(event) ?? []).forEach((handler) => handler(...args))
  }

  return { on, off, once, emit }
}
