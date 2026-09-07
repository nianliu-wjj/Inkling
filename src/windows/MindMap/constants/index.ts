/**
 * 常量入口。贴纸组（参考端 config/icon.js，560KB base64）只在打开图标侧栏时异步加载，
 * 不进入窗口首屏 chunk。
 */
export * from './formulas'
export * from './layouts'
export * from './lists'
export * from './rainbow'
export * from './shortcuts'

export interface StickerGroup {
  name: string
  type: string
  list: Array<{ name: string; icon: string }>
}

let stickersPromise: Promise<StickerGroup[]> | null = null

/** 贴纸组按需加载，多次调用共享同一个 Promise。 */
export function loadStickers(): Promise<StickerGroup[]> {
  if (!stickersPromise) {
    stickersPromise = import('./stickers').then((module) => module.stickerIconGroups as StickerGroup[])
  }
  return stickersPromise
}
