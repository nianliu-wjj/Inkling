<script setup lang="ts">
import { NDropdown } from 'naive-ui'
import { ref } from 'vue'
import { logger } from '@/service/logger'
// 自适应 / 展开全部 / 收起全部：与右键菜单同一条命令（收尾批补齐原型 12 项）
import { expandAll, fitCanvas, unexpandAll } from '../core/canvasCommands'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import Count from './Count.vue'
import MouseAction from './MouseAction.vue'
import Fullscreen from './Fullscreen.vue'
import Scale from './Scale.vue'
import Demonstrate from './Demonstrate.vue'

/**
 * 底部控制栏（阶段五 Task 5：由 NavigatorToolbar.vue + Count.vue 合并而来）。
 *
 * 形态照原型单条 `.mm-bottombar`（docs/index.html:414-433）：左 `.mm-stat-left` 放字数/节点数，
 * 右 `.mm-ctrl-right` 放一排控制按钮，一律用生成层的 `.mm-ctrl-btn` / `.mm-ctrl-sep` / `.mm-zoom-val`。
 * 与原型的两点差异（spec 差异表 #12）：**不放语言下拉**（原型那是死控件）；
 * **保留我们的「鼠标行为」「演示」**（真能力，原型没有），按规格插在「全屏」之后。
 * 原型底栏的 12 个控件**全部到位**（含最初的收尾批补上的「自适应 / 展开全部 / 收起全部」）。
 *
 * 行为来源：前几枚沿用原 NavigatorToolbar 的 `@click`，四个子组件（MouseAction / Fullscreen /
 * Scale / Demonstrate）原样搬入，只把类名从自有层的 `.mm-nav-btn` 换成生成层的 `.mm-ctrl-btn`；
 * 收尾三枚走 `core/canvasCommands`，与右键菜单的同名项**同一条命令**（不另写一份实现）。
 *
 * 挂载点与原型一致：在 `.mm-stage` **之外**、`.mindmap-window` 之内，是窗口的静态 flex 项。
 * 原实现把导航条与字数统计绝对定位在画布右下/左下（压在画布上），原型底栏是占位的独立一条。
 */
const ctx = useMindMap()
const { bus, ui } = ctx

/** 小地图开关（Navigator 组件订阅 toggleMiniMap，见 Task 25）。 */
const openMiniMap = ref(false)

function backToRoot(): void {
  requireMindMap(ctx).renderer.setRootNodeCenter()
}

function showSearch(): void {
  bus.emit('showSearch')
}

function toggleMiniMap(): void {
  openMiniMap.value = !openMiniMap.value
  bus.emit('toggleMiniMap', openMiniMap.value)
}

/** 只读 / 编辑切换（ui.isReadonly 由 MindMapApp 订阅 mode_change 维护）。 */
function toggleReadonly(): void {
  const mindMap = requireMindMap(ctx)
  mindMap.setMode(ui.isReadonly ? 'edit' : 'readonly')
  logger.info('mindmap', `切换为${ui.isReadonly ? '编辑' : '只读'}模式`)
}

const moreOptions = [
  { label: '快捷键', key: 'shortcutKey' },
  { label: '源码编辑', key: 'sourceCode' },
]
function onMoreSelect(key: string): void {
  if (key === 'shortcutKey') ui.activeSidebar = 'shortcutKey'
  else if (key === 'sourceCode') ui.isSourceCodeEdit = true
}

// —— 原型底栏后三枚（自适应 / 展开全部 / 收起全部）——
// 命令走 core/canvasCommands，与右键菜单的同名项完全同源；底栏没有「当前节点」，故 uid 一律为空 = 全图。
function onFitCanvas(): void {
  fitCanvas(ctx)
}
function onExpandAll(): void {
  expandAll(ctx)
}
function onUnexpandAll(): void {
  unexpandAll(ctx)
}
</script>

<template>
  <div class="mm-bottombar">
    <div class="mm-stat-left"><Count /></div>
    <div class="mm-ctrl-right">
      <!-- 按原型顺序（docs/index.html:416-432）：定位中心 / 搜索 / 只读 / 小地图 / 全屏
           ｜ 缩小 / 百分比 / 放大 / 自适应 / 展开全部 / 收起全部。
           语言下拉是原型死控件不放；「鼠标行为」「演示」插在全屏之后；「更多」是我们独有的入口，
           与原实现一样留在最末（在分隔线之后）。 -->
      <button type="button" class="mm-ctrl-btn iconfont icondingwei" title="回到根节点" @click="backToRoot" />
      <button type="button" class="mm-ctrl-btn iconfont iconsousuo" title="搜索" @click="showSearch" />
      <button
        type="button"
        class="mm-ctrl-btn iconfont"
        :class="[ui.isReadonly ? 'iconbianji1' : 'iconyanjing', { active: ui.isReadonly }]"
        :title="ui.isReadonly ? '切换为编辑模式' : '切换为只读浏览'"
        @click="toggleReadonly"
      />
      <button
        type="button"
        class="mm-ctrl-btn iconfont icondaohang1"
        :class="{ active: openMiniMap }"
        :title="openMiniMap ? '关闭小地图' : '开启小地图'"
        @click="toggleMiniMap"
      />
      <Fullscreen />
      <MouseAction />
      <Demonstrate />
      <span class="mm-ctrl-sep" />
      <Scale />
      <!-- 原型底栏后三枚（docs/index.html:429-431）。字形用原型自己的 ⊞ / ⊟ / ⤢：iconfont 里没有
           「收起」字形（只有 iconzhankai 与形状离线不可辨的 iconzhankai1），三个一起用同族符号才不会
           出现「一枚矢量图标 + 两枚文本符号」的混搭；本栏的 −/＋/⋯ 本来就是文本符号（详见报告）。 -->
      <button type="button" class="mm-ctrl-btn" title="自适应大小" @click="onFitCanvas">⤢</button>
      <button type="button" class="mm-ctrl-btn" title="展开全部" @click="onExpandAll">⊞</button>
      <button type="button" class="mm-ctrl-btn" title="收起全部" @click="onUnexpandAll">⊟</button>
      <NDropdown trigger="click" :options="moreOptions" @select="onMoreSelect">
        <button type="button" class="mm-ctrl-btn mm-ctrl-more" title="更多">⋯</button>
      </NDropdown>
    </div>
  </div>
</template>
