import type { Component, Ref } from 'vue'
import { useTodayTodos } from './today-todos'
import TodoDetail from './TodoDetail.vue'

/**
 * 灵动岛插件注册表（spec D8）。
 *
 * 与 `@/panel-plugins` 同一范式：编译期注册、`Settings.island_plugins` 决定启用与顺序。
 * 灵动岛根组件只依赖本注册表；新增一种展示内容不必改根组件。
 * 不做运行时加载外部脚本的理由见 `panel-plugins/index.ts` 头注释（CSP `script-src 'self'`）。
 */

/** 灵动岛上轮播的一条内容。 */
export interface IslandItem {
  id: string
  /** 胶囊单行主文案。 */
  title: string
  /** 胶囊右侧短文案（时间 / 数量等）。 */
  meta?: string
  /** 左侧色点（CSS 颜色）。 */
  dot?: string
  /** 悬停详情组件，接收 `item` prop；不提供则悬停只放大显示 title。 */
  detail?: Component
  /** 原始数据，交给 detail 组件使用。 */
  payload?: unknown
}

export interface IslandPlugin {
  /** 唯一标识，同时作为 Settings.island_plugins 里的键。 */
  id: string
  label: string
  /** 返回响应式条目列表；在灵动岛根组件 setup 内调用一次。 */
  useItems: () => Ref<IslandItem[]>
  /** 点击默认跳转的面板插件 id。 */
  panelPage?: string
  /** 无条目时的空态文案。 */
  emptyText?: string
}

/** 内置插件，数组顺序即默认展示顺序。 */
export const builtinIslandPlugins: readonly IslandPlugin[] = [
  {
    id: 'today-todos',
    label: '当日待办',
    useItems: useTodayTodos,
    panelPage: 'todo',
    emptyText: '今天没有待办 · 点此新建',
  },
]

/** 供插件复用的详情组件（当日待办用）。 */
export { TodoDetail }

/**
 * 按用户设置解析启用的插件列表：在列表里 = 启用，列表次序 = 轮播顺序。
 * 设置为空或全部无效时回落到全部内置插件——灵动岛不能出现「什么都不显示」的状态。
 */
export function resolveIslandPlugins(setting: string): IslandPlugin[] {
  const ids = setting
    .split(',')
    .map((id) => id.trim())
    .filter(Boolean)
  const resolved = ids
    .map((id) => builtinIslandPlugins.find((plugin) => plugin.id === id))
    .filter((plugin): plugin is IslandPlugin => Boolean(plugin))
  return resolved.length > 0 ? resolved : [...builtinIslandPlugins]
}

/** 把插件列表序列化回设置值。 */
export function serializeIslandPlugins(plugins: readonly IslandPlugin[]): string {
  return plugins.map((plugin) => plugin.id).join(',')
}
