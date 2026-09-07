/**
 * 插件注册（等价参考端 simple-mind-map/full.js 与 Edit.vue 的 usePlugin 链）。
 *
 * RichText 与 Scrollbar 不在此常驻注册：它们随本机偏好动态 addPlugin / removePlugin。
 * `registerPlugins` 幂等——窗口热重载时模块会重新执行，重复 usePlugin 会被库忽略但主题包会重复 init。
 */
import MindMap from 'simple-mind-map'
import MiniMap from 'simple-mind-map/src/plugins/MiniMap.js'
import Watermark from 'simple-mind-map/src/plugins/Watermark.js'
import KeyboardNavigation from 'simple-mind-map/src/plugins/KeyboardNavigation.js'
import ExportPDF from 'simple-mind-map/src/plugins/ExportPDF.js'
import ExportXMind from 'simple-mind-map/src/plugins/ExportXMind.js'
import Export from 'simple-mind-map/src/plugins/Export.js'
import Drag from 'simple-mind-map/src/plugins/Drag.js'
import Select from 'simple-mind-map/src/plugins/Select.js'
import AssociativeLine from 'simple-mind-map/src/plugins/AssociativeLine.js'
import NodeImgAdjust from 'simple-mind-map/src/plugins/NodeImgAdjust.js'
import TouchEvent from 'simple-mind-map/src/plugins/TouchEvent.js'
import SearchPlugin from 'simple-mind-map/src/plugins/Search.js'
import Painter from 'simple-mind-map/src/plugins/Painter.js'
import Formula from 'simple-mind-map/src/plugins/Formula.js'
import RainbowLines from 'simple-mind-map/src/plugins/RainbowLines.js'
import Demonstrate from 'simple-mind-map/src/plugins/Demonstrate.js'
import OuterFrame from 'simple-mind-map/src/plugins/OuterFrame.js'
import MindMapLayoutPro from 'simple-mind-map/src/plugins/MindMapLayoutPro.js'
import NodeBase64ImageStorage from 'simple-mind-map/src/plugins/NodeBase64ImageStorage.js'
import RichText from 'simple-mind-map/src/plugins/RichText.js'
import Scrollbar from 'simple-mind-map/src/plugins/Scrollbar.js'
import Themes from 'simple-mind-map-plugin-themes'
import { logger } from '@/service/logger'

export const RichTextPlugin = RichText
export const ScrollbarPlugin = Scrollbar

let registered = false

export function registerPlugins(): void {
  if (registered) return
  registered = true
  MindMap.usePlugin(MiniMap)
    .usePlugin(Watermark)
    .usePlugin(Drag)
    .usePlugin(KeyboardNavigation)
    .usePlugin(ExportPDF)
    .usePlugin(ExportXMind)
    .usePlugin(Export)
    .usePlugin(Select)
    .usePlugin(AssociativeLine)
    .usePlugin(NodeImgAdjust)
    .usePlugin(TouchEvent)
    .usePlugin(SearchPlugin)
    .usePlugin(Painter)
    .usePlugin(Formula)
    .usePlugin(RainbowLines)
    .usePlugin(Demonstrate)
    .usePlugin(OuterFrame)
    .usePlugin(MindMapLayoutPro)
    .usePlugin(NodeBase64ImageStorage)
  Themes.init(MindMap)
  logger.info('mindmap', `已注册 ${MindMap.pluginList.length} 个插件与主题包`)
}
