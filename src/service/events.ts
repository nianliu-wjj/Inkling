/**
 * 跨窗口事件契约。
 *
 * 事件名必须与 `src-tauri/src/events.rs` 中的常量逐字对齐；
 * 后端在数据变更后广播，各窗口据此刷新，避免轮询。
 */
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentScope, onScopeDispose } from 'vue'
import { logger } from './logger'

export const AppEvents = {
  /** 托盘/快捷键请求主窗口切换视图，payload 为视图名。 */
  navigate: 'inkling://navigate',
  panelShown: 'inkling://panel-shown',
  panelHidden: 'inkling://panel-hidden',
  notesChanged: 'inkling://notes-changed',
  clipboardChanged: 'inkling://clipboard-changed',
  todosChanged: 'inkling://todos-changed',
  settingsChanged: 'inkling://settings-changed',
  statsChanged: 'inkling://stats-changed',
  pinUpdated: 'inkling://pin-updated',
  reminderFired: 'inkling://reminder-fired',
  /** 光标进入 / 离开顶部感应区，payload 为 boolean；由后端轮询探测，仅发给 hotzone 窗口。 */
  hotzoneHover: 'inkling://hotzone-hover',
} as const

export type AppEvent = (typeof AppEvents)[keyof typeof AppEvents]

/**
 * 订阅一个跨窗口事件。
 *
 * 在组件 setup 作用域内调用时会自动在卸载时取消订阅，
 * 无需调用方手动持有 UnlistenFn。
 */
export function onAppEvent<T = unknown>(name: AppEvent, handler: (payload: T) => void): Promise<UnlistenFn> {
  logger.debug('events', `订阅事件 ${name}`)
  const pending = listen<T>(name, (event) => {
    logger.debug('events', `收到事件 ${name}`, event.payload)
    handler(event.payload)
  })

  // 组件作用域内自动清理，防止窗口内视图切换时重复订阅。
  // 非组件环境（如窗口入口脚本）下 getCurrentScope() 为空，跳过以免 Vue 告警，
  // 此时订阅随窗口销毁而失效，无需手动清理。
  if (getCurrentScope()) {
    onScopeDispose(() => {
      void pending.then((unlisten) => {
        logger.debug('events', `取消订阅 ${name}`)
        unlisten()
      })
    })
  }

  return pending
}
