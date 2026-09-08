<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { layoutGroupList, layoutImgMap, layoutNameMap } from '../constants/layouts'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import SidebarShell from './SidebarShell.vue'

/**
 * 结构侧栏（移植参考端 Structure.vue）。
 *
 * 分组展示 14 种布局的缩略图；点击 `mindMap.setLayout(value)`。
 * 订阅 `layout_change` 回显当前布局（右键「一键整理」等也会改布局）。
 */
const ctx = useMindMap()
const { bus } = ctx
const current = ref('')

function use(layout: string): void {
  requireMindMap(ctx).setLayout(layout)
  current.value = layout
}

const offs: Array<() => void> = []
onMounted(() => {
  current.value = requireMindMap(ctx).getLayout()
  offs.push(bus.on('layout_change', (layout) => (current.value = layout as string)))
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <SidebarShell name="structure" title="结构">
    <div v-for="group in layoutGroupList" :key="group.name" class="mm-struct-group">
      <div class="mm-struct-groupname">{{ group.name }}</div>
      <div class="mm-struct-grid">
        <button
          v-for="value in group.list"
          :key="value"
          type="button"
          class="mm-struct-item"
          :class="{ active: value === current }"
          :title="layoutNameMap[value] ?? value"
          @click="use(value)"
        >
          <img :src="layoutImgMap[value]" :alt="layoutNameMap[value] ?? value" />
        </button>
      </div>
    </div>
  </SidebarShell>
</template>
