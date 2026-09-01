import { ref, type Ref } from 'vue'
import { DEFAULT_THEME, themes } from '@/constants/themes'
import { logger } from '@/service/logger'

/** 本地缓存键：用于窗口启动瞬间抢先上主题，避免默认深色闪一下再切换。 */
const CACHE_KEY = 'inkling-theme'

/** 合法主题标识集合，用于过滤脏数据。 */
const VALID_KEYS = new Set(themes.map((t) => t.key))

const current = ref<string>(DEFAULT_THEME)

/**
 * 把主题写进 DOM。
 *
 * 约定：`dark` 是 tokens.css 中 :root 的默认值，不设 data-theme 属性；
 * 其余 29 套通过 :root[data-theme="..."] 覆盖（见 styles/themes.css）。
 */
function writeToDom(key: string): void {
  const root = document.documentElement
  if (key === DEFAULT_THEME) root.removeAttribute('data-theme')
  else root.setAttribute('data-theme', key)
}

/**
 * 启动时立即应用本地缓存的主题。
 *
 * 设置的权威来源是 SQLite，但读取需要一次 IPC 往返，期间窗口会先以默认
 * 深色渲染再跳变。因此入口脚本先用 localStorage 的镜像抢先上色，
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
