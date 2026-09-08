<script setup lang="ts">
import { sidebarTriggerList } from '../constants/lists'
import { useMindMap } from '../core/useMindMap'
import type { SidebarName } from '../core/store'

/**
 * 右侧竖排侧栏触发条（移植参考端 SidebarTrigger.vue）。
 *
 * 点击切换对应侧栏的开合；当前打开的高亮。禅模式下整条隐藏（由父级 v-if 控制）。
 */
const { ui } = useMindMap()

function toggle(name: string): void {
  ui.activeSidebar = ui.activeSidebar === name ? '' : (name as SidebarName)
}
</script>

<template>
  <div class="mm-trigger mm-panel">
    <button
      v-for="item in sidebarTriggerList"
      :key="item.value"
      type="button"
      class="mm-trigger-btn"
      :class="{ active: ui.activeSidebar === item.value }"
      :title="item.name"
      @click="toggle(item.value)"
    >
      <i class="iconfont" :class="item.icon" />
      <span class="mm-trigger-label">{{ item.name }}</span>
    </button>
  </div>
</template>
