<script setup lang="ts">
import { NDropdown } from 'naive-ui'
import { ref } from 'vue'
import { logger } from '@/service/logger'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import MouseAction from './MouseAction.vue'
import Fullscreen from './Fullscreen.vue'
import Scale from './Scale.vue'
import Demonstrate from './Demonstrate.vue'

/**
 * 底部导航控制栏（移植参考端 NavigatorToolbar.vue）。
 *
 * 按裁决去掉：语言切换、暗色开关、AI、下载客户端 / Github / 官网 / 版本、结构下拉
 * （结构切换在右侧「结构」侧栏）。保留：回到根节点 / 搜索 / 鼠标行为 / 小地图开关 /
 * 只读切换 / 全屏 / 缩放 / 演示 / 更多（快捷键、源码编辑）。
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
</script>

<template>
  <div class="mm-nav mm-panel">
    <button type="button" class="mm-nav-btn iconfont icondingwei" title="回到根节点" @click="backToRoot" />
    <button type="button" class="mm-nav-btn iconfont iconsousuo" title="搜索" @click="showSearch" />
    <MouseAction />
    <button
      type="button"
      class="mm-nav-btn iconfont icondaohang1"
      :class="{ active: openMiniMap }"
      :title="openMiniMap ? '关闭小地图' : '开启小地图'"
      @click="toggleMiniMap"
    />
    <button
      type="button"
      class="mm-nav-btn iconfont"
      :class="ui.isReadonly ? 'iconbianji1' : 'iconyanjing'"
      :title="ui.isReadonly ? '切换为编辑模式' : '切换为只读浏览'"
      @click="toggleReadonly"
    />
    <Fullscreen />
    <Scale />
    <Demonstrate />
    <NDropdown trigger="click" :options="moreOptions" @select="onMoreSelect">
      <button type="button" class="mm-nav-btn mm-nav-more" title="更多">⋯</button>
    </NDropdown>
  </div>
</template>
