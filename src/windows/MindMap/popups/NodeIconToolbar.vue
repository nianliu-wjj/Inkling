<script setup lang="ts">
import { nodeIconList } from 'simple-mind-map/src/svg/icons.js'
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { logger } from '@/service/logger'
import { loadStickers, type StickerGroup } from '../constants'
import { useMindMap } from '../core/useMindMap'

/**
 * 节点图标浮动栏（移植参考端 NodeIconToolbar.vue）。
 *
 * 点击节点上已有图标（库事件 `node_icon_click(node, icon)`）时在节点下方弹出该图标同组
 * 的候选列表，可替换 / 取消，或删除当前图标。图标组 = 内置 nodeIconList + 异步贴纸组
 * （与图标侧栏同源）。Teleport 到 body 避免被画布裁剪；缩放 / 点空白 / 双击 / 切换激活节点时关闭。
 */
const { bus } = useMindMap()

const show = ref(false)
const style = reactive<{ left: string; top: string }>({ left: '0', top: '0' })
const node = ref<MindMapNode | null>(null)
const iconType = ref('')
const iconName = ref('')
const currentIcons = ref<string[]>([])
const groupIcons = ref<Array<{ name: string; icon: string }>>([])

/** 全部图标组（内置 + 贴纸），首次挂载异步加载。 */
let allGroups: StickerGroup[] = nodeIconList as StickerGroup[]

function render(icon: string): string {
  return /^<svg/.test(icon) ? icon : `<img src="${icon}" alt="" />`
}

function updatePos(): void {
  if (!node.value) return
  const rect = node.value.getRect()
  style.left = rect.left + 'px'
  style.top = rect.top + rect.height + 'px'
}

function open(target: MindMapNode, icon: string): void {
  node.value = target
  iconType.value = icon.split('_')[0]
  iconName.value = icon.split('_')[1]
  currentIcons.value = (target.getData('icon') as string[]) || []
  const group = allGroups.find((g) => g.type === iconType.value)
  groupIcons.value = group ? [...group.list] : []
  updatePos()
  show.value = true
}

function close(): void {
  show.value = false
  node.value = null
  iconType.value = ''
  iconName.value = ''
  currentIcons.value = []
  groupIcons.value = []
}

/** 设置图标：同 key 取消、同 type 替换、否则追加（参考端 setIcon 逻辑）。 */
function setIcon(name: string): void {
  if (!node.value) return
  const key = `${iconType.value}_${name}`
  const list = [...currentIcons.value]
  const index = list.indexOf(key)
  if (index !== -1) {
    list.splice(index, 1)
  } else {
    const typeIndex = list.findIndex((item) => item.split('_')[0] === iconType.value)
    if (typeIndex !== -1) {
      list.splice(typeIndex, 1, key)
      iconName.value = name
    } else {
      list.push(key)
    }
  }
  currentIcons.value = list
  node.value.setIcon([...list])
}

function deleteIcon(): void {
  setIcon(iconName.value)
  close()
}

function onNodeActive(active: MindMapNode | null): void {
  if (active === node.value) return
  close()
}

const offs: Array<() => void> = []
onMounted(async () => {
  offs.push(
    bus.on('node_icon_click', (target, icon) => open(target as MindMapNode, icon as string)),
    bus.on('draw_click', close),
    bus.on('svg_mousedown', close),
    bus.on('node_dblclick', close),
    bus.on('node_active', (active) => onNodeActive((active as MindMapNode) ?? null)),
    bus.on('scale', updatePos),
    bus.on('closeNodeIconToolbar', close),
  )
  try {
    const stickers = await loadStickers()
    allGroups = [...(nodeIconList as StickerGroup[]), ...stickers]
  } catch (error) {
    logger.error('mindmap', '图标浮动栏加载贴纸组失败，仅用内置图标', error)
  }
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <Teleport to="body">
    <div v-show="show" class="mm-icontb mm-panel" :style="style" @click.stop>
      <div class="mm-icontb-list">
        <button
          v-for="icon in groupIcons"
          :key="icon.name"
          type="button"
          class="mm-icontb-item"
          :class="{ selected: currentIcons.includes(`${iconType}_${icon.name}`) }"
          @click="setIcon(icon.name)"
          v-html="render(icon.icon)"
        />
      </div>
      <div class="mm-icontb-foot">
        <button type="button" class="mm-icontb-del" title="删除该图标" @click="deleteIcon">🗑</button>
      </div>
    </div>
  </Teleport>
</template>
