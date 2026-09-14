<script setup lang="ts">
import { computed } from 'vue'
import { useMindMap } from '../core/useMindMap'
import type { SidebarName } from '../core/store'

/**
 * 侧栏容器（移植参考端 Sidebar.vue）。
 *
 * 右侧抽屉，`ui.activeSidebar === name` 时滑出；同一时刻只开一个（由 activeSidebar 单值保证）。
 * 关闭按钮把 activeSidebar 置空。多个侧栏共用本壳，各自放自己的表单内容到默认插槽。
 */
const props = defineProps<{ name: SidebarName; title: string }>()
const { ui } = useMindMap()

const show = computed(() => ui.activeSidebar === props.name)

function close(): void {
  ui.activeSidebar = ''
}
</script>

<template>
  <Transition name="mm-sidebar-slide">
    <aside v-if="show" class="mm-sidebar mm-panel" @click.stop>
      <header class="mm-sidebar-head">
        <span class="mm-sidebar-title">{{ title }}</span>
        <button type="button" class="mm-sidebar-close" title="关闭" @click="close">✕</button>
      </header>
      <div class="mm-sidebar-body">
        <slot />
      </div>
    </aside>
  </Transition>
</template>
