/**
 * 运行平台探测。
 *
 * 用 UserAgent 同步判断，而不是从 Rust 拿：窗口控件要在**首帧**就按平台渲染，
 * 走 IPC 会先渲染一种样式再切换，用户能看到跳变。
 * WebView2（Windows）的 UA 必含 `Windows`，WKWebView（macOS）必含 `Macintosh`，
 * 对「选哪套窗口控件」这个用途足够可靠。
 */

/** 运行平台。除 Windows / macOS 外统一归为 other，按 Windows 布局渲染。 */
export type Platform = 'windows' | 'macos' | 'other'

function detect(): Platform {
  const ua = navigator.userAgent
  if (ua.includes('Macintosh') || ua.includes('Mac OS X')) return 'macos'
  if (ua.includes('Windows')) return 'windows'
  return 'other'
}

export const platform: Platform = detect()

/**
 * 窗口控件是否放在标题栏左侧。
 *
 * macOS 的红黄绿三点在左上角，Windows 的三个方形按钮在右上角——
 * 这是两个系统的既定惯例，位置本身就是「原生样式」的一部分，因此不做成可配置项。
 */
export const controlsOnLeft = platform === 'macos'
