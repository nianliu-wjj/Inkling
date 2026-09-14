<script setup lang="ts">
import { sidebarTriggerList } from '../constants/lists'
import { useMindMap } from '../core/useMindMap'
import type { SidebarName } from '../core/store'

/**
 * 右侧竖排侧栏触发条（移植参考端 SidebarTrigger.vue）。
 *
 * 点击切换对应侧栏的开合；当前打开的高亮。禅模式下整条隐藏（由父级 v-if 控制）。
 *
 * 阶段五 Task 7：类名从自有层的 `.mm-trigger*` 换成生成层的 `.mm-sidebar-dock` / `.mm-dock-item` /
 * `.mm-dock-ico` / `.mm-dock-lbl`（`components.css:849-890`，照原型 `docs/index.html:373-380`），
 * 六项与顺序本就与原型一致（spec 差异表 #6）。壳上的 `.mm-panel` 一并去掉：生成层那颗 dock 自带
 * 底色 / 描边 / 投影，原型就是这样。自有层只补层序 28 与图标去色复位，理由见 mindmap.css。
 */
const { ui } = useMindMap()

function toggle(name: string): void {
  ui.activeSidebar = ui.activeSidebar === name ? '' : (name as SidebarName)
}
</script>

<template>
  <div class="mm-sidebar-dock">
    <button
      v-for="item in sidebarTriggerList"
      :key="item.value"
      type="button"
      class="mm-dock-item"
      :class="{ active: ui.activeSidebar === item.value }"
      :title="item.name"
      @click="toggle(item.value)"
    >
      <i class="iconfont mm-dock-ico" :class="item.icon" />
      <span class="mm-dock-lbl">{{ item.name }}</span>
    </button>
  </div>
</template>
