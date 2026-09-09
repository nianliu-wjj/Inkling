import MindMap, { type MindMapOptions } from 'simple-mind-map'
import brokenImage from '@/assets/mindmap/image-broken.svg'
import { logger } from '@/service/logger'
import type { Bus } from './bus'
import { handleClipboardText } from './clipboardText'
import type { LocalConfig, MapConfig } from './localConfig'
import type { MindMapFullData } from './persistence'
import { registerPlugins, RichTextPlugin, ScrollbarPlugin } from './plugins'

/** 从库转发到 bus 的事件（参考端 Edit.vue 列表 + 本项目补充）。 */
export const FORWARDED_EVENTS = [
  'node_active',
  'data_change',
  'view_data_change',
  'back_forward',
  'node_contextmenu',
  'node_click',
  'node_dblclick',
  'draw_click',
  'expand_btn_click',
  'svg_mousedown',
  'mouseup',
  'mode_change',
  'node_tree_render_end',
  'rich_text_selection_change',
  'transforming-dom-to-images',
  'generalization_node_contextmenu',
  'painter_start',
  'painter_end',
  'scrollbar_change',
  'scale',
  'translate',
  'node_attachmentClick',
  'node_attachmentContextmenu',
  'demonstrate_jump',
  'exit_demonstrate',
  'node_note_dblclick',
  'node_mousedown',
  'node_img_dblclick',
  'node_icon_click',
  'node_img_click',
  'node_img_adjust_btn_mousedown',
  'delete_node_img_from_delete_btn',
  'search_info_change',
  'search_match_node_list_change',
  'set_data',
  'layout_change',
  'theme_change',
  'outer_frame_active',
  'outer_frame_delete',
  'outer_frame_deactivate',
  'associative_line_click',
  'associative_line_deactivate',
  'node_tag_click',
] as const

export interface CreateMindMapParams {
  el: HTMLElement
  data: MindMapFullData
  localConfig: LocalConfig
  mapConfig: MapConfig
  bus: Bus
  /** 二次确认（naive-ui dialog 封装），resolve true 表示用户确认。 */
  confirm: (content: string) => Promise<boolean>
  /** 导出对话框设置的底部文字。 */
  extraTextOnExport: () => string
  onExportError: () => void
}

/**
 * 构造库实例（等价参考端 Edit.vue 的 init()，去掉多语言 / AI / 本地文件相关项）。
 * 全部库事件转发到 bus；RichText / Scrollbar 按本机偏好动态挂载。
 */
export function createMindMap(params: CreateMindMapParams): MindMap {
  registerPlugins()
  const { el, data, localConfig, mapConfig, bus } = params
  const options: MindMapOptions = {
    el,
    data: data.root,
    fit: false,
    nodeTextEditZIndex: 1000,
    nodeNoteTooltipZIndex: 1000,
    customNoteContentShow: {
      show: (content: string, left: number, top: number, node: unknown) =>
        bus.emit('showNoteContent', content, left, top, node),
      hide: () => bus.emit('hideNoteContent'),
    },
    ...mapConfig,
    useLeftKeySelectionRightKeyDrag: localConfig.useLeftKeySelectionRightKeyDrag,
    customInnerElsAppendTo: null,
    customHandleClipboardText: handleClipboardText,
    defaultNodeImage: brokenImage,
    initRootNodePosition: ['center', 'center'],
    handleIsSplitByWrapOnPasteCreateNewNode: () => params.confirm('是否按换行自动分割节点？'),
    errorHandler: (code: string, error: unknown) => {
      logger.error('mindmap', `库错误 ${code}`, error)
      if (code === 'export_error') params.onExportError()
    },
    addContentToFooter: () => {
      const text = params.extraTextOnExport().trim()
      if (!text) return null
      const footer = document.createElement('div')
      footer.className = 'footer'
      footer.innerHTML = text
      return {
        el: footer,
        cssText: `.footer{width:100%;height:30px;display:flex;justify-content:center;align-items:center;font-size:12px;color:#979797;}`,
        height: 30,
      }
    },
    expandBtnNumHandler: (num: number) => (num >= 100 ? '…' : num),
    // 超链接点击：交给 MindMapApp 统一处理（`#uid` 跳转到目标节点，否则用系统默认程序打开外链）。
    customHyperlinkJump: (link: string) => bus.emit('hyperlinkJump', link),
    // 参考端语义：resolve(false) 表示允许删除（用户已确认），resolve(true) 表示阻止。
    beforeDeleteNodeImg: () => params.confirm('是否确认删除该节点图片？').then((ok) => !ok),
  }
  // 主题 / 结构 / 视图仅在持久化数据里确有其值时注入。
  // 关键：绝不能以 `themeConfig: undefined` 显式覆盖库默认值——那会让 initTheme 的
  // deepmerge 对 undefined 执行 Object.keys 而抛「Cannot convert undefined or null to object」，
  // 旧格式（仅 root）笔记因此打不开。省略该键则库沿用默认 {}。
  if (data.layout) options.layout = data.layout
  if (data.theme?.template) options.theme = data.theme.template
  if (data.theme?.config) options.themeConfig = data.theme.config
  if (data.view) options.viewData = data.view
  logger.info('mindmap', `创建实例 layout=${data.layout ?? '(默认)'} theme=${data.theme?.template ?? '(默认)'}`)
  const mindMap = new MindMap(options)
  FORWARDED_EVENTS.forEach((event) => mindMap.on(event, (...args: unknown[]) => bus.emit(event, ...args)))
  if (localConfig.openNodeRichText) mindMap.addPlugin(RichTextPlugin)
  if (localConfig.isShowScrollbar) mindMap.addPlugin(ScrollbarPlugin)
  return mindMap
}

/** 富文本插件动态挂卸（设置侧栏切换）。 */
export function toggleRichText(mindMap: MindMap, enabled: boolean): void {
  logger.info('mindmap', `富文本插件 ${enabled ? '挂载' : '卸载'}`)
  if (enabled) mindMap.addPlugin(RichTextPlugin)
  else mindMap.removePlugin(RichTextPlugin)
}

/** 滚动条插件动态挂卸（设置侧栏切换）。 */
export function toggleScrollbar(mindMap: MindMap, enabled: boolean): void {
  logger.info('mindmap', `滚动条插件 ${enabled ? '挂载' : '卸载'}`)
  if (enabled) mindMap.addPlugin(ScrollbarPlugin)
  else mindMap.removePlugin(ScrollbarPlugin)
}
