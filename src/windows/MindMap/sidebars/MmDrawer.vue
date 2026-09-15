<script setup lang="ts">
import type { Component } from 'vue'
import { computed, shallowRef, watch } from 'vue'
import { logger } from '@/service/logger'
import { sidebarTriggerList } from '../constants/lists'
import type { SidebarName } from '../core/store'
import { useMindMap } from '../core/useMindMap'
import BaseStyleSidebar from './BaseStyleSidebar.vue'
import NodeStyleSidebar from './NodeStyleSidebar.vue'
import OutlineSidebar from './OutlineSidebar.vue'
import SettingSidebar from './SettingSidebar.vue'
import ShortcutSidebar from './ShortcutSidebar.vue'
import StructureSidebar from './StructureSidebar.vue'
import ThemeSidebar from './ThemeSidebar.vue'

/**
 * 右侧抽屉壳（对齐原型单个 `#mmDrawer`，`docs/index.html:383-389`）。
 *
 * 原型是「一个抽屉换 title / body 内容」。收敛前我们是**每个侧栏组件各自包一层壳**
 * （`<SidebarShell name="…">内容</SidebarShell>`），10 个壳实例常驻：DOM 里虽只有
 * `activeSidebar` 命中的那一个 `<aside>` 可见，但 10 套表单（含 480 行的节点样式面板）
 * 的 setup、watcher、computed 全都常驻在跑。这里收敛成单壳：壳只有这一份，内容按
 * `activeSidebar` 动态渲染，同一时刻只有一个侧栏在渲染。
 *
 * 代价与取舍：内容用 `<KeepAlive>` 缓存、关闭时只 `v-show` 隐藏，所以「切换侧栏」「关掉再
 * 打开」都不会丢状态（收敛前 10 个侧栏常驻挂载，本就如此），DOM 里也始终只有这一个抽屉；
 * 缓存的实例在 MmDrawer 卸载时一并释放。
 */
const { ui } = useMindMap()

/**
 * 标题：dock 六项以 `sidebarTriggerList`（触发条同一份数据）为唯一来源，
 * 快捷键不在 dock 列表里，用它的标题常量兜底——少了这层回退，这个入口的抽屉标题会是空串
 * （旧壳是从各侧栏的 props 拿标题的）。
 */
const EXTRA_TITLES: Partial<Record<OpenSidebarName, string>> = {
  shortcutKey: '快捷键',
}

/** 键类型排除空串：`activeSidebar === ''` 表示抽屉关闭，不查表。 */
type OpenSidebarName = Exclude<SidebarName, ''>

/**
 * 内容表。用 `Record<OpenSidebarName, …>` 而不是宽松的字典，是为了让 `SidebarName`
 * 增删取值时 typecheck 直接报错——Task 4 把图标 / 公式迁到模态时，删掉 `SidebarName` 的两个取值
 * 就同时逼着这里删条目（`CONTENT` 缺键即报错），「入口可达」由此变成编译期约束。
 */
const CONTENT: Record<OpenSidebarName, Component> = {
  nodeStyle: NodeStyleSidebar,
  baseStyle: BaseStyleSidebar,
  theme: ThemeSidebar,
  structure: StructureSidebar,
  outline: OutlineSidebar,
  setting: SettingSidebar,
  // 快捷键是**过渡**项：它是我们的扩展（原型无对应物，spec D43），入口在底栏「更多」下拉，
  // 本步先纳进来保证收敛壳之后「更多 → 快捷键」仍打得开。Task 5 决定它的最终归属。
  shortcutKey: ShortcutSidebar,
}

/* 这里的断言不是多余的：`SidebarName` 含空串（抽屉关闭），而 `CONTENT` / `EXTRA_TITLES`
   的键是 `OpenSidebarName`（已排除空串），此处没有真值分支可供 TS 收窄，直接取掉会报
   TS7053（`Property '' does not exist…`）。空串的落空由下面两处的 `?? null` / `?? ''` 兜住。 */
const activeName = computed(() => ui.activeSidebar as OpenSidebarName)
/** 本次请求要展示的侧栏；`activeSidebar` 为空（或取值不认识）时为 null。 */
const activeContent = computed<Component | null>(() => CONTENT[activeName.value] ?? null)

/**
 * 关闭后仍留在 `<KeepAlive>` 里的那一份内容（最后展示过的侧栏）。
 *
 * 抽屉关闭只是 `v-show` 隐藏、不卸载内容，理由有二：
 * 1. 状态不回归——收敛前 10 个侧栏常驻挂载，关掉再打开不会丢状态；若关闭时就卸载，
 *    大纲树的展开态、公式输入框里没提交的 LaTeX 都会被清掉，那是相对现状的退化。
 * 2. 关闭动画里正文不会先空掉（`v-show` 的离场动画期间 DOM 仍在）。
 * 组件实例由 `<KeepAlive>` 缓存，切换 dock 项时同样复用，只有真正卸载 MmDrawer 才会释放。
 */
const lastShown = shallowRef<Component | null>(null)
watch(activeContent, (value) => {
  if (value) lastShown.value = value
})

const current = computed<Component | null>(() => activeContent.value ?? lastShown.value)
/** 抽屉是否可见：`activeSidebar` 指向某个已知侧栏。 */
const visible = computed(() => activeContent.value !== null)
const title = computed(
  () =>
    sidebarTriggerList.find((item) => item.value === activeName.value)?.name ?? EXTRA_TITLES[activeName.value] ?? '',
)

function close(): void {
  ui.activeSidebar = ''
}

// 抽屉开合是本窗口里少见的状态切换，记一条日志便于排查「点不动 / 打不开」类问题。
// 打标题而不是键名：`nodeStyle` 这类键名对用户和排查者都不直观，标题才认得出是哪个面板。
// （`title` 由 `activeName` 派生、computed 惰性求值，此刻读到的就是本次要打开的侧栏标题。）
watch(
  () => ui.activeSidebar,
  (name) => logger.info('mindmap', `右侧抽屉：${name ? `打开「${title.value}」` : '关闭'}`),
)
</script>

<template>
  <!--
    【不要改回 <Transition>】显隐动画走 `:class` + CSS 过渡（规则在 mindmap.css 的「侧栏抽屉」一节），
    不是因为 `<Transition>` 不能用，而是**它和下面的 `<KeepAlive>` 套在一起会炸**：切侧栏时
    KeepAlive 卸载旧实例走的 `deactivate` 落在了外层 Transition 的上下文里，控制台每次切换都报
    `Uncaught (in promise) TypeError: parentComponent.ctx.deactivate is not a function`
    （Vue warn: Unhandled error during execution of component update，指向
    `<BaseTransition persisted> at <Transition name="mm-drawer-slide" persisted>`）。
    `:class` 驱动 CSS 过渡既有同样的动画，又不与保活打架。

    副作用一并交代：显隐不再能靠 `v-show`（它切的是 `display: none`，而 display 不可过渡，
    切了动画就没了），改为常驻 DOM、用 `.closed` 类切 `opacity` / `transform` / `visibility`。

    【不要改回 <aside v-if>】计划简报里字面写的是 `<aside v-if="current">`，照着改会丢状态——这是
    **夹具实测**结论（不是推测）：`<KeepAlive>` 此时落在被 `v-if` 销毁的子树里，抽屉一关
    它连同缓存一起没了，重开时 setup 重跑、状态回退（大纲树展开态、公式框里没提交的 LaTeX）。
    该取舍的唯一证据是 `.tmp/keepalive-check.mjs`（已 gitignore，可 `node .tmp/keepalive-check.mjs`
    复跑）；Task 3 报告第三节给出了本写法与两种对照写法的完整读数。它同样是不用 `<Transition>`
    的前提：保活要求 aside 常驻。

    `@click.stop` 沿用旧壳：抽屉浮在画布上方，内部点击不应冒泡到舞台，
    免得顺手带出画布的选中 / 拖拽逻辑。
  -->
  <aside class="mm-drawer" :class="{ closed: !visible }" @click.stop>
    <div class="mm-drawer-header">
      <span class="mm-drawer-title">{{ title }}</span>
      <button type="button" class="mm-drawer-close" title="关闭面板" @click="close">✕</button>
    </div>
    <div class="mm-drawer-body">
      <!-- 同一时刻只有一个侧栏实例在渲染；切换时旧的被 KeepAlive 收起，DOM 里始终只有这一个抽屉 -->
      <KeepAlive>
        <component :is="current" v-if="current" />
      </KeepAlive>
    </div>
  </aside>
</template>
