import type { Component } from 'vue'
import ClipPage from '@/windows/Panel/ClipPage.vue'
import NotePage from '@/windows/Panel/NotePage.vue'
import TodoPage from '@/windows/Panel/TodoPage.vue'

/**
 * 呼出面板的插件注册表。
 *
 * 面板不再硬编码「笔记 / 粘贴板 / 待办」三态：能力以插件形式登记在本文件，
 * `PanelApp` 只依赖注册表与用户的启用设置，新增一种捕获能力**不必改 PanelApp 本体**。
 *
 * 为什么是编译期注册表而不是运行时加载外部 JS：
 * `tauri.conf.json` 的 CSP 是 `script-src 'self'`，运行时加载外部脚本必须放宽它，
 * 而本应用能读写本地 SQLite 与文件系统——那等于把脚本沙箱打开，代价远超收益。
 * 若将来确实需要第三方插件，正确做法是让插件跑在独立 WebView 里、通过 IPC 通信。
 */

/**
 * 插件页可选暴露给 PanelApp 的能力（通过 `defineExpose`）。全部可选，PanelApp 只调用存在的方法。
 *
 * - `focus`：呼出 / 进入 Zen 后把光标放进页面主输入区；
 * - `dismissOverlays`：关闭本页的删除确认 / 弹窗，关了返回 true（Esc 链与切页副作用用）；
 * - `loadNote`：把某条笔记回显进编辑态（只有笔记页实现）；
 * - `onPanelHide`：面板即将隐藏（丢弃回显编辑、恢复草稿），PanelApp 会等待它完成再隐藏窗口；
 * - `isEditing`：是否处于回显编辑态（Zen 入口显隐）。页面 expose 的是 `ComputedRef<boolean>`，
 *   经模板 ref 拿到的实例代理会自动解包，这里按解包后的 boolean 声明。
 */
export interface PanelPageExpose {
  focus?(): void
  dismissOverlays?(): boolean
  loadNote?(id: string): Promise<void>
  onPanelHide?(): void | Promise<void>
  isEditing?: boolean
}

/** 一个面板插件。 */
export interface PanelPlugin {
  /** 唯一标识，同时作为 Settings.panel_plugins 里的键。 */
  id: string
  /** 圆点导航的展示名。 */
  label: string
  /**
   * 圆点导航的颜色类名（原型 `.nav-dot.dot-note / .dot-clip / .dot-todo`）。
   * 生成层用 `--dot-c` 令牌绘制 ::before 矢量圆；新插件若没有对应令牌，
   * 会落到 extensions.css §3 `.nav-dots { --dot-c: var(--accent-rgb) }` 的兜底色。
   */
  dotClass: string
  /** 页面组件。约定只使用既有 CSS 令牌，类名以插件 id 为前缀。 */
  component: Component
}

/**
 * 内置插件，数组顺序即默认展示顺序与快捷键序号（⌃1、⌃2…）。
 *
 * 快捷键刻意**不由插件自选**：否则两个插件都想要 ⌃1 时无解；
 * 由顺序决定则天然唯一。
 */
export const builtinPlugins: readonly PanelPlugin[] = [
  { id: 'note', label: '笔记', dotClass: 'dot-note', component: NotePage },
  { id: 'clipboard', label: '粘贴板', dotClass: 'dot-clip', component: ClipPage },
  { id: 'todo', label: '待办', dotClass: 'dot-todo', component: TodoPage },
] as const

/** 快捷键上限：⌃1..⌃9，超出的插件只能用圆点点击切换。 */
export const MAX_HOTKEY_SLOTS = 9

/**
 * 按用户设置解析出启用的插件列表。
 *
 * `setting` 是逗号分隔的插件 id 有序列表：**在列表里 = 启用，列表次序 = 展示顺序**。
 * 用一个字段同时表达启用与排序，比「启用集合 + 顺序数组」两个字段更难产生不一致状态。
 *
 * 兜底：设置为空、或过滤后一个插件都不剩时，回落到全部内置插件的默认顺序——
 * 面板不能出现「一个页面都没有」的状态。
 */
export function resolvePlugins(setting: string): PanelPlugin[] {
  const ids = setting
    .split(',')
    .map((id) => id.trim())
    .filter(Boolean)

  const resolved = ids
    .map((id) => builtinPlugins.find((plugin) => plugin.id === id))
    .filter((plugin): plugin is PanelPlugin => Boolean(plugin))

  return resolved.length > 0 ? resolved : [...builtinPlugins]
}

/** 把插件列表序列化回设置值。 */
export function serializePlugins(plugins: readonly PanelPlugin[]): string {
  return plugins.map((plugin) => plugin.id).join(',')
}
