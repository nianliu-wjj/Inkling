import { ref, type Ref } from 'vue'
import { DEFAULT_GLASS_LEVEL, glassLevels } from '@/constants/glass'
import { logger } from '@/service/logger'

/**
 * 玻璃质感档位的应用与缓存。
 *
 * 与 `useTheme` 同构：质感是与配色正交的另一个维度，
 * 走同一套「localStorage 抢先上色 + SQLite 为权威」的模式。
 */

/** 本地缓存键：窗口启动瞬间抢先应用，避免默认标准档闪一下再切换。 */
const CACHE_KEY = 'inkling-glass'

/** 合法档位集合，用于过滤脏数据。 */
const VALID_KEYS = new Set(glassLevels.map((item) => item.key))

const current = ref<string>(DEFAULT_GLASS_LEVEL)

/**
 * 把档位写进 DOM。
 *
 * 约定：`standard` 是 tokens.css 中 :root 的默认值，不设 data-glass 属性；
 * 其余两档通过 :root[data-glass="..."] 覆盖（见 styles/glass.css）。
 */
function writeToDom(key: string): void {
  const root = document.documentElement
  if (key === DEFAULT_GLASS_LEVEL) root.removeAttribute('data-glass')
  else root.setAttribute('data-glass', key)
}

/**
 * 启动时立即应用本地缓存的档位。
 *
 * 与主题同理：设置的权威来源是 SQLite，但读取需要一次 IPC 往返，
 * 期间窗口会先以标准档渲染再跳变，因此先用 localStorage 镜像抢先应用。
 */
export function applyCachedGlass(): void {
  try {
    const cached = localStorage.getItem(CACHE_KEY)
    if (cached && VALID_KEYS.has(cached)) {
      current.value = cached
      writeToDom(cached)
    }
  } catch (error) {
    // localStorage 在某些隔离环境下不可用，静默降级为标准档即可。
    logger.warn('glass', '读取玻璃质感缓存失败，使用标准档', error)
  }
}

export function useGlass(): {
  level: Ref<string>
  applyGlass: (key: string) => void
} {
  /**
   * 应用档位并更新本地镜像。
   *
   * 只负责视觉与缓存，**不做持久化**——持久化统一走 settings_save，
   * 由调用方（偏好设置页）在同一次保存中带上 glass_level 字段。
   */
  function applyGlass(key: string): void {
    const target = VALID_KEYS.has(key) ? key : DEFAULT_GLASS_LEVEL
    if (target !== key) logger.warn('glass', `未知玻璃档位 ${key}，回退到 ${target}`)

    logger.info('glass', `应用玻璃质感 ${target}`)
    current.value = target
    writeToDom(target)

    try {
      localStorage.setItem(CACHE_KEY, target)
    } catch (error) {
      logger.warn('glass', '写入玻璃质感缓存失败', error)
    }
  }

  return { level: current, applyGlass }
}
