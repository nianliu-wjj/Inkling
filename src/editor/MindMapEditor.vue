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
/** 等待容器尺寸就绪的观察器（见 mountWhenSized）。 */
let sizeWaiter: ResizeObserver | null = null

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
  if (!host.value || mindMap) return
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

/**
 * 等容器真正拿到非零尺寸后再初始化。
 *
 * simple-mind-map 要求「提供一个宽高不为 0 的容器元素」（官方文档 README_MORE_ZH）。
 * 本组件常在弹窗或 v-if 分支中出现，挂载当帧容器可能仍是 0×0，
 * 此时初始化会得到一张 0 尺寸画布，表现为整片空白且不会自行恢复。
 * 因此先用 ResizeObserver 等到尺寸就绪，再建实例。
 */
function mountWhenSized(): void {
  const el = host.value
  if (!el) return

  if (el.clientWidth > 0 && el.clientHeight > 0) {
    mountMindMap()
    return
  }

  const waiter = new ResizeObserver(() => {
    if (!host.value) return
    if (host.value.clientWidth > 0 && host.value.clientHeight > 0) {
      waiter.disconnect()
      mountMindMap()
    }
  })
  waiter.observe(el)
  // 组件提前卸载时也要断开，避免观察器泄漏。
  sizeWaiter = waiter
}

onMounted(() => {
  void nextTick(mountWhenSized)
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
  sizeWaiter?.disconnect()
  sizeWaiter = null
  resizeObserver?.disconnect()
  resizeObserver = null
  mindMap?.destroy()
  mindMap = null
})
</script>

<template>
  <div class="mindmap-editor" ref="host" role="application" aria-label="思维导图编辑器" />
</template>

<style scoped>
/* simple-mind-map 要求容器宽高不为 0，否则画布尺寸为 0、整片空白。
   这里显式占满可用空间并给出最小高度，避免在 flex 列容器中被压塌。 */
.mindmap-editor {
  position: relative;
  flex: 1;
  width: 100%;
  min-height: 260px;
  overflow: hidden;
  border-radius: 8px;
  background: rgba(var(--wsa), 0.04);
}

/* 官方文档要求的容器内重置。项目全局已有 * { margin:0; padding:0 }，
   此处再声明一次，避免将来全局规则调整后思维导图布局错位。 */
.mindmap-editor :deep(*) {
  margin: 0;
  padding: 0;
}

/* 全局 tokens.css 关闭了 user-select，节点文本编辑需要放开，
   否则双击节点无法选中与输入。 */
.mindmap-editor :deep([contenteditable='true']),
.mindmap-editor :deep(.smm-richtext-node-wrap) {
  user-select: text;
  -webkit-user-select: text;
}
</style>
