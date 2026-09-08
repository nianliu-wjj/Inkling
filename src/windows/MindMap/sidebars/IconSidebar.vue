<script setup lang="ts">
import { nodeIconList } from 'simple-mind-map/src/svg/icons.js'
import { mergerIconList } from 'simple-mind-map/src/utils/index.js'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { logger } from '@/service/logger'
import { loadStickers, type StickerGroup } from '../constants'
import { useMindMap } from '../core/useMindMap'
import SidebarShell from './SidebarShell.vue'

/**
 * 图标与贴纸侧栏（移植参考端 NodeIconSidebar.vue）。
 *
 * 图标组 = 库内置 nodeIconList + 参考端贴纸组（icon.js，异步 chunk，首次打开时加载）合并。
 * 点击：同 key 取消、同 type 替换、否则追加；对所有激活节点生效。
 * 参考端另有 image.js 的 2.1MB SVG 贴纸页签，本项目按 spec D9 不移植。
 */
const { bus, ui } = useMindMap()

const groups = ref<StickerGroup[]>(nodeIconList as StickerGroup[])
/** 当前节点已有的图标 key（type_name）列表，用于选中态。 */
const selected = ref<string[]>([])
const loading = ref(false)

function sync(): void {
  const node = ui.activeNodes[0]
  selected.value = node && ui.activeNodes.length === 1 ? ((node.getData('icon') as string[]) ?? []) : []
}

function toggle(type: string, name: string): void {
  const key = `${type}_${name}`
  ui.activeNodes.forEach((node) => {
    const list: string[] = [...((node.getData('icon') as string[]) ?? [])]
    const index = list.indexOf(key)
    if (index !== -1) {
      list.splice(index, 1)
    } else {
      const typeIndex = list.findIndex((item) => item.split('_')[0] === type)
      if (typeIndex !== -1) list.splice(typeIndex, 1, key)
      else list.push(key)
    }
    node.setIcon(list)
  })
  sync()
}

function render(icon: string): string {
  return /^<svg/.test(icon) ? icon : `<img src="${icon}" alt="" />`
}

const offs: Array<() => void> = []
onMounted(async () => {
  sync()
  offs.push(bus.on('node_active', sync))
  loading.value = true
  try {
    const stickers = await loadStickers()
    groups.value = mergerIconList([...nodeIconList, ...stickers]) as StickerGroup[]
  } catch (error) {
    logger.error('mindmap', '加载贴纸组失败，仅显示内置图标', error)
  } finally {
    loading.value = false
  }
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <SidebarShell name="nodeIconSidebar" title="图标 / 贴纸">
    <p v-if="!ui.activeNodes.length" class="mm-dialog-hint">请先选择一个节点</p>
    <div v-for="group in groups" :key="group.type" class="mm-icon-group">
      <div class="mm-struct-groupname">{{ group.name }}</div>
      <div class="mm-icon-grid">
        <button
          v-for="icon in group.list"
          :key="icon.name"
          type="button"
          class="mm-icon-item"
          :class="{ active: selected.includes(`${group.type}_${icon.name}`) }"
          :disabled="!ui.activeNodes.length"
          @click="toggle(group.type, icon.name)"
          v-html="render(icon.icon)"
        />
      </div>
    </div>
    <p v-if="loading" class="mm-dialog-hint">正在加载贴纸…</p>
  </SidebarShell>
</template>
