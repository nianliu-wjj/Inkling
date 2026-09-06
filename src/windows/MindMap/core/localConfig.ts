import { logger } from '@/service/logger'

/**
 * 导图窗口的本机持久化配置（spec D5），两个 localStorage 键：
 * - localConfig：编辑器行为偏好（参考端 vuex.localConfig）；
 * - mapConfig：「设置」侧栏里的库实例配置（参考端 storeConfig），建实例时展开进 options。
 * 读写都包 try/catch：隐私模式或存储被清空时按默认值工作。
 */
const LOCAL_KEY = 'inkling.mindmap.localConfig'
const MAP_KEY = 'inkling.mindmap.config'

export interface LocalConfig {
  isZenMode: boolean
  openNodeRichText: boolean
  useLeftKeySelectionRightKeyDrag: boolean
  isShowScrollbar: boolean
  enableDragImport: boolean
}

export const DEFAULT_LOCAL_CONFIG: LocalConfig = {
  isZenMode: false,
  openNodeRichText: true,
  useLeftKeySelectionRightKeyDrag: false,
  isShowScrollbar: false,
  enableDragImport: true,
}

/** 库实例配置：键名与 simple-mind-map options 一致，值类型放宽为 unknown 由库校验。 */
export type MapConfig = Record<string, unknown>

export const DEFAULT_MAP_CONFIG: MapConfig = {
  enableFreeDrag: false,
  mousewheelAction: 'zoom',
  mousewheelZoomActionReverse: true,
  createNewNodeBehavior: 'default',
  openRealtimeRenderOnNodeTextEdit: true,
  enableAutoEnterTextEditWhenKeydown: true,
  isUseHandDrawnLikeStyle: false,
  isUseMomentum: true,
  alwaysShowExpandBtn: false,
  enableInheritAncestorLineStyle: false,
  imgTextMargin: 5,
  textContentMargin: 2,
  openPerformance: false,
  demonstrateConfig: { openBlankMode: false },
}

function read<T extends object>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    if (!raw) return { ...fallback }
    const parsed = JSON.parse(raw) as Partial<T>
    return { ...fallback, ...parsed }
  } catch (error) {
    logger.warn('mindmap', `读取 ${key} 失败，使用默认配置`, error)
    return { ...fallback }
  }
}

function write(key: string, value: object): void {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch (error) {
    logger.warn('mindmap', `写入 ${key} 失败`, error)
  }
}

export const loadLocalConfig = (): LocalConfig => read(LOCAL_KEY, DEFAULT_LOCAL_CONFIG)
export const saveLocalConfig = (config: LocalConfig): void => write(LOCAL_KEY, config)
export const loadMapConfig = (): MapConfig => read(MAP_KEY, DEFAULT_MAP_CONFIG)
export const saveMapConfig = (config: MapConfig): void => write(MAP_KEY, config)
