/**
 * simple-mind-map 0.14.0-fix.3 的最小类型声明。
 *
 * 库本身是 JS，此处只声明本项目用到的 API；插件实例属性（renderer / view / keyCommand /
 * associativeLine / painter / watermark / miniMap / search / formula / demonstrate / scrollbar）
 * 按 any 放行，调用点以参考端 web 代码（D:\参考项目\mind-map\web）为准。
 */
declare module 'simple-mind-map' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  type Any = any

  export interface MindMapNode {
    uid: string
    isRoot: boolean
    isGeneralization: boolean
    layerIndex: number
    nodeData: { data: Record<string, Any>; children: Any[] }
    parent: MindMapNode | null
    children: MindMapNode[]
    width: number
    height: number
    left: number
    top: number
    getData(key?: string): Any
    getStyle(prop: string, root?: boolean): Any
    getSelfStyle(prop: string): Any
    setStyle(prop: string, value: Any): void
    setStyles(styles: Record<string, Any>): void
    setText(text: string, richText?: boolean, resetRichText?: boolean): void
    setImage(img: { url: string; title: string; width: number; height: number }): void
    setIcon(icons: string[]): void
    setHyperlink(link: string, title: string): void
    setNote(note: string): void
    setTag(tags: Array<string | { text: string; style?: Record<string, Any> }>): void
    setAttachment(url: string, name: string): void
    setShape(shape: string): void
    active(): void
    deactivate(): void
    removeExpandBtn(): void
    getRect(): { left: number; top: number; width: number; height: number }
    getRectInSvg(): { left: number; top: number; width: number; height: number }
    hasCustomStyle(): boolean
    [key: string]: Any
  }

  export interface MindMapOptions {
    el: HTMLElement
    data?: Any
    layout?: string
    theme?: string
    themeConfig?: Record<string, Any>
    viewData?: Any
    fit?: boolean
    readonly?: boolean
    [key: string]: Any
  }

  export default class MindMap {
    constructor(options: MindMapOptions)
    static usePlugin(plugin: Any, options?: Any): typeof MindMap
    static hasPlugin(plugin: Any): number
    static pluginList: Any[]
    static iconList: Any[]

    opt: Record<string, Any>
    el: HTMLElement
    width: number
    height: number
    elRect: DOMRect
    renderer: Any
    view: Any
    keyCommand: Any
    command: Any
    event: Any
    svg: Any
    draw: Any
    themeConfig: Record<string, Any>
    // 插件实例（注册后存在）
    doExport?: Any
    miniMap?: Any
    watermark?: Any
    search?: Any
    painter?: Any
    formula?: Any
    associativeLine?: Any
    demonstrate?: Any
    scrollbar?: Any
    select?: Any
    richText?: Any
    outerFrame?: Any
    rainbowLines?: Any

    on(event: string, listener: (...args: Any[]) => void): void
    off(event: string, listener: (...args: Any[]) => void): void
    emit(event: string, ...args: Any[]): void
    getData(withConfig?: boolean): Any
    setData(data: Any): void
    setFullData(data: Any): void
    updateData(data: Any): void
    render(callback?: () => void, source?: string): void
    reRender(callback?: () => void, source?: string): void
    resize(): void
    destroy(): void
    setTheme(theme: string, notRender?: boolean): void
    getTheme(): string
    setThemeConfig(config: Record<string, Any>, notRender?: boolean): void
    getThemeConfig(prop?: string): Any
    getCustomThemeConfig(): Record<string, Any>
    getLayout(): string
    setLayout(layout: string, notRender?: boolean): void
    getConfig(prop?: string): Any
    updateConfig(config: Record<string, Any>): void
    execCommand(name: string, ...args: Any[]): void
    setMode(mode: 'readonly' | 'edit'): void
    export(type: string, isDownload?: boolean, name?: string, ...args: Any[]): Promise<Any>
    addPlugin(plugin: Any, options?: Any): void
    removePlugin(plugin: Any): void
    toPos(x: number, y: number): { x: number; y: number }
    getElRectInfo(): void
    addCss(key: string, css: string): void
    removeCss(key: string): void
  }
}

declare module 'simple-mind-map/src/plugins/*' {
  const plugin: unknown
  export default plugin
}

declare module 'simple-mind-map/src/parse/xmind.js' {
  const xmind: {
    parseXmindFile(file: Blob, handleMultiCanvas?: (content: unknown[]) => Promise<unknown>): Promise<unknown>
    transformXmind(content: string, files: unknown[]): Promise<unknown>
  }
  export default xmind
}

declare module 'simple-mind-map/src/parse/markdown.js' {
  const markdown: { transformMarkdownTo(md: string): unknown }
  export default markdown
}

declare module 'simple-mind-map/src/parse/toMarkdown.js' {
  export function transformToMarkdown(root: unknown): string
}

declare module 'simple-mind-map/src/parse/toTxt.js' {
  export function transformToTxt(root: unknown): string
}

declare module 'simple-mind-map/src/svg/icons.js' {
  export const nodeIconList: Array<{ name: string; type: string; list: Array<{ name: string; icon: string }> }>
}

declare module 'simple-mind-map/src/utils/index.js' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  type Any = any
  export function mergerIconList(list: Any[]): Any[]
  export function imgToDataUrl(src: string): Promise<string>
  export function isMobile(): boolean
  export function readBlob(blob: Blob): Promise<string>
  export function getTextFromHtml(html: string): string
  export function nodeRichTextToTextWithWrap(html: string): string
  export function textToNodeRichTextWithWrap(text: string): string
  export function simpleDeepClone<T>(v: T): T
  export function getVisibleColorFromTheme(config: Any): string
  export function createUid(): string
  export function throttle<T extends (...args: Any[]) => void>(fn: T, time?: number, ctx?: Any): T
}

declare module 'simple-mind-map/src/constants/constant.js' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const CONSTANTS: any
  export const layoutList: Array<{ name: string; value: string }>
  export const layoutValueList: string[]
}

declare module 'simple-mind-map-plugin-themes' {
  const Themes: { init(mindMap: unknown): void }
  export default Themes
}

declare module 'simple-mind-map-plugin-themes/themeList' {
  const themeList: Array<{ name: string; value: string; dark: boolean }>
  export default themeList
}

declare module 'simple-mind-map-plugin-themes/themeImgMap' {
  const themeImgMap: Record<string, string>
  export default themeImgMap
}

declare module 'simple-mind-map/dist/simpleMindMap.esm.css'
