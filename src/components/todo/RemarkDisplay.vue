<script setup lang="ts">
import { computed } from 'vue'
import type { RemarkStyle } from '@/typings/domain'

/**
 * 备注展示。
 *
 * 需求 2.2「备注展示样式」（偏好设置可配）：
 * - icon  ：徽章区显示 📄 图标，悬浮 tooltip 显示全文；
 * - text  ：置灰单行文本，超出省略，悬浮显示全文；
 * - mixed ：默认——≤100 字用文本行，>100 字用图标徽章。
 *
 * 组件同时渲染两种形态中的一种，由 `slot` 位置决定它落在徽章区还是内容下方，
 * 因此调用方需按 `mode` 决定挂载位置（见 TodoCard）。
 */
const props = defineProps<{
  remark: string
  /** 展示形态。命名为 mode 而非 style —— style 是 Vue 的保留属性，会被当成内联样式。 */
  mode: RemarkStyle
}>()

const emit = defineEmits<{ (e: 'edit'): void }>()

/** 混合模式的字数阈值（含）。 */
const MIXED_THRESHOLD = 100

/** 实际生效的展示形态。 */
const resolved = computed<'icon' | 'text' | 'none'>(() => {
  if (!props.remark) return 'none'
  if (props.mode === 'icon') return 'icon'
  if (props.mode === 'text') return 'text'
  return props.remark.length > MIXED_THRESHOLD ? 'icon' : 'text'
})

defineExpose({ resolved })
</script>

<template>
  <span v-if="resolved === 'icon'" class="remark-badge" :title="props.remark" @click.stop="emit('edit')">📄</span>
  <div v-else-if="resolved === 'text'" class="todo-remark" :title="props.remark" @click.stop="emit('edit')">
    {{ props.remark }}
  </div>
</template>
