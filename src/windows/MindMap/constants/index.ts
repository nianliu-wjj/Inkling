/**
 * 常量入口。贴纸组（参考端 config/icon.js，560KB base64）只在首次打开图标弹窗时异步加载，
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

/**
 * 贴纸组按需加载，多次调用共享同一个 Promise。
 *
 * 失败时必须把缓存清掉：已拒绝的 Promise 若留在 `stickersPromise` 里，下次调用拿到的还是同一个
 * 失败的 Promise，「下次再试」就退化成再打一条错误日志（`NodeIconModal.ensureStickers` 承诺的是
 * 「失败后允许下次打开重试」）。清缓存的同时把错误原样抛出，调用方仍能拿到失败。
 */
export function loadStickers(): Promise<StickerGroup[]> {
  if (!stickersPromise) {
    stickersPromise = import('./stickers')
      .then((module) => module.stickerIconGroups as StickerGroup[])
      .catch((error: unknown) => {
        stickersPromise = null // 失败不留缓存，下次调用重新拉
        throw error
      })
  }
  return stickersPromise
}
