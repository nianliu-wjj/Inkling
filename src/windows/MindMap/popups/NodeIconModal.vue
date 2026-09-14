<script setup lang="ts">
import { nodeIconList } from 'simple-mind-map/src/svg/icons.js'
import { mergerIconList } from 'simple-mind-map/src/utils/index.js'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { loadStickers, type StickerGroup } from '../constants'
import { useMindMap } from '../core/useMindMap'
import MmModal from './MmModal.vue'

/**
 * 节点图标 / 贴纸弹窗（原型 `docs/app.js:1898-1930` 对通用弹窗的用法，spec D42）。
 *
 * 内容从 `sidebars/IconSidebar.vue` 原样迁入，只换渲染宿主（右侧抽屉 → 工具栏按钮弹出的模态）：
 * 图标组仍 = 库内置 `nodeIconList` + 参考端贴纸组（`icon.js`，异步 chunk）+ `mergerIconList` 合并
 * ——按 spec D47 保留我们的原生图标集，不照抄原型写死的 emoji 列表。
 * 点击语义不变：同 key 取消、同 type 替换、否则追加，对所有激活节点生效；
 * 与原型一致的是「点一下就应用并关窗」（`docs/app.js:1927-1929`）。
 *
 * @author nianliu-jj
 * @since 2026-09-14
 */
const { bus, ui } = useMindMap()
const { toast } = useToast()

const visible = ref(false)
const groups = ref<StickerGroup[]>(nodeIconList as StickerGroup[])
/** 当前节点已有的图标 key（type_name）列表，用于选中态。 */
const selected = ref<string[]>([])
/** 贴纸 chunk 是否在加载中——挡掉重复发起，同时驱动「正在加载贴纸…」提示。 */
const loading = ref(false)
/** 贴纸是否已合并进 `groups`；失败时保持 false，下次打开可重试。 */
const stickersReady = ref(false)

function sync(): void {
  const node = ui.activeNodes[0]
  selected.value = node && ui.activeNodes.length === 1 ? ((node.getData('icon') as string[]) ?? []) : []
}

function toggle(type: string, name: string): void {
  // 无激活节点时不动数据也不关窗：工具栏按钮本就禁用，这里是兜底。
  if (!ui.activeNodes.length) return
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
  logger.info('mindmap', `节点图标：${key}（${ui.activeNodes.length} 个激活节点）`)
  sync()
  // 与原型一致：点一下即应用并关窗（`docs/app.js:1927-1929`）
  visible.value = false
}

/** 图标值可能是内联 SVG 源码，也可能是图片地址（贴纸组）。 */
function render(icon: string): string {
  return /^<svg/.test(icon) ? icon : `<img src="${icon}" alt="" />`
}

/** 打开弹窗：无激活节点时提示并中止（工具栏按钮已禁用，这里是兜底）。 */
function open(): void {
  if (!ui.activeNodes.length) {
    toast('请先选择一个节点')
    return
  }
  sync()
  visible.value = true
  void ensureStickers()
}

/**
 * 按需加载贴纸组。
 *
 * 贴纸是 560KB 的异步 chunk（见 `constants/index.ts`），**不能在组件挂载时就拉**：
 * 本组件跟随窗口常驻挂载（与其它弹层一样靠 bus 事件唤出），挂载即加载等于把它塞进窗口首屏的
 * 加载路径——那是侧栏时期没有的问题（旧侧栏只在抽屉首次切到图标时才挂载）。
 * 因此改为首次打开弹窗时才发起，失败后允许下次打开重试。
 */
async function ensureStickers(): Promise<void> {
  if (stickersReady.value || loading.value) return
  loading.value = true
  try {
    const stickers = await loadStickers()
    groups.value = mergerIconList([...nodeIconList, ...stickers]) as StickerGroup[]
    stickersReady.value = true
  } catch (error) {
    logger.error('mindmap', '加载贴纸组失败，仅显示内置图标', error)
  } finally {
    loading.value = false
  }
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(bus.on('showNodeIcon', open))
  // 选中态随激活节点变化（与侧栏时期一致）
  offs.push(bus.on('node_active', sync))
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <MmModal :visible="visible" title="图标/贴纸" @close="visible = false">
    <p v-if="!ui.activeNodes.length" class="mm-dialog-hint">请先选择一个节点</p>
    <div v-for="group in groups" :key="group.type" class="mm-icon-group">
      <div class="mm-group-title">{{ group.name }}</div>
      <div class="mm-icon-grid">
        <button
          v-for="icon in group.list"
          :key="icon.name"
          type="button"
          class="mm-icon-tile"
          :class="{ active: selected.includes(`${group.type}_${icon.name}`) }"
          :disabled="!ui.activeNodes.length"
          @click="toggle(group.type, icon.name)"
          v-html="render(icon.icon)"
        />
      </div>
    </div>
    <p v-if="loading" class="mm-dialog-hint">正在加载贴纸…</p>
  </MmModal>
</template>
