declare module 'simple-mind-map' {
  interface MindMapOptions {
    el: HTMLElement
    data?: unknown
    fit?: boolean
    enableFreeDrag?: boolean
    mousewheelAction?: string
    [key: string]: unknown
  }

  class MindMap {
    constructor(options: MindMapOptions)
    on(event: string, listener: (...args: any[]) => void): void
    off?(event: string, listener: (...args: any[]) => void): void
    getData(withConfig?: boolean): unknown
    setData(data: unknown): void
    resize(): void
    destroy(): void
  }

  export default MindMap
}
