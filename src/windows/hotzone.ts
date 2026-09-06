/**
 * 感应区窗口入口。
 *
 * 需求 2.1「鼠标触顶」：屏幕边缘中央的透明感应区，鼠标稳定悬停 3 秒后才呼出面板，
 * 悬停期间显示轻量感应动画，避免快速划过误触发。
 *
 * 感应区在**哪条边**跟随偏好设置「面板唤出位置」（top / bottom / left / right）：
 * 窗口本身由 Rust 侧移到对应边缘，本文件只负责把边的方向写到 `data-edge`，
 * 让指示器按横条 / 竖条两种形态排版（见 components.css 的 hotzone 段）。
 *
 * 该窗口对鼠标**完全穿透**（否则会挡住下层浏览器标签栏、其他应用的标题栏按钮），
 * 因此这里收不到 DOM 的 mouseenter / mouseleave；进入 / 离开由后端
 * `hotzone_watcher` 轮询全局光标坐标后以事件推送，本文件只负责动画与计时。
 */
import { invoke } from '@tauri-apps/api/core'
import { AppEvents, onAppEvent } from '@/service/events'
import { logger } from '@/service/logger'
import type { PanelPosition, Settings } from '@/typings/domain'
import '@/styles'

/** 稳定悬停触发阈值（需求指定 3 秒）。 */
const HOVER_DELAY_MS = 3000

/** 合法的边取值；设置里出现未知值时回落顶部，与 Rust 侧 hotzone_geometry 一致。 */
const EDGES: readonly PanelPosition[] = ['top', 'bottom', 'left', 'right']

document.documentElement.dataset.window = 'hotzone'

const zone = document.createElement('div')
zone.id = 'hotzone'
zone.dataset.edge = 'top'

const indicator = document.createElement('div')
indicator.className = 'hotzone-indicator'
indicator.setAttribute('aria-hidden', 'true')
indicator.innerHTML = `
  <span class="hotzone-indicator-label">正在感应</span>
  <span class="hotzone-progress"><span class="hotzone-progress-value"></span></span>
  <span class="hotzone-indicator-dots">•••</span>
`
zone.appendChild(indicator)

let animationFrame: number | null = null
let hoverStartedAt = 0

/** 进度以 CSS 变量下发：横条按宽度、竖条按高度增长，由样式各取所需。 */
function setProgress(ratio: number): void {
  indicator.style.setProperty('--hz-progress', String(ratio))
}

function stopSensing(): void {
  if (animationFrame !== null) {
    cancelAnimationFrame(animationFrame)
    animationFrame = null
  }
  zone.classList.remove('sensing')
  setProgress(0)
}

function updateSensing(timestamp: number): void {
  if (!zone.classList.contains('sensing')) return
  const elapsed = timestamp - hoverStartedAt
  setProgress(Math.min(elapsed / HOVER_DELAY_MS, 1))
  if (elapsed >= HOVER_DELAY_MS) {
    animationFrame = null
    zone.classList.remove('sensing')
    logger.info('hotzone', '悬停达到 3 秒阈值，呼出面板')
    void invoke('panel_show').catch((error) => logger.error('hotzone', '呼出面板失败', error))
    return
  }
  animationFrame = requestAnimationFrame(updateSensing)
}

function startSensing(): void {
  if (zone.classList.contains('sensing')) return
  hoverStartedAt = performance.now()
  zone.classList.add('sensing')
  animationFrame = requestAnimationFrame(updateSensing)
}

/** 把设置里的唤出位置写到 data-edge，驱动指示器的横/竖排版。 */
function applyEdge(position: string): void {
  const edge = EDGES.includes(position as PanelPosition) ? position : 'top'
  if (zone.dataset.edge === edge) return
  logger.info('hotzone', `感应区方向切换为 ${edge}`)
  zone.dataset.edge = edge
}

// 后端仅在状态翻转时推送：true = 光标进入感应区，false = 离开（或面板已展开）。
void onAppEvent<boolean>(AppEvents.hotzoneHover, (inside) => {
  logger.debug('hotzone', inside ? '光标进入感应区' : '光标离开感应区')
  if (inside) startSensing()
  else stopSensing()
}).catch((error) => logger.error('hotzone', '订阅感应区悬停事件失败', error))

// 启动时读一次唤出位置，之后跟随设置变更（设置页改了方向，窗口已被 Rust 移走，这里只换排版）。
void invoke<Settings>('settings_get')
  .then((settings) => applyEdge(settings.panel_position))
  .catch((error) => logger.error('hotzone', '读取唤出位置失败，按顶部排版', error))
void onAppEvent<Settings>(AppEvents.settingsChanged, (settings) => {
  if (settings) applyEdge(settings.panel_position)
}).catch((error) => logger.error('hotzone', '订阅设置变更失败', error))

const root = document.getElementById('app')
if (root) root.appendChild(zone)
else logger.error('hotzone', '未找到 #app 挂载点')
