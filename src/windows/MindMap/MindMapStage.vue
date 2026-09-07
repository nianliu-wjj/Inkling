<script setup lang="ts">
import { onBeforeUnmount, onMounted, nextTick, ref } from 'vue'
import type MindMap from 'simple-mind-map'
import 'simple-mind-map/dist/simpleMindMap.esm.css'
import { createMindMap } from './core/createMindMap'
import { useConfirm } from './core/confirm'
import { useMindMap } from './core/useMindMap'
import type { MindMapFullData } from './core/persistence'
import { logger } from '@/service/logger'

/**
 * 画布宿主：铺满父容器，等尺寸就绪后创建库实例（simple-mind-map 要求容器宽高非 0），
 * 创建成功后通过 created 事件把实例回传给 MindMapApp。ResizeObserver 驱动 resize。
 */
const props = defineProps<{ data: MindMapFullData }>()
const emit = defineEmits<{ (e: 'created', mindMap: MindMap): void }>()

const { bus, ui, localConfig, mapConfig } = useMindMap()
const confirm = useConfirm()

const host = ref<HTMLElement | null>(null)
let mindMap: MindMap | null = null
let resizeObserver: ResizeObserver | null = null
let sizeWaiter: ResizeObserver | null = null

function mount(): void {
  if (!host.value || mindMap) return
  mindMap = createMindMap({
    el: host.value,
    data: props.data,
    localConfig,
    mapConfig,
    bus,
    confirm: (content) => confirm(content),
    extraTextOnExport: () => ui.extraTextOnExport,
    onExportError: () => bus.emit('toast', '导出失败'),
  })
  resizeObserver = new ResizeObserver(() => mindMap?.resize())
  resizeObserver.observe(host.value)
  emit('created', mindMap)
  logger.info('mindmap', '画布已挂载')
}

/** 等容器拿到非零尺寸后再初始化，避免 0×0 画布。 */
function mountWhenSized(): void {
  const el = host.value
  if (!el) return
  if (el.clientWidth > 0 && el.clientHeight > 0) {
    mount()
    return
  }
  const waiter = new ResizeObserver(() => {
    if (host.value && host.value.clientWidth > 0 && host.value.clientHeight > 0) {
      waiter.disconnect()
      mount()
    }
  })
  waiter.observe(el)
  sizeWaiter = waiter
}

onMounted(() => void nextTick(mountWhenSized))

onBeforeUnmount(() => {
  sizeWaiter?.disconnect()
  resizeObserver?.disconnect()
  mindMap?.destroy()
  mindMap = null
})
</script>

<template>
  <div ref="host" class="mm-canvas" role="application" aria-label="思维导图画布" />
</template>
