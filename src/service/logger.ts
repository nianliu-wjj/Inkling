/**
 * 分级日志。
 *
 * 四个 WebView 各自独立运行，控制台输出混在一起难以区分来源，
 * 因此统一加「时间戳 + 窗口标识 + 模块作用域」前缀，便于按窗口过滤。
 * 全项目禁止裸 console.*，一律走此模块（见 CLAUDE.md 编码习惯约定）。
 */

type Level = 'debug' | 'info' | 'warn' | 'error'

/** 窗口标识：优先取入口 HTML 文件名（panel.html → panel），兜底用 pathname。 */
const WINDOW_TAG: string = (() => {
  const file = location.pathname.split('/').pop() ?? ''
  return file.replace(/\.html$/, '') || 'main'
})()

/** 控制台方法映射：debug 走 log，避免默认被浏览器折叠隐藏。 */
const CONSOLE_METHOD: Record<Level, 'log' | 'info' | 'warn' | 'error'> = {
  debug: 'log',
  info: 'info',
  warn: 'warn',
  error: 'error',
}

function emit(level: Level, scope: string, message: string, args: unknown[]): void {
  const line = `[${new Date().toISOString()}][${WINDOW_TAG}][${scope}] ${message}`
  // eslint-disable-next-line no-console
  console[CONSOLE_METHOD[level]](line, ...args)
}

export const logger = {
  debug: (scope: string, message: string, ...args: unknown[]): void => emit('debug', scope, message, args),
  info: (scope: string, message: string, ...args: unknown[]): void => emit('info', scope, message, args),
  warn: (scope: string, message: string, ...args: unknown[]): void => emit('warn', scope, message, args),
  error: (scope: string, message: string, ...args: unknown[]): void => emit('error', scope, message, args),
}
