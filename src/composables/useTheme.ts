import { ref, type Ref } from 'vue'
import { DEFAULT_THEME, themes } from '@/constants/themes'
import { logger } from '@/service/logger'

/** 本地缓存键：用于窗口启动瞬间抢先上主题，避免入口 html 上的默认打字机闪一下再切换。 */
const CACHE_KEY = 'inkling-theme'

/** 合法主题标识集合，用于过滤脏数据。 */
const VALID_KEYS = new Set(themes.map((t) => t.key))

const current = ref<string>(DEFAULT_THEME)

/**
 * 把主题写进 DOM。
 *
 * 与原型 `document.documentElement.dataset.theme = t.id` 一致：任何主题（含 dark）都写
 * data-theme 属性。dark 没有对应的 [data-theme] 块，直接落到 tokens.css 的 :root 基础令牌；
 * 其余 31 套由 themes.css / extensions.css 的 :root[data-theme="..."] 覆盖。
 */
function writeToDom(key: string): void {
  document.documentElement.setAttribute('data-theme', key)
}

/**
 * 启动时立即应用本地缓存的主题。
 *
 * 设置的权威来源是 SQLite，但读取需要一次 IPC 往返，期间窗口会先以入口 html 上的
 * 默认打字机渲染再跳变。因此入口脚本先用 localStorage 的镜像抢先上色，
 * 待 settings_get 返回后再以后端值为准校正。
 */
export function applyCachedTheme(): void {
  try {
    const cached = localStorage.getItem(CACHE_KEY)
    if (cached && VALID_KEYS.has(cached)) {
      current.value = cached
      writeToDom(cached)
    }
  } catch (error) {
    // localStorage 在某些隔离环境下不可用，静默降级为默认主题即可。
    logger.warn('theme', '读取主题缓存失败，使用默认主题', error)
  }
}

export function useTheme(): {
  theme: Ref<string>
  applyTheme: (key: string) => void
} {
  /**
   * 应用主题并更新本地镜像。
   *
   * 只负责视觉与缓存，**不做持久化**——持久化统一走 settings_save，
   * 由调用方（偏好设置页）在同一次保存中带上 theme 字段。
   */
  function applyTheme(key: string): void {
    const target = VALID_KEYS.has(key) ? key : DEFAULT_THEME
    if (target !== key) logger.warn('theme', `未知主题 ${key}，回退到 ${target}`)

    logger.info('theme', `应用主题 ${target}`)
    current.value = target
    writeToDom(target)

    try {
      localStorage.setItem(CACHE_KEY, target)
    } catch (error) {
      logger.warn('theme', '写入主题缓存失败', error)
    }
  }

  return { theme: current, applyTheme }
}
