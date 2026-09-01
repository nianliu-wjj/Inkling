/**
 * 顶部感应区窗口入口。
 *
 * 需求 2.1「鼠标触顶」：屏幕顶部中央（水平居中 ±120px、高 80px）的透明感应区，
 * 鼠标稳定悬停 3 秒后才呼出面板，悬停期间显示轻量感应动画，避免快速划过误触发。
 */
import { invoke } from '@tauri-apps/api/core'
import { logger } from '@/service/logger'
import '@/styles'

/** 稳定悬停触发阈值（需求指定 3 秒）。 */
const HOVER_DELAY_MS = 3000

document.documentElement.dataset.window = 'hotzone'

const zone = document.createElement('div')
zone.id = 'hotzone'
zone.title = 'Inkling 感应区（悬停 3 秒展开面板）'

const indicator = document.createElement('div')
indicator.className = 'hotzone-indicator'
indicator.setAttribute('aria-hidden', 'true')
indicator.innerHTML = `
  <span class="hotzone-indicator-label">正在感应</span>
  <span class="hotzone-progress"><span class="hotzone-progress-value"></span></span>
  <span class="hotzone-indicator-dots">•••</span>
`
zone.appendChild(indicator)

const progress = indicator.querySelector<HTMLElement>('.hotzone-progress-value')
let animationFrame: number | null = null
let hoverStartedAt = 0

function stopSensing(): void {
  if (animationFrame !== null) {
    cancelAnimationFrame(animationFrame)
    animationFrame = null
  }
  zone.classList.remove('sensing')
  if (progress) progress.style.width = '0%'
}

function updateSensing(timestamp: number): void {
  if (!zone.classList.contains('sensing')) return
  const elapsed = timestamp - hoverStartedAt
  const ratio = Math.min(elapsed / HOVER_DELAY_MS, 1)
  if (progress) progress.style.width = `${ratio * 100}%`
  if (elapsed >= HOVER_DELAY_MS) {
    animationFrame = null
    zone.classList.remove('sensing')
    logger.info('hotzone', '悬停达到 3 秒阈值，呼出面板')
    void invoke('panel_show').catch((error) => logger.error('hotzone', '呼出面板失败', error))
    return
  }
  animationFrame = requestAnimationFrame(updateSensing)
}

zone.addEventListener('mouseenter', () => {
  if (zone.classList.contains('sensing')) return
  hoverStartedAt = performance.now()
  zone.classList.add('sensing')
  animationFrame = requestAnimationFrame(updateSensing)
})

zone.addEventListener('mouseleave', stopSensing)

const root = document.getElementById('app')
if (root) root.appendChild(zone)
else logger.error('hotzone', '未找到 #app 挂载点')
