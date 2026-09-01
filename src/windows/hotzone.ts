/**
 * 顶部感应区窗口入口。
 *
 * 需求 2.1「鼠标触顶」：屏幕顶部中央（水平居中 ±120px、高 80px）的隐形感应区，
 * 悬停超过 100ms 自动滑出面板。
 *
 * 该窗口是一个常驻的透明无边框窗口（相比鼠标轮询零 CPU 开销），
 * 区域本身不可见——原型 #hotzone 的 outline 透明度为 0，仅调试时改成 .2 查看。
 * 因此这里不渲染任何可见指示器，避免在桌面顶部留下一条可见色块。
 */
import { invoke } from '@tauri-apps/api/core'
import { logger } from '@/service/logger'
import '@/styles'

/** 悬停触发阈值（需求指定 100ms）。 */
const HOVER_DELAY_MS = 100

document.documentElement.dataset.window = 'hotzone'

const zone = document.createElement('div')
zone.id = 'hotzone'
zone.title = 'Inkling 感应区（悬停 100ms 展开面板）'

/** 悬停计时器：离开则取消，避免快速划过误触发。 */
let hoverTimer: ReturnType<typeof setTimeout> | null = null

function cancelHover(): void {
  if (hoverTimer !== null) {
    clearTimeout(hoverTimer)
    hoverTimer = null
  }
}

zone.addEventListener('mouseenter', () => {
  cancelHover()
  hoverTimer = setTimeout(() => {
    hoverTimer = null
    logger.info('hotzone', '悬停达到阈值，呼出面板')
    void invoke('panel_show').catch((error) => {
      logger.error('hotzone', '呼出面板失败', error)
    })
  }, HOVER_DELAY_MS)
})

zone.addEventListener('mouseleave', cancelHover)

const root = document.getElementById('app')
if (root) root.appendChild(zone)
else logger.error('hotzone', '未找到 #app 挂载点')
