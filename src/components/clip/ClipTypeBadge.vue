<script setup lang="ts">
import { computed } from 'vue'

/**
 * 剪贴板类型徽章。
 *
 * 需求 2.2：数据类型（文本/链接/代码/图片/富文本）以彩色标签徽章包裹展示。
 * 配色由原型 .clip-type 的修饰类提供，richtext 沿用默认（蓝）配色。
 */
const props = defineProps<{ contentType: string }>()

const LABELS: Record<string, string> = {
  text: '文本',
  link: '链接',
  code: '代码',
  image: '图片',
  richtext: '富文本',
}

const label = computed(() => LABELS[props.contentType] ?? props.contentType)
</script>

<template>
  <span class="clip-type" :class="props.contentType">{{ label }}</span>
</template>
