<script setup lang="ts">
import MindMap from 'simple-mind-map'
import 'simple-mind-map/dist/simpleMindMap.esm.css'
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: string | null
    placeholder?: string
  }>(),
  { placeholder: '中心主题' },
)

const emit = defineEmits<{
  (event: 'update:modelValue', value: string): void
}>()

const host = ref<HTMLElement | null>(null)
let mindMap: InstanceType<typeof MindMap> | null = null
let syncing = false
let resizeObserver: ResizeObserver | null = null

function createDefaultData(): Record<string, unknown> {
  return {
    data: { text: props.placeholder },
    children: [],
  }
}

function parseData(source: string | null): unknown {
  if (!source?.trim()) return createDefaultData()
  try {
    const parsed: unknown = JSON.parse(source)
    if (parsed && typeof parsed === 'object') return parsed
  } catch {
    // 旧数据或手工损坏的数据不阻断编辑器，回退为一个可编辑根节点。
  }
  return createDefaultData()
}

function publishData(data: unknown = mindMap?.getData(false)): void {
  if (!data) return
  syncing = true
  try {
    emit('update:modelValue', JSON.stringify(data))
  } finally {
    syncing = false
  }
}

function handleDataChange(data: unknown): void {
  publishData(data)
}

function mountMindMap(): void {
  if (!host.value) return
  mindMap = new MindMap({
    el: host.value,
    data: parseData(props.modelValue),
    fit: true,
    enableFreeDrag: true,
    mousewheelAction: 'zoom',
  })
  mindMap.on('data_change', handleDataChange)
  publishData()
  resizeObserver = new ResizeObserver(() => mindMap?.resize())
  resizeObserver.observe(host.value)
}

onMounted(() => {
  void nextTick(mountMindMap)
})

watch(
  () => props.modelValue,
  (next) => {
    if (syncing || !mindMap) return
    const current = JSON.stringify(mindMap.getData(false))
    if (current === next) return
    mindMap.setData(parseData(next))
  },
)

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
  mindMap?.destroy()
  mindMap = null
})
</script>

<template>
  <div class="mindmap-editor" ref="host" role="application" aria-label="思维导图编辑器" />
</template>
